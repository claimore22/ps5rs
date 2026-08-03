//! End-to-end guest execution: a synthetic ELF whose entry calls `puts` and
//! `exit` through GOT slots patched with HLE stubs.  Exercises the full
//! materialize → stub → dispatcher → Registry → escape pipeline on real
//! x86-64 machine code.

use std::sync::Mutex;

use ps5_emu::{Emulator, Process};

static GUEST_LOCK: Mutex<()> = Mutex::new(());

/// This binary runs as its own process, so it must not share the default
/// load base with the other test binaries (their identity-mapped
/// reservations would collide).
const LOAD_BASE: u64 = 0x840000000;

#[cfg(target_os = "windows")]
unsafe extern "system" {
    fn VirtualAlloc(addr: *mut u8, size: usize, ty: u32, protect: u32) -> *mut u8;
    fn VirtualFree(addr: *mut u8, size: usize, ty: u32) -> i32;
    fn VirtualProtect(addr: *mut u8, size: usize, new: u32, old: *mut u32) -> i32;
}

const ELFCLASS64: u8 = 2;
const ELFDATA2LSB: u8 = 1;
const EM_X86_64: u16 = 62;
const ET_SCE_DYNEXEC: u16 = 0xFE10;
const PT_LOAD: u32 = 1;
const PT_DYNAMIC: u32 = 2;
const PF_R: u32 = 4;
const PF_W: u32 = 2;
const PF_X: u32 = 1;

const CODE_VA: u64 = 0x1000;
const GOT_PUTS: u64 = 0x2000;
const GOT_PRINTF: u64 = 0x2000;
const GOT_EXIT: u64 = 0x2008;
const STRING_VA: u64 = 0x2010;
const RELA_VA: u64 = 0x2030;
const SYMTAB_VA: u64 = 0x2060;
const STRTAB_VA: u64 = 0x20A8;
const DYNAMIC_VA: u64 = 0x2400;

fn put_u16(buf: &mut [u8], off: usize, val: u16) {
    buf[off..off + 2].copy_from_slice(&val.to_le_bytes());
}

fn put_u32(buf: &mut [u8], off: usize, val: u32) {
    buf[off..off + 4].copy_from_slice(&val.to_le_bytes());
}

fn put_u64(buf: &mut [u8], off: usize, val: u64) {
    buf[off..off + 8].copy_from_slice(&val.to_le_bytes());
}

/// x86-64 machine code for the guest entry:
///   lea  rdi, [rip + 0x1009]   ; &"hello from guest"
///   mov  rax, [rip + 0x0FF2]   ; GOT[puts] → HLE stub
///   call rax
///   xor  edi, edi
///   mov  rax, [rip + 0x0FEF]   ; GOT[exit] → HLE stub
///   call rax
///   hlt
const CODE: [u8; 28] = [
    0x48, 0x8D, 0x3D, 0x09, 0x10, 0x00, 0x00, //
    0x48, 0x8B, 0x05, 0xF2, 0x0F, 0x00, 0x00, //
    0xFF, 0xD0, //
    0x31, 0xFF, //
    0x48, 0x8B, 0x05, 0xEF, 0x0F, 0x00, 0x00, //
    0xFF, 0xD0, //
    0xF4, //
];

/// x86-64 machine code for a guest entry that calls `printf` with a variadic
/// mix and then `exit`:
///   lea  rdi, [rip + 0x1009]   ; &"n=%d s=%s x=%x\n"
///   mov  esi, 42
///   lea  rdx, [rip + 0x100D]   ; &"world"
///   mov  ecx, 26
///   mov  rax, [rip + 0x0FE1]   ; GOT[printf] → HLE stub
///   call rax
///   xor  edi, edi
///   mov  rax, [rip + 0x0FDE]   ; GOT[exit] → HLE stub
///   call rax
///   hlt
const PRINTF_CODE: [u8; 45] = [
    0x48, 0x8D, 0x3D, 0x09, 0x10, 0x00, 0x00, //
    0xBE, 0x2A, 0x00, 0x00, 0x00, //
    0x48, 0x8D, 0x15, 0x0D, 0x10, 0x00, 0x00, //
    0xB9, 0x1A, 0x00, 0x00, 0x00, //
    0x48, 0x8B, 0x05, 0xE1, 0x0F, 0x00, 0x00, //
    0xFF, 0xD0, //
    0x31, 0xFF, //
    0x48, 0x8B, 0x05, 0xDE, 0x0F, 0x00, 0x00, //
    0xFF, 0xD0, //
    0xF4, //
];

/// Build a minimal PS5 ELF with two LOAD segments (R-X code, RW data), a
/// dynsym/dynstr/rela set, and a dynamic section describing two `libc`
/// imports whose GOT slots are `got_slots[0]` and `got_slots[1]`.
fn guest_elf_with(
    code: &[u8],
    nid_names: &[String],
    got_slots: &[u64],
    strings: &[(u64, &[u8])],
) -> Vec<u8> {
    assert_eq!(nid_names.len(), 2);
    assert_eq!(got_slots.len(), 2);

    let mut strtab = vec![0u8];
    let mut name_offs = Vec::new();
    for name in nid_names {
        name_offs.push(strtab.len() as u32);
        strtab.extend_from_slice(name.as_bytes());
        strtab.push(0);
    }

    let code_off = 0x200usize;
    let data_off = 0x400usize;
    let rela_file = data_off + (RELA_VA - 0x2000) as usize;
    let symtab_file = data_off + (SYMTAB_VA - 0x2000) as usize;
    let strtab_file = data_off + (STRTAB_VA - 0x2000) as usize;
    let dyn_file = data_off + (DYNAMIC_VA - 0x2000) as usize;

    let mut buf = vec![0u8; 0x900];

    buf[0..4].copy_from_slice(&[0x7f, b'E', b'L', b'F']);
    buf[4] = ELFCLASS64;
    buf[5] = ELFDATA2LSB;
    buf[6] = 1;
    put_u16(&mut buf, 16, ET_SCE_DYNEXEC);
    put_u16(&mut buf, 18, EM_X86_64);
    put_u32(&mut buf, 20, 1);
    put_u64(&mut buf, 24, CODE_VA);
    put_u64(&mut buf, 32, 64);
    put_u64(&mut buf, 40, 0);
    put_u16(&mut buf, 52, 64);
    put_u16(&mut buf, 54, 56);
    put_u16(&mut buf, 56, 3);

    let ph0 = 64usize;
    put_u32(&mut buf, ph0, PT_LOAD);
    put_u32(&mut buf, ph0 + 4, PF_R | PF_X);
    put_u64(&mut buf, ph0 + 8, code_off as u64);
    put_u64(&mut buf, ph0 + 16, CODE_VA);
    put_u64(&mut buf, ph0 + 24, CODE_VA);
    put_u64(&mut buf, ph0 + 32, 0x100);
    put_u64(&mut buf, ph0 + 40, 0x100);
    put_u64(&mut buf, ph0 + 48, 0x1000);

    let ph1 = ph0 + 56;
    put_u32(&mut buf, ph1, PT_LOAD);
    put_u32(&mut buf, ph1 + 4, PF_R | PF_W);
    put_u64(&mut buf, ph1 + 8, data_off as u64);
    put_u64(&mut buf, ph1 + 16, 0x2000);
    put_u64(&mut buf, ph1 + 24, 0x2000);
    put_u64(&mut buf, ph1 + 32, 0x500);
    put_u64(&mut buf, ph1 + 40, 0x500);
    put_u64(&mut buf, ph1 + 48, 0x1000);

    let ph2 = ph1 + 56;
    put_u32(&mut buf, ph2, PT_DYNAMIC);
    put_u32(&mut buf, ph2 + 4, PF_R | PF_W);
    put_u64(&mut buf, ph2 + 8, dyn_file as u64);
    put_u64(&mut buf, ph2 + 16, DYNAMIC_VA);
    put_u64(&mut buf, ph2 + 24, DYNAMIC_VA);
    put_u64(&mut buf, ph2 + 32, 112);
    put_u64(&mut buf, ph2 + 40, 112);
    put_u64(&mut buf, ph2 + 48, 8);

    buf[code_off..code_off + code.len()].copy_from_slice(code);

    for (va, bytes) in strings {
        let str_off = data_off + (va - 0x2000) as usize;
        buf[str_off..str_off + bytes.len()].copy_from_slice(bytes);
    }

    put_u64(&mut buf, rela_file, got_slots[0]);
    put_u64(&mut buf, rela_file + 8, (1u64 << 32) | 6);
    put_u64(&mut buf, rela_file + 16, 0);
    put_u64(&mut buf, rela_file + 24, got_slots[1]);
    put_u64(&mut buf, rela_file + 32, (2u64 << 32) | 6);
    put_u64(&mut buf, rela_file + 40, 0);

    for (i, &name_off) in name_offs.iter().enumerate() {
        let sym = symtab_file + 24 + i * 24;
        put_u32(&mut buf, sym, name_off);
        buf[sym + 4] = 0x12;
        buf[sym + 5] = 0;
        put_u16(&mut buf, sym + 6, 0);
        put_u64(&mut buf, sym + 8, 0);
        put_u64(&mut buf, sym + 16, 0);
    }

    buf[strtab_file..strtab_file + strtab.len()].copy_from_slice(&strtab);

    for (i, (tag, val)) in [
        (5u64, STRTAB_VA),
        (10u64, strtab.len() as u64),
        (6u64, SYMTAB_VA),
        (11u64, 24u64),
        (7u64, RELA_VA),
        (8u64, 48u64),
        (0u64, 0u64),
    ]
    .iter()
    .enumerate()
    {
        put_u64(&mut buf, dyn_file + i * 16, *tag);
        put_u64(&mut buf, dyn_file + i * 16 + 8, *val);
    }

    buf
}

fn guest_elf() -> Vec<u8> {
    let puts_nid = ps5_nid::algorithm::hash("puts");
    let exit_nid = ps5_nid::algorithm::hash("exit");
    guest_elf_with(
        &CODE,
        &[format!("{puts_nid}#libc"), format!("{exit_nid}#libc")],
        &[GOT_PUTS, GOT_EXIT],
        &[(STRING_VA, b"hello from guest\0")],
    )
}

fn printf_guest() -> Vec<u8> {
    let printf_nid = ps5_nid::algorithm::hash("printf");
    let exit_nid = ps5_nid::algorithm::hash("exit");
    guest_elf_with(
        &PRINTF_CODE,
        &[format!("{printf_nid}#libc"), format!("{exit_nid}#libc")],
        &[GOT_PRINTF, GOT_EXIT],
        &[(0x2010, b"n=%d s=%s x=%x\n\0"), (0x2020, b"world\0")],
    )
}

#[test]
fn guest_calls_puts_then_exits_with_code_zero() {
    let _guard = GUEST_LOCK.lock().unwrap();
    let process = Process::load_at("eboot.elf", guest_elf(), |_| None, None, LOAD_BASE).unwrap();
    let mut emulator = Emulator::new(process);
    emulator.resolve_imports().unwrap();
    let code = emulator.run().unwrap().exit_code;
    assert_eq!(code, 0);
    assert!(matches!(emulator.state(), ps5_emu::EmuState::Halted));
}

#[test]
fn entry_that_just_returns_gives_code_zero() {
    let _guard = GUEST_LOCK.lock().unwrap();
    let mut elf = guest_elf();
    elf[0x200..0x200 + CODE.len()].fill(0xCC);
    elf[0x200] = 0xC3;
    let process = Process::load_at("eboot.elf", elf, |_| None, None, LOAD_BASE).unwrap();
    let mut emulator = Emulator::new(process);
    emulator.resolve_imports().unwrap();
    let code = emulator.run().unwrap().exit_code;
    assert_eq!(code, 0);
}

#[test]
fn got_read_only_then_return_gives_code_zero() {
    let _guard = GUEST_LOCK.lock().unwrap();
    let mut elf = guest_elf();
    const GOT_READ_ONLY: [u8; 8] = [
        0x48, 0x8B, 0x05, 0xF9, 0x0F, 0x00, 0x00, // mov rax, [rip+0xFF9]
        0xC3, // ret
    ];
    elf[0x200..0x200 + GOT_READ_ONLY.len()].copy_from_slice(&GOT_READ_ONLY);
    elf[0x200 + GOT_READ_ONLY.len()..0x200 + CODE.len()].fill(0xCC);
    let process = Process::load_at("eboot.elf", elf, |_| None, None, LOAD_BASE).unwrap();
    let mut emulator = Emulator::new(process);
    emulator.resolve_imports().unwrap();
    let code = emulator.run().unwrap().exit_code;
    assert_eq!(code, 0);
}

#[test]
#[cfg(target_os = "windows")]
fn host_indirect_call_to_rx_page() {
    unsafe {
        let p = VirtualAlloc(std::ptr::null_mut(), 4096, 0x3000, 0x04);
        assert!(!p.is_null());
        p.write(0xC3);
        let mut old = 0u32;
        VirtualProtect(p, 4096, 0x20, &mut old);
        let f: extern "C" fn() = std::mem::transmute::<usize, extern "C" fn()>(p as usize);
        f();
        VirtualFree(p, 0, 0x8000);
    }
}

#[test]
fn stub_call_then_return_gives_code_zero() {
    let _guard = GUEST_LOCK.lock().unwrap();
    let mut elf = guest_elf();
    const STUB_ONLY: [u8; 19] = [
        0x48, 0x8D, 0x3D, 0x09, 0x10, 0x00, 0x00, // lea rdi, [rip+0x1009] (string)
        0x48, 0x8B, 0x05, 0xF2, 0x0F, 0x00, 0x00, // mov rax, [rip+0xFF2]
        0xFF, 0xD0, // call rax (puts stub)
        0x31, 0xC0, // xor eax,eax
        0xC3, // ret
    ];
    elf[0x200..0x200 + STUB_ONLY.len()].copy_from_slice(&STUB_ONLY);
    elf[0x200 + STUB_ONLY.len()..0x200 + CODE.len()].fill(0xCC);
    let process = Process::load_at("eboot.elf", elf, |_| None, None, LOAD_BASE).unwrap();
    let mut emulator = Emulator::new(process);
    emulator.resolve_imports().unwrap();
    let code = emulator.run().unwrap().exit_code;
    assert_eq!(code, 0);
}

#[test]
fn guest_imports_patched_with_stubs() {
    let process = Process::load_at("eboot.elf", guest_elf(), |_| None, None, LOAD_BASE).unwrap();
    let load_bias = process.eboot().unwrap().load_bias;
    let mut emulator = Emulator::new(process);
    let table = emulator.resolve_imports().unwrap();
    assert_eq!(table.bindings.len(), 2);
    assert_eq!(table.unknown, 2);

    for binding in &table.bindings {
        assert_eq!(binding.module, "eboot.elf");
        assert_eq!(binding.library, "libc");
    }
    let puts = table
        .bindings
        .iter()
        .find(|b| b.nid_str == ps5_nid::algorithm::hash("puts"))
        .expect("puts import");
    assert_eq!(puts.got_slot, load_bias + GOT_PUTS);
}

#[test]
fn guest_printf_formats_variadic_args_and_exits_zero() {
    let _guard = GUEST_LOCK.lock().unwrap();
    let process = Process::load_at("eboot.elf", printf_guest(), |_| None, None, LOAD_BASE).unwrap();
    let mut emulator = Emulator::new(process);
    let catalog = ps5_emu::nid::catalog();
    emulator.resolve_imports_with(&catalog).unwrap();

    let report = emulator.run().unwrap();
    assert_eq!(report.exit_code, 0);
    assert_eq!(report.import_calls.len(), 1);
    let call = &report.import_calls[0];
    assert_eq!(call.library, "libc");
    assert_eq!(call.name, "printf");
    assert_eq!(call.args[1], 42);
    assert_eq!(call.args[3], 26);
    assert_eq!(call.return_value, 18);
}
