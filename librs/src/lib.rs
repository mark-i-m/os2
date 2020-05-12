//! A library for userspace programs to use that exposes kernel functionality in a typed way, sort
//! of like `libc` does. The difference is that it is **unsound** to try to use the kernel via a
//! c-like interface; you **must** use this library because the kernel ABI is an unstable, typed,
//! Rust ABI (and Rust's ABI is unstable).

#![no_std]
#![feature(llvm_asm, start)]

pub mod bare_bones;

/// Instructs the kernel to terminate the current task and free its resources. The exit `code` is
/// passed to the kernel.
pub fn exit(code: isize) -> ! {
    unsafe {
        llvm_asm!(
            "
        __librs_exit:
            syscall
            jmp __librs_exit
            "
            : /* no outputs */
            : "{rax}"(code)
            : "stack", "memory"
            : "volatile"
        );
    }

    unreachable!();
}
