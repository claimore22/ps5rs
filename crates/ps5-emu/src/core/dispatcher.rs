//! Dispatch machinery: the active [`Dispatcher`] and the Rust entry point the
//! machine-code stub calls with a captured [`ImportCallFrame`].

use crate::abi::{ImportCallFrame, escape, escape_ctx};
use crate::error::EmuError;
use crate::hle::{HleContext, Host, Registry};
use crate::platform::memory::GuestMemory;
use crate::trace::ImportCall;

use super::relocator::StubRegion;

/// One HLE import reachable from executable guest code.
pub struct ImportSlot {
    pub nid: u64,
    pub name: String,
    pub library: String,
    pub got_slot: u64,
}

/// Owns every resource live while the guest runs: the HLE [`Registry`], the
/// shared [`HleContext`], the materialized guest memory, and the executable
/// stub region its GOT slots point at.
pub struct Dispatcher {
    registry: Registry,
    ctx: HleContext,
    host: GuestMemory,
    slots: Vec<ImportSlot>,
    hits: Vec<u64>,
    calls: Vec<ImportCall>,
    _stubs: StubRegion,
}

/// Process-global handle to the one active dispatcher; single-threaded.
static mut DISPATCHER: *mut Dispatcher = core::ptr::null_mut();

impl Dispatcher {
    pub fn new(
        registry: Registry,
        ctx: HleContext,
        host: GuestMemory,
        slots: Vec<ImportSlot>,
        stubs: StubRegion,
    ) -> Self {
        let hits = vec![0; slots.len()];
        Self {
            registry,
            ctx,
            host,
            slots,
            hits,
            calls: Vec::new(),
            _stubs: stubs,
        }
    }

    /// Make this dispatcher visible to `ps5emu_dispatch_frame`.
    pub fn install(&mut self) {
        unsafe { DISPATCHER = self as *mut Self }
    }

    pub fn uninstall(&mut self) {
        unsafe { DISPATCHER = core::ptr::null_mut() }
    }

    pub fn hits(&self) -> &[u64] {
        &self.hits
    }

    /// Import calls recorded so far, in call order.
    pub fn calls(&self) -> &[ImportCall] {
        &self.calls
    }

    /// Host memory backing the guest address space (testing / inspection).
    pub fn host(&self) -> &GuestMemory {
        &self.host
    }

    /// Chunks the guest emitted to stdout through the HLE modules.
    pub fn take_output(&mut self) -> Vec<String> {
        self.host.take_output()
    }

    pub fn into_parts(self) -> (Registry, HleContext) {
        (self.registry, self.ctx)
    }
}

/// Rust side of the import boundary, called by `ps5emu_dispatcher`.
///
/// Reads the guest registers and stack arguments out of `frame`, dispatches to
/// the registered HLE handler, and returns the handler result to the stub.  A
/// handler signalling `GuestExit` escapes back to the host caller instead.
///
/// # Safety
///
/// `frame` must be a valid pointer to the [`ImportCallFrame`] the stub built
/// on the guest stack, alive for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "sysv64" fn ps5emu_dispatch_frame(frame: *const ImportCallFrame) -> u64 {
    let f = unsafe { &*frame };
    let raw = unsafe { DISPATCHER };
    if raw.is_null() {
        return 1;
    }
    let disp = unsafe { &mut *raw };
    let index = f.import_index as usize;
    let Some(slot) = disp.slots.get(index) else {
        eprintln!("[ps5-emu] dispatch hit unknown import index {index}");
        return 1;
    };
    disp.hits[index] += 1;
    tracing::trace!(index, import = %slot.name, "dispatch");

    let mut args = [0u64; 24];
    args[..6].copy_from_slice(&[f.rdi, f.rsi, f.rdx, f.rcx, f.r8, f.r9]);
    let mut count = 6;
    if !f.stack_args.is_null() {
        for i in 0..(args.len() - 6) {
            let addr = f.stack_args as u64 + (i as u64) * 8;
            match disp.host.read_bytes(addr, 8) {
                Ok(bytes) => {
                    args[6 + i] = u64::from_le_bytes(bytes.try_into().unwrap_or([0; 8]));
                    count += 1;
                }
                Err(_) => break,
            }
        }
    }

    match disp
        .registry
        .call(&mut disp.ctx, &mut disp.host, slot.nid, &args[..count])
    {
        Ok(value) => {
            disp.calls.push(ImportCall {
                library: slot.library.clone(),
                name: slot.name.clone(),
                args: [args[0], args[1], args[2], args[3], args[4], args[5]],
                return_value: value,
            });
            value
        }
        Err(EmuError::GuestExit(code)) => {
            let ctx = escape_ctx();
            unsafe { escape(ctx, code) };
        }
        Err(err) => {
            eprintln!("[ps5-emu] import {} failed: {err}", slot.name);
            let ctx = escape_ctx();
            unsafe { escape(ctx, 1) };
        }
    }
}
