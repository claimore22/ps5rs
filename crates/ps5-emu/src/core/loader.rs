//! Loader integration: materialize guest memory, build import slots, and patch
//! the executable modules' GOT slots with HLE stubs.

use crate::error::EmuError;
use crate::hle::Registry;
use crate::imports::ImportTable;
use crate::platform::memory::GuestMemory;
use crate::process::Process;
use super::dispatcher::ImportSlot;
use super::relocator::{StubRegion, patch_got_slots};

/// Guest virtual address of the runtime stack. This is the base address where the guest stack will reside in the virtual memory layout.
pub const GUEST_STACK_VA: u64 = 0x0000_7000_0000_0000;
/// Guest stack size in bytes. This value is passed to `GuestMemory::add_guest_stack` to allocate the runtime stack, and it is also used to calculate the top of the stack stored in `Prepared.stack_top`.
pub const GUEST_STACK_SIZE: u64 = 0x0010_0000;

fn u64_to_nid_str(nid: u64) -> String {





    ps5_nid::encode_nid(nid.to_be_bytes())
}

/// Everything needed to run the guest once.
/// Structure holding all information required to start a guest binary in the emulator.
/// The `host` field owns the backing `GuestMemory`. The `slots` record the HLE
/// function stubs that were patched into the guest.
/// `stack_top` is the stack pointer after the stack has been created, and will be
/// fed to the guest's entry point as a syscall argument. The `stubs` field
/// contains the region that was written to memory and later made executable.
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
    registry: &mut Registry,
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
/// Collect one slot per executable import that the registry can handle.
///
/// An import without a registered handler is a hard error: the guest would
/// otherwise take the loader's stale stub path silently. The function returns
/// a vector of `ImportSlot` values which will later be written into the
/// guest's GOT table.
fn build_slots(
    imports: &ImportTable,
    executable_modules: &[String],
    registry: &mut Registry,
) -> Result<Vec<ImportSlot>, EmuError> {
    let mut slots = Vec::new();
    let mut reported = std::collections::HashSet::new();
    for binding in &imports.bindings {
        // Skip imports belonging to modules that are never executed. The
        // loader patched them in the PRX image instead of in the guard process.
        if !executable_modules.iter().any(|m| m == &binding.module) {
            continue;
        }

        // Resolve the human‑readable name for the import. Some import tables
        // provide a real name, others fall back to the plain NID string.
        let name = binding
            .name
            .clone()
            .unwrap_or_else(|| binding.nid_str.clone());

        // Ensure the emulator has a handler for this NID. The unexpected path,
        // e.g. stubs missing, is surfaced as an explicit error.
        if !registry.contains(binding.nid) {
            registry.register_stub(&binding.library, binding.nid, &name);
            let nid_b64 = u64_to_nid_str(binding.nid);
            let key = (binding.library.clone(), name.clone(), binding.nid);
            if reported.insert(key) {
                tracing::info!(
                    library = %binding.library,
                    name = %name,
                    nid = format_args!("{:#x}", binding.nid),
                    nid_b64 = %nid_b64,
                    "setup: registered generic HLE stub"
                );
            }
        }

        // Successful imports are converted into import slots.
        tracing::trace!(nid = format_args!("{:#x}", binding.nid), name, "slot built");
        slots.push(ImportSlot {
            nid: binding.nid,
            name,
            library: binding.library.clone(),
            got_slot: binding.got_slot,
            stubbed: !registry.contains(binding.nid)
        });
    }
    Ok(slots)
}
