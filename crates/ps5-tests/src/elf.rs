//! Byte-exact ELF64 writer for generated fixtures.
//!
//! Produces a plain `ET_EXEC` x86-64 image with two `PT_LOAD` segments
//! (RX code at `0x1000`, RW data after it), using standard ELF constants from
//! `ps5-format` so the writer never re-declares magic numbers.

use ps5_format::elf_constants::{
    EI_ABIVERSION, EI_CLASS, EI_DATA, EI_OSABI, EI_VERSION, ELF_MAGIC, ELFCLASS64, ELFDATA2LSB,
    ELFOSABI_NONE, EM_X86_64, PF_R, PF_W, PF_X, PT_LOAD,
};

use crate::codegen;

pub const ELF_HEADER_SIZE: usize = 64;
pub const PHDR_SIZE: usize = 56;
pub const PAGE_SIZE: u64 = 0x1000;
/// File offset and virtual address of the code segment.
pub const CODE_OFFSET: u64 = PAGE_SIZE;

/// Layout spec for one generated ELF.
pub struct ElfSpec {
    /// `e_type` — always `ET_EXEC` for v0.1 fixtures.
    pub e_type: u16,
    /// Guest entry machine code; padded to a page with `int3`.
    pub code: Vec<u8>,
    /// Writable data contents; padded to a page with zeros.
    pub data: Vec<u8>,
}

/// A fully laid-out ELF with the resulting addresses.
pub struct Elf64 {
    pub bytes: Vec<u8>,
    /// Virtual address of the code segment (= `e_entry`).
    pub code_va: u64,
    /// Virtual address of the data segment.
    pub data_va: u64,
}

/// Assemble the ELF bytes.
///
/// Layout is deterministic: header, program headers, then page-aligned
/// segments, so the output is byte-exact for identical inputs.
pub fn build(spec: &ElfSpec) -> Elf64 {
    let code = codegen::padded_code(&spec.code);
    let data = pad_data(&spec.data);
    let data_offset = CODE_OFFSET + code.len() as u64;
    let total = data_offset as usize + data.len();
    let mut buf = vec![0u8; total];

    buf[0..4].copy_from_slice(&ELF_MAGIC);
    buf[EI_CLASS] = ELFCLASS64;
    buf[EI_DATA] = ELFDATA2LSB;
    buf[EI_VERSION] = 1;
    buf[EI_OSABI] = ELFOSABI_NONE;
    buf[EI_ABIVERSION] = 0;
    put_u16(&mut buf, 16, spec.e_type);
    put_u16(&mut buf, 18, EM_X86_64);
    put_u32(&mut buf, 20, 1);
    put_u64(&mut buf, 24, CODE_OFFSET);
    put_u64(&mut buf, 32, ELF_HEADER_SIZE as u64);
    put_u64(&mut buf, 40, 0);
    put_u32(&mut buf, 48, 0);
    put_u16(&mut buf, 52, ELF_HEADER_SIZE as u16);
    put_u16(&mut buf, 54, PHDR_SIZE as u16);
    put_u16(&mut buf, 56, 2);
    put_u16(&mut buf, 58, 0);
    put_u16(&mut buf, 60, 0);
    put_u16(&mut buf, 62, 0);

    let ph0 = ELF_HEADER_SIZE;
    put_u32(&mut buf, ph0, PT_LOAD);
    put_u32(&mut buf, ph0 + 4, PF_R | PF_X);
    put_u64(&mut buf, ph0 + 8, CODE_OFFSET);
    put_u64(&mut buf, ph0 + 16, CODE_OFFSET);
    put_u64(&mut buf, ph0 + 24, CODE_OFFSET);
    put_u64(&mut buf, ph0 + 32, code.len() as u64);
    put_u64(&mut buf, ph0 + 40, code.len() as u64);
    put_u64(&mut buf, ph0 + 48, PAGE_SIZE);

    let ph1 = ph0 + PHDR_SIZE;
    put_u32(&mut buf, ph1, PT_LOAD);
    put_u32(&mut buf, ph1 + 4, PF_R | PF_W);
    put_u64(&mut buf, ph1 + 8, data_offset);
    put_u64(&mut buf, ph1 + 16, data_offset);
    put_u64(&mut buf, ph1 + 24, data_offset);
    put_u64(&mut buf, ph1 + 32, data.len() as u64);
    put_u64(&mut buf, ph1 + 40, data.len() as u64);
    put_u64(&mut buf, ph1 + 48, PAGE_SIZE);

    buf[CODE_OFFSET as usize..CODE_OFFSET as usize + code.len()].copy_from_slice(&code);
    buf[data_offset as usize..].copy_from_slice(&data);

    Elf64 {
        bytes: buf,
        code_va: CODE_OFFSET,
        data_va: data_offset,
    }
}

fn pad_data(data: &[u8]) -> Vec<u8> {
    const PAGE: usize = PAGE_SIZE as usize;
    let mut out = data.to_vec();
    let rem = PAGE - (out.len() % PAGE);
    if rem < PAGE {
        out.resize(out.len() + rem, 0);
    }
    out
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

#[cfg(test)]
mod tests {
    use super::*;
    use ps5_format::elf_constants::ET_EXEC;

    fn hello_spec() -> ElfSpec {
        ElfSpec {
            e_type: ET_EXEC,
            code: vec![0xc3],
            data: vec![0; 16],
        }
    }

    #[test]
    fn header_is_well_formed() {
        let elf = build(&hello_spec());
        let b = &elf.bytes;
        assert_eq!(&b[0..4], &ELF_MAGIC);
        assert_eq!(b[EI_CLASS], ELFCLASS64);
        assert_eq!(b[EI_DATA], ELFDATA2LSB);
        assert_eq!(read_u16(b, 16), ET_EXEC);
        assert_eq!(read_u16(b, 18), EM_X86_64);
        assert_eq!(read_u64(b, 24), CODE_OFFSET);
        assert_eq!(read_u16(b, 56), 2);
    }

    #[test]
    fn segments_lie_on_page_boundaries() {
        let elf = build(&hello_spec());
        assert_eq!(elf.code_va, 0x1000);
        assert_eq!(elf.data_va, 0x2000);
        assert_eq!(elf.bytes.len(), 0x3000);
    }

    #[test]
    fn program_headers_describe_two_loads() {
        let elf = build(&hello_spec());
        let b = &elf.bytes;
        let ph0 = ELF_HEADER_SIZE;
        assert_eq!(read_u32(b, ph0), PT_LOAD);
        assert_eq!(read_u32(b, ph0 + 4), PF_R | PF_X);
        assert_eq!(read_u64(b, ph0 + 16), 0x1000);
        assert_eq!(read_u32(b, ph0 + 56), PT_LOAD);
        assert_eq!(read_u32(b, ph0 + 60), PF_R | PF_W);
        assert_eq!(read_u64(b, ph0 + 72), 0x2000);
    }

    #[test]
    fn code_lands_at_segment_start() {
        let elf = build(&hello_spec());
        assert_eq!(elf.bytes[0x1000], 0xc3);
        assert_eq!(elf.bytes[0x1001], codegen::CODE_FILL);
    }

    fn read_u16(b: &[u8], off: usize) -> u16 {
        u16::from_le_bytes([b[off], b[off + 1]])
    }

    fn read_u32(b: &[u8], off: usize) -> u32 {
        u32::from_le_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
    }

    fn read_u64(b: &[u8], off: usize) -> u64 {
        u64::from_le_bytes([
            b[off],
            b[off + 1],
            b[off + 2],
            b[off + 3],
            b[off + 4],
            b[off + 5],
            b[off + 6],
            b[off + 7],
        ])
    }
}
