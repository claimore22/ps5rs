//! x86-64 machine-code helpers for generated guest entries.

/// `ret` — the entire body of a minimal guest entry that returns immediately.
pub fn ret() -> Vec<u8> {
    vec![0xc3]
}

/// Guest entry that loads a string pointer and an imported function slot, calls
/// it with the string in `rdi`, and returns:
///
/// ```asm
/// lea  rdi, [rip + string]
/// mov  rax, [rip + got_puts]
/// call rax
/// ret
/// ```
///
/// Displacements are computed from the virtual addresses, so the caller picks
/// the layout.  Exercises RIP-relative addressing, GOT relocation, an indirect
/// call, the SysV first-argument register, and natural return.
pub fn puts_and_ret(code_va: u64, string_va: u64, puts_got: u64) -> Vec<u8> {
    let mut out = Vec::with_capacity(20);
    out.extend_from_slice(&[0x48, 0x8D, 0x3D]);
    out.extend_from_slice(&(string_va.wrapping_sub(code_va + 7) as u32).to_le_bytes());
    out.extend_from_slice(&[0x48, 0x8B, 0x05]);
    let after_lea = code_va + 7;
    out.extend_from_slice(&(puts_got.wrapping_sub(after_lea + 7) as u32).to_le_bytes());
    out.extend_from_slice(&[0xFF, 0xD0, 0xC3]);
    out
}

/// SysV x86-64 register encodings used by the [`Asm`] emitter.
pub mod reg {
    pub const RAX: u8 = 0;
    pub const RCX: u8 = 1;
    pub const RDX: u8 = 2;
    pub const RBX: u8 = 3;
    pub const RSI: u8 = 6;
    pub const RDI: u8 = 7;
    pub const R8: u8 = 8;
    pub const R9: u8 = 9;
    pub const R12: u8 = 12;
    pub const R13: u8 = 13;
    pub const R14: u8 = 14;
    pub const R15: u8 = 15;
}

/// Positional x86-64 emitter.
///
/// Every instruction knows its own virtual address, so RIP-relative
/// displacements are computed as bytes are appended and the caller never
/// tracks offsets by hand.
pub struct Asm {
    bytes: Vec<u8>,
    start_va: u64,
}

impl Asm {
    pub fn new(start_va: u64) -> Self {
        Asm {
            bytes: Vec::new(),
            start_va,
        }
    }

    /// Virtual address the next appended instruction will occupy.
    fn here(&self) -> u64 {
        self.start_va + self.bytes.len() as u64
    }

    /// RIP-relative displacement from `start` (the instruction's own address)
    /// to `target` for an instruction of `len` bytes.
    fn disp32(start: u64, target: u64, len: u64) -> u32 {
        target.wrapping_sub(start + len) as u32
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    pub fn ret(&mut self) {
        self.bytes.push(0xC3);
    }

    pub fn xor_eax_eax(&mut self) {
        self.bytes.extend_from_slice(&[0x31, 0xC0]);
    }

    pub fn sub_rsp(&mut self, imm: u8) {
        self.bytes.extend_from_slice(&[0x48, 0x83, 0xEC, imm]);
    }

    pub fn add_rsp(&mut self, imm: u8) {
        self.bytes.extend_from_slice(&[0x48, 0x83, 0xC4, imm]);
    }

    /// `mov r32, imm32` (zero-extends into the full register).
    pub fn mov_r32_imm(&mut self, dst: u8, imm: u32) {
        if dst >= 8 {
            self.bytes.push(0x41);
        }
        self.bytes.push(0xB8 + (dst & 7));
        self.bytes.extend_from_slice(&imm.to_le_bytes());
    }

    /// `xor r32, r32` — zeroes a register and its upper 32 bits.
    pub fn xor_r32(&mut self, reg: u8) {
        if reg >= 8 {
            self.bytes.push(0x45);
        }
        let code = reg & 7;
        self.bytes.push(0x31);
        self.bytes.push(0xC0 | (code << 3) | code);
    }

    /// Zero every SysV integer argument register so the recorded call args
    /// are deterministic rather than leftover-garbage.
    pub fn zero_args(&mut self) {
        for reg in [reg::RDI, reg::RSI, reg::RDX, reg::RCX, reg::R8, reg::R9] {
            self.xor_r32(reg);
        }
    }

    /// `lea r64, [rip + target]`.
    pub fn lea_rip(&mut self, dst: u8, target: u64) {
        let start = self.here();
        self.bytes.push(if dst >= 8 { 0x4C } else { 0x48 });
        self.bytes.push(0x8D);
        self.bytes.push(0x05 | ((dst & 7) << 3));
        self.bytes
            .extend_from_slice(&Self::disp32(start, target, 7).to_le_bytes());
    }

    /// `call qword ptr [rip + target]` — an indirect call through a GOT slot.
    pub fn call_got(&mut self, got_va: u64) {
        let start = self.here();
        self.bytes.extend_from_slice(&[0xFF, 0x15]);
        self.bytes
            .extend_from_slice(&Self::disp32(start, got_va, 6).to_le_bytes());
    }

    /// `mov r64, r64`.
    pub fn mov_r64(&mut self, dst: u8, src: u8) {
        self.bytes.push(if dst >= 8 { 0x49 } else { 0x48 });
        self.bytes.push(0x89);
        self.bytes.push(0xC0 | ((src & 7) << 3) | (dst & 7));
    }

    /// `push r64`.
    pub fn push_r(&mut self, reg: u8) {
        if reg >= 8 {
            self.bytes.extend_from_slice(&[0x41, 0x50 + (reg & 7)]);
        } else {
            self.bytes.push(0x50 + reg);
        }
    }
}

/// Guest addresses for [`libdbg_basic_code`]: GOT slots and string messages.
pub struct LibdbgBasicAddrs {
    pub min_level: u64,
    pub puts: u64,
    pub rand: u64,
    pub handler: u64,
    pub msg_hello: u64,
    pub file: u64,
    pub component: u64,
    pub fmt_trace: u64,
    pub fmt_debug: u64,
    pub fmt_warning: u64,
    pub fmt_error: u64,
    pub mind: u64,
    pub daisy: u64,
}

/// Guest entry for the `libdbg_basic` fixture: set the minimum log level to
/// DEBUG, print a banner, then replay the same trace/debug/warning/error
/// sequence the SDK sample compiles — plus a compile-time-enabled trace call
/// to exercise the HLE severity filter.
///
/// `rand` values are parked in callee-saved registers so they survive the
/// stub calls, and stack varargs are pushed right-to-left so the first stack
/// argument lands at the lowest address, exactly as a SysV caller would.
pub fn libdbg_basic_code(a: &LibdbgBasicAddrs) -> Vec<u8> {
    use reg::*;
    let mut asm = Asm::new(0x1000);

    asm.sub_rsp(8);

    // sceDbgSetMinimumLogLevel(SCE_DBG_LOG_LEVEL_DEBUG)
    asm.zero_args();
    asm.mov_r32_imm(RDI, 1);
    asm.call_got(a.min_level);

    // puts(banner)
    asm.zero_args();
    asm.lea_rip(RDI, a.msg_hello);
    asm.call_got(a.puts);

    // TRACE "One random number: %d" — below the DEBUG minimum, so the HLE
    // records the call but emits nothing.
    asm.zero_args();
    asm.call_got(a.rand);
    asm.mov_r64(R9, RAX);
    asm.lea_rip(RDI, a.file);
    asm.mov_r32_imm(RSI, 33);
    asm.mov_r32_imm(RDX, 0);
    asm.lea_rip(RCX, a.component);
    asm.lea_rip(R8, a.fmt_trace);
    asm.call_got(a.handler);

    // DEBUG "Two random numbers: %d, %d" — first vararg in r9, second on stack.
    asm.zero_args();
    asm.call_got(a.rand);
    asm.mov_r64(RBX, RAX);
    asm.zero_args();
    asm.call_got(a.rand);
    asm.mov_r64(R12, RAX);
    asm.push_r(R12);
    asm.lea_rip(RDI, a.file);
    asm.mov_r32_imm(RSI, 36);
    asm.mov_r32_imm(RDX, 1);
    asm.lea_rip(RCX, a.component);
    asm.lea_rip(R8, a.fmt_debug);
    asm.mov_r64(R9, RBX);
    asm.call_got(a.handler);
    asm.add_rsp(8);

    // WARNING "Three random numbers: %d, %d, %d %s" — first vararg in r9, the
    // remaining three on the stack pushed right-to-left.
    asm.zero_args();
    asm.call_got(a.rand);
    asm.mov_r64(RBX, RAX);
    asm.zero_args();
    asm.call_got(a.rand);
    asm.mov_r64(R12, RAX);
    asm.zero_args();
    asm.call_got(a.rand);
    asm.mov_r64(R13, RAX);
    asm.lea_rip(R14, a.mind);
    asm.push_r(R14);
    asm.push_r(R13);
    asm.push_r(R12);
    asm.lea_rip(RDI, a.file);
    asm.mov_r32_imm(RSI, 39);
    asm.mov_r32_imm(RDX, 3);
    asm.lea_rip(RCX, a.component);
    asm.lea_rip(R8, a.fmt_warning);
    asm.mov_r64(R9, RBX);
    asm.call_got(a.handler);
    asm.add_rsp(24);

    // ERROR "Four random numbers: %d, %d, %d, %d\n%s" — first vararg in r9, the
    // remaining four on the stack pushed right-to-left.
    asm.zero_args();
    asm.call_got(a.rand);
    asm.mov_r64(RBX, RAX);
    asm.zero_args();
    asm.call_got(a.rand);
    asm.mov_r64(R12, RAX);
    asm.zero_args();
    asm.call_got(a.rand);
    asm.mov_r64(R13, RAX);
    asm.zero_args();
    asm.call_got(a.rand);
    asm.mov_r64(R14, RAX);
    asm.lea_rip(R15, a.daisy);
    asm.push_r(R15);
    asm.push_r(R14);
    asm.push_r(R13);
    asm.push_r(R12);
    asm.lea_rip(RDI, a.file);
    asm.mov_r32_imm(RSI, 42);
    asm.mov_r32_imm(RDX, 4);
    asm.lea_rip(RCX, a.component);
    asm.lea_rip(R8, a.fmt_error);
    asm.mov_r64(R9, RBX);
    asm.call_got(a.handler);
    asm.add_rsp(32);

    asm.xor_eax_eax();
    asm.add_rsp(8);
    asm.ret();
    asm.into_bytes()
}

/// Fill byte for padded code pages: `int3`, so execution that runs past the
/// emitted instructions traps loudly instead of gliding into adjacent data.
pub const CODE_FILL: u8 = 0xcc;

/// Pad `code` to a page boundary with [`CODE_FILL`].
pub fn padded_code(code: &[u8]) -> Vec<u8> {
    let mut out = code.to_vec();
    out.resize(round_up(out.len()), CODE_FILL);
    out
}

fn round_up(len: usize) -> usize {
    const PAGE: usize = 0x1000;
    if len.is_multiple_of(PAGE) {
        len
    } else {
        (len / PAGE + 1) * PAGE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ret_is_single_byte() {
        assert_eq!(ret(), [0xc3]);
    }

    #[test]
    fn puts_and_ret_emits_expected_sequence() {
        let code = puts_and_ret(0x1000, 0x2000, 0x2010);
        assert_eq!(
            code,
            [
                0x48, 0x8D, 0x3D, 0xF9, 0x0F, 0x00, 0x00, //
                0x48, 0x8B, 0x05, 0x02, 0x10, 0x00, 0x00, //
                0xFF, 0xD0, 0xC3,
            ]
        );
    }

    #[test]
    fn puts_and_ret_ends_with_ret() {
        let code = puts_and_ret(0x1000, 0x2000, 0x2010);
        assert_eq!(code[code.len() - 1], 0xC3);
        assert_eq!(code.len(), 17);
    }

    #[test]
    fn padded_code_rounds_to_page() {
        assert_eq!(padded_code(&[0xc3]).len(), 0x1000);
        assert_eq!(padded_code(&[0xc3])[0], 0xc3);
        assert_eq!(padded_code(&[0xc3])[1], 0xcc);
        assert_eq!(padded_code(&[0xc3])[0xfff], 0xcc);
    }

    #[test]
    fn page_multiple_not_resized() {
        let code = vec![0x90; 0x2000];
        assert_eq!(padded_code(&code).len(), 0x2000);
    }

    fn asm() -> Asm {
        Asm::new(0x1000)
    }

    #[test]
    fn asm_ret_is_single_byte() {
        let mut a = asm();
        a.ret();
        assert_eq!(a.into_bytes(), [0xC3]);
    }

    #[test]
    fn asm_zero_args_zeroes_all_argument_registers() {
        let mut a = asm();
        a.zero_args();
        assert_eq!(
            a.into_bytes(),
            [
                0x31, 0xFF, 0x31, 0xF6, 0x31, 0xD2, 0x31, 0xC9, 0x45, 0x31, 0xC0, 0x45, 0x31, 0xC9
            ]
        );
    }

    #[test]
    fn asm_lea_rip_matches_hand_written_encoding() {
        let mut a = asm();
        a.lea_rip(reg::RDI, 0x2000);
        let lea = a.into_bytes();
        assert_eq!(&lea[..7], &[0x48, 0x8D, 0x3D, 0xF9, 0x0F, 0x00, 0x00]);
        assert_eq!(lea.len(), 7);
    }

    #[test]
    fn asm_call_got_displacement_is_computed_from_position() {
        let mut a = asm();
        a.lea_rip(reg::RDI, 0x2000);
        a.call_got(0x2018);
        let code = a.into_bytes();
        assert_eq!(code.len(), 13);
        assert_eq!(&code[7..9], &[0xFF, 0x15]);
        let disp = i32::from_le_bytes(code[9..13].try_into().unwrap());
        assert_eq!(disp, 0x2018 - (0x1007 + 6));
    }

    #[test]
    fn asm_mov_r64_and_push_r_emit_expected_bytes() {
        let mut a = asm();
        a.mov_r64(reg::R12, reg::RAX);
        a.mov_r64(reg::R9, reg::RBX);
        a.push_r(reg::R12);
        a.push_r(reg::R15);
        assert_eq!(
            a.into_bytes(),
            [0x49, 0x89, 0xC4, 0x49, 0x89, 0xD9, 0x41, 0x54, 0x41, 0x57]
        );
    }

    #[test]
    fn asm_stack_ops_emit_expected_bytes() {
        let mut a = asm();
        a.sub_rsp(8);
        a.add_rsp(24);
        assert_eq!(
            a.into_bytes(),
            [0x48, 0x83, 0xEC, 0x08, 0x48, 0x83, 0xC4, 0x18]
        );
    }

    fn test_addrs() -> LibdbgBasicAddrs {
        let got = [0x2050, 0x2058, 0x2060, 0x2068];
        let msg = [
            0x2000, 0x2019, 0x2023, 0x202B, 0x2042, 0x205F, 0x2085, 0x20AD, 0x20BE,
        ];
        LibdbgBasicAddrs {
            min_level: got[0],
            puts: got[1],
            rand: got[2],
            handler: got[3],
            msg_hello: msg[0],
            file: msg[1],
            component: msg[2],
            fmt_trace: msg[3],
            fmt_debug: msg[4],
            fmt_warning: msg[5],
            fmt_error: msg[6],
            mind: msg[7],
            daisy: msg[8],
        }
    }

    #[test]
    fn libdbg_basic_code_starts_and_ends_with_expected_shape() {
        let code = libdbg_basic_code(&test_addrs());
        assert_eq!(code[0], 0x48);
        assert_eq!(code[1], 0x83);
        assert_eq!(code[2], 0xEC);
        assert_eq!(code[3], 0x08);
        assert_eq!(code[code.len() - 1], 0xC3);
        assert!(code.len() < 0x200, "fixture code should stay small");
    }

    #[test]
    fn libdbg_basic_code_makes_sixteen_got_calls() {
        let code = libdbg_basic_code(&test_addrs());
        let calls = code.windows(2).filter(|w| *w == [0xFF, 0x15]).count();
        assert_eq!(calls, 16, "one call per import invocation");
    }

    #[test]
    fn libdbg_basic_code_pushes_eight_stack_varargs() {
        let code = libdbg_basic_code(&test_addrs());
        let pushes = code
            .windows(2)
            .filter(|w| w[0] == 0x41 && (0x50..=0x57).contains(&w[1]))
            .count();
        assert_eq!(pushes, 8, "1 + 3 + 4 stack varargs");
    }
}
