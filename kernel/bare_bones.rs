//! This module contains some basic functionality that libstd would normally
//! otherwise provide. Most importantly, it defines `rust_begin_unwind` which is
//! used by `panic!`.

#![allow(private_no_mangle_fns)]

use core::fmt::{Arguments, Write};

use debug::Debug;
use x86_64::instructions::interrupts;

// For bare-bones rust
#[lang = "eh_personality"]
#[no_mangle]
pub fn eh_personality() {}

/// This function is used by `panic!` to display an error message.
#[lang = "panic_fmt"]
#[no_mangle]
pub extern "C" fn rust_begin_panic(
    args: Arguments,
    file: &'static str,
    line: u32,
    column: u32,
) -> ! {
    // we should no be interrupting any more
    interrupts::disable();

    printk!("\n========{{ PANIC }}========\n");
    printk!("{}:{}:{}\n", file, line, column);
    printk!("...........................\n");
    let _ = Debug.write_fmt(args);
    printk!("\n===========================\n");
    loop {}
}
