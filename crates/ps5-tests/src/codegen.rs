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
}
