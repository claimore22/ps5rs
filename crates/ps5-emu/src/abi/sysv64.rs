//! SysV x86-64 ABI boundary: guest entry, the import-stub dispatcher, and the
//! escape that unwinds from the guest stack back to the host caller.
//!
//! The guest runs as native x86-64 machine code, so every import funnels
//! through a machine-code stub that forwards the SysV integer registers and a
//! pointer to the guest's stack arguments into an [`ImportCallFrame`].
//! [`ImportCallFrame`] is the C-compatible record the stub builds on its own
//! stack; [`EscapeContext`] records the host stack position to resume at once
//! the guest calls `exit`.

use core::mem::size_of;
use std::arch::naked_asm;

/// Argument block a stub hands to the Rust dispatcher.
///
/// The stub preserves `rdi..r9`, points `stack_args` at the guest's first
/// stack argument (the seventh SysV argument), and stores its own import index
/// alongside.  Exactly 64 bytes on the stack.
#[repr(C)]
pub struct ImportCallFrame {
    pub rdi: u64,
    pub rsi: u64,
    pub rdx: u64,
    pub rcx: u64,
    pub r8: u64,
    pub r9: u64,
    pub stack_args: *const u64,
    pub import_index: u32,
    pub _pad: u32,
}

/// Host stack position and frame pointer saved just before control enters the
/// guest.
///
/// The escape sequence restores `rsp` and `rbp` from here and `ret`s straight
/// to the host caller with the exit code in `rax`, unwinding past the guest
/// stack.  `rbp` matters: the dispatcher pushes it and never pops it on the
/// escape path, so the host caller's frame pointer would otherwise be lost.
#[repr(C)]
pub struct EscapeContext {
    pub rsp: u64,
    pub rbp: u64,
}

const _: () = assert!(size_of::<ImportCallFrame>() == 64);
const _: () = assert!(size_of::<EscapeContext>() == 16);

/// Currently armed escape context.  The emulator is single-threaded, so one
/// process-global slot suffices.
#[unsafe(no_mangle)]
static mut ESCAPE_CTX: *const EscapeContext = core::ptr::null();

/// Address embedded in every HLE import stub.
///
/// Builds an [`ImportCallFrame`] from the live registers and the guest stack,
/// forwards it to `ps5emu_dispatch_frame`, then returns the handler result
/// to the stub, which pops its index and returns to the guest.
///
/// Only ever reached from machine code, so it carries no language-level ABI
/// contract beyond the SysV register layout.
#[unsafe(no_mangle)]
#[unsafe(naked)]
unsafe extern "sysv64" fn ps5emu_dispatcher() -> u64 {
    naked_asm!(
        "push rbp",
        "mov rbp, rsp",
        "and rsp, -16",
        "sub rsp, 64",
        "mov [rsp + 0], rdi",
        "mov [rsp + 8], rsi",
        "mov [rsp + 16], rdx",
        "mov [rsp + 24], rcx",
        "mov [rsp + 32], r8",
        "mov [rsp + 40], r9",
        "lea rax, [rbp + 32]",
        "mov [rsp + 48], rax",
        "mov rax, [rbp + 16]",
        "mov [rsp + 56], rax",
        "mov dword ptr [rsp + 60], 0",
        "mov rdi, rsp",
        "call ps5emu_dispatch_frame",
        "mov rsp, rbp",
        "pop rbp",
        "ret",
    );
}

/// Transfer control to the guest entry point.
///
/// Captures the host `rsp` into the armed [`EscapeContext`], switches to the
/// guest stack, and calls the guest `_start`.  If the entry point returns
/// (unusual for a well-formed crt0), returns exit code `0` to the caller.
#[unsafe(no_mangle)]
#[unsafe(naked)]
unsafe extern "sysv64" fn ps5emu_invoke_guest(entry: u64, stack_top: u64) -> u64 {
    naked_asm!(
        "mov rax, qword ptr [rip + ESCAPE_CTX]",
        "mov qword ptr [rax], rsp",
        "mov qword ptr [rax + 8], rbp",
        "mov rsp, rsi",
        "call rdi",
        "xor eax, eax",
        "mov rcx, qword ptr [rip + ESCAPE_CTX]",
        "mov rbp, qword ptr [rcx + 8]",
        "mov rsp, qword ptr [rcx]",
        "ret",
    );
}

/// Unwind from the guest stack back to the host caller with `code` in `rax`.
///
/// # Safety
///
/// `ctx` must be the [`EscapeContext`] armed by [`ps5emu_invoke_guest`], which
/// is guaranteed to outlive this call.
#[unsafe(no_mangle)]
#[unsafe(naked)]
unsafe extern "sysv64" fn ps5emu_escape(ctx: *const EscapeContext, code: u64) -> ! {
    naked_asm!(
        "mov rax, rsi",
        "mov rsp, qword ptr [rdi]",
        "mov rbp, qword ptr [rdi + 8]",
        "ret",
    );
}

/// Arm the escape context for an upcoming [`invoke_guest`].
///
/// # Safety
///
/// `ctx` must stay valid until the guest run completes.
pub unsafe fn arm_escape_ctx(ctx: *const EscapeContext) {
    unsafe { ESCAPE_CTX = ctx }
}

pub fn disarm_escape_ctx() {
    unsafe { ESCAPE_CTX = core::ptr::null() }
}

pub fn escape_ctx() -> *const EscapeContext {
    unsafe { ESCAPE_CTX }
}

pub fn invoke_guest(entry: u64, stack_top: u64) -> u64 {
    unsafe { ps5emu_invoke_guest(entry, stack_top) }
}

pub fn dispatcher_address() -> u64 {
    ps5emu_dispatcher as *const () as u64
}

/// # Safety
///
/// `ctx` must be the context armed by [`arm_escape_ctx`], valid until the
/// guest run completes.
pub unsafe fn escape(ctx: *const EscapeContext, code: u64) -> ! {
    unsafe { ps5emu_escape(ctx, code) }
}
