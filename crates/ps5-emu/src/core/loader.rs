//! Loader integration: materialize guest memory, build import slots, and patch
//! the executable modules' GOT slots with HLE stubs.

use crate::error::EmuError;
use crate::hle::Registry;
use crate::imports::ImportTable;
use crate::platform::memory::GuestMemory;
use crate::process::Process;

fn u64_to_nid_str(nid: u64) -> String {
    const B64: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+-";
    let mut val = (nid as u128) << 2;
    let mut out = [0u8; 11];
    for i in (0..11).rev() {
        out[i] = B64[(val & 0x3F) as usize];
        val >>= 6;
    }
    String::from_utf8(out.to_vec()).unwrap()
}

use super::dispatcher::ImportSlot;
use super::relocator::{StubRegion, patch_got_slots};

/// Guest virtual address of the runtime stack.
pub const GUEST_STACK_VA: u64 = 0x0000_7000_0000_0000;
/// Guest stack size in bytes.
pub const GUEST_STACK_SIZE: u64 = 0x0010_0000;

/// Everything needed to run the guest once.
pub struct Prepared {
    pub host: GuestMemory,
    pub slots: Vec<ImportSlot>,
    pub stack_top: u64,
    pub stubs: StubRegion,
}

/// Materialize the process, add a guest stack, and route every import of the
/// given executable modules through an HLE stub.
///
/// Imports of modules that will never execute (loaded PRX images) are left
/// untouched; only the modules listed in `executable_modules` are patched.
pub fn prepare(
    process: &Process,
    imports: &ImportTable,
    executable_modules: &[String],
    registry: &Registry,
) -> Result<Prepared, EmuError> {
    tracing::info!(
        executable_modules = executable_modules.len(),
        imports = imports.bindings.len(),
        "prepare: start"
    );
    let mut host = GuestMemory::materialize(process)?;
    host.add_guest_stack(GUEST_STACK_VA, GUEST_STACK_SIZE)?;
    let stack_top = GUEST_STACK_VA + GUEST_STACK_SIZE;

    let slots = build_slots(imports, executable_modules, registry)?;

    let stubs = StubRegion::new(slots.len())?;
    stubs.write_stubs();
    tracing::debug!(got_slots = slots.len(), "patching GOT slots");
    patch_got_slots(&mut host, &slots, &stubs)?;
    stubs.make_exec()?;
    tracing::debug!(stack_top, "prepare: complete");

    Ok(Prepared {
        host,
        slots,
        stack_top,
        stubs,
    })
}

/// Collect one slot per executable import that the registry can handle.
///
/// An import without a registered handler is a hard error: the guest would
/// otherwise take the loader's stale stub path silently.
fn build_slots(
    imports: &ImportTable,
    executable_modules: &[String],
    registry: &Registry,
) -> Result<Vec<ImportSlot>, EmuError> {
    let mut slots = Vec::new();
    for binding in &imports.bindings {
        if !executable_modules.iter().any(|m| m == &binding.module) {
            continue;
        }
        let name = binding
            .name
            .clone()
            .unwrap_or_else(|| binding.nid_str.clone());
        if !registry.contains(binding.nid) {
            let nid_b64 = u64_to_nid_str(binding.nid);
            return Err(EmuError::NoHandler(format!(
                "{name}#{} [nid {} / {:#x}]",
                binding.library, nid_b64, binding.nid
            )));
        }
        tracing::trace!(nid = format_args!("{:#x}", binding.nid), name, "slot built");
        slots.push(ImportSlot {
            nid: binding.nid,
            name,
            library: binding.library.clone(),
            got_slot: binding.got_slot,
        });
    }
    Ok(slots)
}
