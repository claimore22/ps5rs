//! Dynamic ELF builder for fixtures that exercise the loader's import
//! pipeline: `ET_SCE_DYNEXEC` with a masked dynsym import, one GLOB_DAT
//! relocation, and a `PT_DYNAMIC` table describing the dynamic structures.
//!
//! The guest imports a single symbol through a GOT slot; the loader patches
//! the slot with an HLE stub, so running the fixture records an import call.

use ps5_format::elf_constants::{
    DT_NULL, DT_RELA, DT_RELASZ, DT_STRSZ, DT_STRTAB, DT_SYMENT, DT_SYMTAB, ELF_MAGIC, ELFCLASS64,
    ELFDATA2LSB, EM_X86_64, ET_SCE_DYNEXEC, PF_R, PF_W, PF_X, PT_DYNAMIC, PT_LOAD,
    R_X86_64_GLOB_DAT, STB_GLOBAL, STT_FUNC,
};

use crate::codegen;

pub const CODE_VA: u64 = 0x1000;
const MESSAGE_VA: u64 = 0x2000;
const GOT_VA: u64 = 0x2018;
const RELA_VA: u64 = 0x2030;
const SYMTAB_VA: u64 = 0x2060;
const STRTAB_VA: u64 = 0x20A8;
const DYNAMIC_VA: u64 = 0x2100;

const CODE_FILE: usize = 0x200;
const DATA_FILE: usize = 0x400;
const ELF_HEADER_SIZE: usize = 64;
const PHDR_SIZE: usize = 56;

/// The dynamic tables live in the RW segment; translate a vaddr to its file
/// offset.
fn file_off(va: u64) -> usize {
    DATA_FILE + (va - 0x2000) as usize
}

fn put_u16(buf: &mut [u8], off: usize, val: u16) {
    buf[off..off + 2].copy_from_slice(&val.to_le_bytes());
}

fn put_u32(buf: &mut [u8], off: usize, val: u32) {
    buf[off..off + 4].copy_from_slice(&val.to_le_bytes());
}

fn put_u64(buf: &mut [u8], off: usize, val: u64) {
    buf[off..off + 8].copy_from_slice(&val.to_le_bytes());
}

/// Layout spec for a one-import dynamic fixture.
pub struct DynamicSpec {
    /// Guest entry machine code.
    pub code: Vec<u8>,
    /// NUL-terminated message placed in the RW segment at `MESSAGE_VA`.
    pub message: &'static [u8],
    /// Masked symbol name in the dynstr, e.g. `"01234567890#libkernel"`.
    pub masked_name: String,
}

/// Assemble the full ELF image for [`DynamicSpec`].
pub fn build(spec: &DynamicSpec) -> Vec<u8> {
    let mut strtab = vec![0u8];
    let name_off = strtab.len() as u32;
    strtab.extend_from_slice(spec.masked_name.as_bytes());
    strtab.push(0);
    let mut buf = vec![0u8; 0x600];

    buf[0..4].copy_from_slice(&ELF_MAGIC);
    buf[4] = ELFCLASS64;
    buf[5] = ELFDATA2LSB;
    buf[6] = 1;
    put_u16(&mut buf, 16, ET_SCE_DYNEXEC);
    put_u16(&mut buf, 18, EM_X86_64);
    put_u32(&mut buf, 20, 1);
    put_u64(&mut buf, 24, CODE_VA);
    put_u64(&mut buf, 32, ELF_HEADER_SIZE as u64);
    put_u16(&mut buf, 52, ELF_HEADER_SIZE as u16);
    put_u16(&mut buf, 54, PHDR_SIZE as u16);
    put_u16(&mut buf, 56, 3);

    let ph0 = ELF_HEADER_SIZE;
    put_u32(&mut buf, ph0, PT_LOAD);
    put_u32(&mut buf, ph0 + 4, PF_R | PF_X);
    put_u64(&mut buf, ph0 + 8, CODE_FILE as u64);
    put_u64(&mut buf, ph0 + 16, CODE_VA);
    put_u64(&mut buf, ph0 + 24, CODE_VA);
    put_u64(&mut buf, ph0 + 32, 0x100);
    put_u64(&mut buf, ph0 + 40, 0x100);
    put_u64(&mut buf, ph0 + 48, 0x1000);

    let ph1 = ph0 + PHDR_SIZE;
    put_u32(&mut buf, ph1, PT_LOAD);
    put_u32(&mut buf, ph1 + 4, PF_R | PF_W);
    put_u64(&mut buf, ph1 + 8, DATA_FILE as u64);
    put_u64(&mut buf, ph1 + 16, 0x2000);
    put_u64(&mut buf, ph1 + 24, 0x2000);
    put_u64(&mut buf, ph1 + 32, 0x200);
    put_u64(&mut buf, ph1 + 40, 0x200);
    put_u64(&mut buf, ph1 + 48, 0x1000);

    let ph2 = ph1 + PHDR_SIZE;
    put_u32(&mut buf, ph2, PT_DYNAMIC);
    put_u32(&mut buf, ph2 + 4, PF_R | PF_W);
    put_u64(&mut buf, ph2 + 8, file_off(DYNAMIC_VA) as u64);
    put_u64(&mut buf, ph2 + 16, DYNAMIC_VA);
    put_u64(&mut buf, ph2 + 24, DYNAMIC_VA);
    put_u64(&mut buf, ph2 + 32, 112);
    put_u64(&mut buf, ph2 + 40, 112);
    put_u64(&mut buf, ph2 + 48, 8);

    buf[CODE_FILE..CODE_FILE + spec.code.len()].copy_from_slice(&spec.code);

    let message_file = file_off(MESSAGE_VA);
    buf[message_file..message_file + spec.message.len()].copy_from_slice(spec.message);

    let rela_file = file_off(RELA_VA);
    put_u64(&mut buf, rela_file, GOT_VA);
    put_u64(
        &mut buf,
        rela_file + 8,
        (1u64 << 32) | R_X86_64_GLOB_DAT as u64,
    );
    put_u64(&mut buf, rela_file + 16, 0);

    let symtab_file = file_off(SYMTAB_VA);
    put_u32(&mut buf, symtab_file + 24, name_off);
    buf[symtab_file + 28] = (STB_GLOBAL << 4) | STT_FUNC;

    let strtab_file = file_off(STRTAB_VA);
    buf[strtab_file..strtab_file + strtab.len()].copy_from_slice(&strtab);

    let dyn_file = file_off(DYNAMIC_VA);
    for (i, (tag, val)) in [
        (DT_STRTAB, STRTAB_VA),
        (DT_STRSZ, strtab.len() as u64),
        (DT_SYMTAB, SYMTAB_VA),
        (DT_SYMENT, 24),
        (DT_RELA, RELA_VA),
        (DT_RELASZ, 24),
        (DT_NULL, 0),
    ]
    .iter()
    .enumerate()
    {
        put_u64(&mut buf, dyn_file + i * 16, *tag);
        put_u64(&mut buf, dyn_file + i * 16 + 8, *val);
    }

    buf
}

/// The `hello_puts.elf` guest: import `libkernel::puts`, hand it the message,
/// and return.
pub fn hello_puts(message: &'static [u8], masked_name: String) -> Vec<u8> {
    build(&DynamicSpec {
        code: codegen::puts_and_ret(CODE_VA, MESSAGE_VA, GOT_VA),
        message,
        masked_name,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name() -> String {
        "01234567890#libkernel".to_string()
    }

    #[test]
    fn build_emits_dyn_sections_within_rw_segment() {
        let bytes = hello_puts(b"hi\0", name());
        assert_eq!(bytes.len(), 0x600);
        assert_eq!(&bytes[0..4], &ELF_MAGIC);
        assert_eq!(bytes[4], ELFCLASS64);
        assert_eq!(bytes[5], ELFDATA2LSB);
        assert_eq!(u16::from_le_bytes([bytes[16], bytes[17]]), ET_SCE_DYNEXEC);
        assert_eq!(u16::from_le_bytes([bytes[18], bytes[19]]), EM_X86_64);
        assert_eq!(
            u64::from_le_bytes(bytes[24..32].try_into().unwrap()),
            CODE_VA
        );
        assert_eq!(u16::from_le_bytes([bytes[56], bytes[57]]), 3);
    }

    #[test]
    fn message_and_masked_name_land_in_data() {
        let bytes = hello_puts(b"hi\0", name());
        let message_file = file_off(MESSAGE_VA);
        assert_eq!(&bytes[message_file..message_file + 3], b"hi\0");
        let strtab_file = file_off(STRTAB_VA);
        assert_eq!(bytes[strtab_file], 0);
        assert_eq!(
            &bytes[strtab_file + 1..strtab_file + 22],
            b"01234567890#libkernel"
        );
        assert_eq!(bytes[strtab_file + 22], 0);
    }

    #[test]
    fn code_lands_at_entry() {
        let bytes = hello_puts(b"hi\0", name());
        assert_eq!(
            &bytes[CODE_FILE..CODE_FILE + 17],
            &codegen::puts_and_ret(CODE_VA, MESSAGE_VA, GOT_VA)
        );
    }
}
