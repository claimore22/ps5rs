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
pub const GOT_VA: u64 = 0x2018;
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

/// Virtual addresses for a multi-import fixture layout.
pub struct Plan {
    /// VA of each message string, in the order given to [`plan`].
    pub messages: Vec<u64>,
    /// VA of each GOT slot, one per import.
    pub got: Vec<u64>,
    pub rela_va: u64,
    pub symtab_va: u64,
    pub strtab_va: u64,
    pub dynamic_va: u64,
    /// One past the last byte of the dynamic table; defines the RW filesz.
    pub end_va: u64,
}

/// Compute the deterministic layout [`build_multi`] will emit, so codegen and
/// the binary agree on every address without drift.
pub fn plan(messages: &[&[u8]], imports: &[String]) -> Plan {
    let mut next = MESSAGE_VA;
    let mut msg_addrs = Vec::with_capacity(messages.len());
    for m in messages {
        msg_addrs.push(next);
        next += m.len() as u64;
    }
    next = (next + 7) & !7;
    let mut got = Vec::with_capacity(imports.len());
    for _ in 0..imports.len() {
        got.push(next);
        next += 8;
    }
    let rela_va = next;
    let symtab_va = rela_va + (imports.len() * 24) as u64;
    let strtab_va = symtab_va + ((imports.len() + 1) * 24) as u64;
    let strtab_end = strtab_va + strtab_size(imports) as u64;
    let dynamic_va = (strtab_end + 7) & !7;
    let end_va = dynamic_va + 112;
    Plan {
        messages: msg_addrs,
        got,
        rela_va,
        symtab_va,
        strtab_va,
        dynamic_va,
        end_va,
    }
}

fn strtab_size(imports: &[String]) -> usize {
    1 + imports.iter().map(|s| s.len() + 1).sum::<usize>()
}

/// Layout spec for a multi-import dynamic fixture.
pub struct MultiDynamicSpec {
    /// Guest entry machine code; displacements are RIP-relative and therefore
    /// load-bias invariant.
    pub code: Vec<u8>,
    /// NUL-terminated messages placed contiguously in the RW segment.
    pub messages: Vec<&'static [u8]>,
    /// Masked symbol names, one per import and per GOT slot, e.g.
    /// `"OaQI1HqFAtk#libSceDbg"`.
    pub imports: Vec<String>,
}

/// Assemble a dynamic `ET_SCE_DYNEXEC` image with one GLOB_DAT relocation per
/// import.  The RW segment grows with the table chain (messages → GOT → RELA →
/// SYMTAB → STRTAB → DYNAMIC) so every table stays inside a mapped region.
pub fn build_multi(spec: &MultiDynamicSpec) -> Vec<u8> {
    let plan = plan(&spec.messages, &spec.imports);
    let n = spec.imports.len();

    let mut strtab = vec![0u8];
    let mut name_offs = Vec::with_capacity(n);
    for name in &spec.imports {
        name_offs.push(strtab.len() as u32);
        strtab.extend_from_slice(name.as_bytes());
        strtab.push(0);
    }

    let data_size = (plan.end_va - 0x2000) as usize;
    let memsz = data_size.div_ceil(0x1000) * 0x1000;
    let code_filesz = (spec.code.len() + 7) & !7;
    assert!(
        code_filesz <= 0x200,
        "fixture code too large for the RX segment"
    );
    let mut buf = vec![0u8; DATA_FILE + memsz];

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
    put_u64(&mut buf, ph0 + 32, code_filesz as u64);
    put_u64(&mut buf, ph0 + 40, code_filesz as u64);
    put_u64(&mut buf, ph0 + 48, 0x1000);

    let ph1 = ph0 + PHDR_SIZE;
    put_u32(&mut buf, ph1, PT_LOAD);
    put_u32(&mut buf, ph1 + 4, PF_R | PF_W);
    put_u64(&mut buf, ph1 + 8, DATA_FILE as u64);
    put_u64(&mut buf, ph1 + 16, 0x2000);
    put_u64(&mut buf, ph1 + 24, 0x2000);
    put_u64(&mut buf, ph1 + 32, data_size as u64);
    put_u64(&mut buf, ph1 + 40, memsz as u64);
    put_u64(&mut buf, ph1 + 48, 0x1000);

    let ph2 = ph1 + PHDR_SIZE;
    put_u32(&mut buf, ph2, PT_DYNAMIC);
    put_u32(&mut buf, ph2 + 4, PF_R | PF_W);
    put_u64(&mut buf, ph2 + 8, file_off(plan.dynamic_va) as u64);
    put_u64(&mut buf, ph2 + 16, plan.dynamic_va);
    put_u64(&mut buf, ph2 + 24, plan.dynamic_va);
    put_u64(&mut buf, ph2 + 32, 112);
    put_u64(&mut buf, ph2 + 40, 112);
    put_u64(&mut buf, ph2 + 48, 8);

    buf[CODE_FILE..CODE_FILE + spec.code.len()].copy_from_slice(&spec.code);

    for (va, message) in plan.messages.iter().zip(spec.messages.iter()) {
        let message_file = file_off(*va);
        buf[message_file..message_file + message.len()].copy_from_slice(message);
    }

    for (i, got_va) in plan.got.iter().enumerate() {
        let rela_file = file_off(plan.rela_va) + i * 24;
        put_u64(&mut buf, rela_file, *got_va);
        put_u64(
            &mut buf,
            rela_file + 8,
            ((i as u64 + 1) << 32) | R_X86_64_GLOB_DAT as u64,
        );
        put_u64(&mut buf, rela_file + 16, 0);
    }

    for (i, name_off) in name_offs.iter().enumerate() {
        let sym_file = file_off(plan.symtab_va) + (i + 1) * 24;
        put_u32(&mut buf, sym_file, *name_off);
        buf[sym_file + 4] = (STB_GLOBAL << 4) | STT_FUNC;
    }

    let strtab_file = file_off(plan.strtab_va);
    buf[strtab_file..strtab_file + strtab.len()].copy_from_slice(&strtab);

    let dyn_file = file_off(plan.dynamic_va);
    for (i, (tag, val)) in [
        (DT_STRTAB, plan.strtab_va),
        (DT_STRSZ, strtab.len() as u64),
        (DT_SYMTAB, plan.symtab_va),
        (DT_SYMENT, 24),
        (DT_RELA, plan.rela_va),
        (DT_RELASZ, (n * 24) as u64),
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

    #[test]
    fn plan_lays_out_chain_after_messages() {
        let messages = [b"alpha\0".as_slice(), b"beta\0".as_slice()];
        let imports = vec![
            "NID0#libA".to_string(),
            "NID1#libB".to_string(),
            "NID2#libC".to_string(),
        ];
        let p = plan(&messages, &imports);
        assert_eq!(p.messages, vec![MESSAGE_VA, MESSAGE_VA + 6]);
        assert_eq!(p.got[0], 0x2010, "GOT aligns right after 12 message bytes");
        assert_eq!(p.got, vec![0x2010, 0x2018, 0x2020]);
        assert_eq!(p.rela_va, p.got[2] + 8);
        assert_eq!(p.symtab_va, p.rela_va + 3 * 24);
        assert_eq!(p.strtab_va, p.symtab_va + 4 * 24, "null symtab entry");
        assert_eq!(
            p.dynamic_va,
            (p.strtab_va + strtab_size(&imports) as u64 + 7) & !7
        );
        assert_eq!(p.end_va, p.dynamic_va + 112);
    }

    #[test]
    fn build_multi_writes_relocations_for_every_import() {
        let imports = vec![
            "NID0#libA".to_string(),
            "NID1#libB".to_string(),
            "NID2#libC".to_string(),
        ];
        let messages: Vec<&'static [u8]> = vec![b"x\0".as_slice()];
        let spec = MultiDynamicSpec {
            code: codegen::ret(),
            messages,
            imports: imports.clone(),
        };
        let bytes = build_multi(&spec);
        let p = plan(&spec.messages, &spec.imports);
        let data_len = (p.end_va - 0x2000).div_ceil(0x1000) * 0x1000;
        assert_eq!(bytes.len(), DATA_FILE + data_len as usize);
        for (i, got_va) in p.got.iter().enumerate() {
            let rela_file = file_off(p.rela_va) + i * 24;
            let r_offset = u64::from_le_bytes(bytes[rela_file..rela_file + 8].try_into().unwrap());
            assert_eq!(r_offset, *got_va);
            let info = u64::from_le_bytes(bytes[rela_file + 8..rela_file + 16].try_into().unwrap());
            assert_eq!(info, ((i as u64 + 1) << 32) | R_X86_64_GLOB_DAT as u64);
        }
        for (i, name) in imports.iter().enumerate() {
            let sym_file = file_off(p.symtab_va) + (i + 1) * 24;
            let str_off =
                u32::from_le_bytes(bytes[sym_file..sym_file + 4].try_into().unwrap()) as usize;
            assert_eq!(
                &bytes
                    [file_off(p.strtab_va) + str_off..file_off(p.strtab_va) + str_off + name.len()],
                name.as_bytes()
            );
        }
        let dyn_file = file_off(p.dynamic_va);
        let relasz = u64::from_le_bytes(
            bytes[dyn_file + 5 * 16 + 8..dyn_file + 6 * 16]
                .try_into()
                .unwrap(),
        );
        assert_eq!(relasz, (imports.len() * 24) as u64);
    }

    #[test]
    fn build_multi_with_one_import_stays_self_consistent() {
        let imports = vec![name()];
        let messages: Vec<&'static [u8]> = vec![b"hi\0".as_slice()];
        let p = plan(&messages, &imports);
        let spec = MultiDynamicSpec {
            code: codegen::puts_and_ret(CODE_VA, p.messages[0], p.got[0]),
            messages,
            imports,
        };
        let bytes = build_multi(&spec);
        assert!(p.got[0] >= p.messages[0] + spec.messages[0].len() as u64);
        assert_eq!(
            &bytes[CODE_FILE..CODE_FILE + 17],
            &codegen::puts_and_ret(CODE_VA, p.messages[0], p.got[0])
        );
        let rela_file = file_off(p.rela_va);
        assert_eq!(
            u64::from_le_bytes(bytes[rela_file..rela_file + 8].try_into().unwrap()),
            p.got[0]
        );
    }
}
