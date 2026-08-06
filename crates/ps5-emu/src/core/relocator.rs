//! HLE import stubs: machine-code trampolines written into an executable block
//! and pointed to from the guest's GOT slots.

use crate::abi::dispatcher_address;
use crate::error::EmuError;
use crate::hle::Host;
use crate::platform::memory::{ExecBlock, GuestMemory};

use super::dispatcher::ImportSlot;

/// Bytes per stub slot; a stub is 24 bytes, padded for uniform indexing.
const STUB_SLOT_SIZE: usize = 32;

/// One executable allocation holding every import stub.
pub struct StubRegion {
    block: ExecBlock,
    count: usize,
}

impl StubRegion {
    pub fn new(count: usize) -> Result<Self, EmuError> {
        let block = ExecBlock::alloc_rw(count.saturating_mul(STUB_SLOT_SIZE).max(1))?;
        Ok(Self { block, count })
    }

    pub fn stub_address(&self, index: usize) -> u64 {
        self.block.address() + (index * STUB_SLOT_SIZE) as u64
    }

    /// Write the 24-byte stub for every slot: `push imm32 index` /
    /// `mov r11, dispatcher` / `call r11` / `add rsp, 8` / `ret`.
    pub fn write_stubs(&self) {
        let dispatcher = dispatcher_address();
        let mut bytes = [0x90u8; STUB_SLOT_SIZE];
        for index in 0..self.count {
            bytes[0] = 0x68;
            bytes[1..5].copy_from_slice(&(index as u32).to_le_bytes());
            bytes[5..7].copy_from_slice(&[0x49, 0xBB]);
            bytes[7..15].copy_from_slice(&dispatcher.to_le_bytes());
            bytes[15..18].copy_from_slice(&[0x41, 0xFF, 0xD3]);
            bytes[18..22].copy_from_slice(&[0x48, 0x83, 0xC4, 0x08]);
            bytes[22] = 0xC3;
            self.block.write(index * STUB_SLOT_SIZE, &bytes);
        }
    }

    pub fn make_exec(&self) -> Result<(), EmuError> {
        self.block.make_exec()
    }
}

/// Overwrite each GOT slot with its HLE stub address.
pub fn patch_got_slots(
    host: &mut GuestMemory,
    slots: &[ImportSlot],
    stubs: &StubRegion,
) -> Result<(), EmuError> {
    for (index, slot) in slots.iter().enumerate() {
        let stub = stubs.stub_address(index);
        host.write(slot.got_slot, &stub.to_le_bytes())?;
    }
    Ok(())
}
