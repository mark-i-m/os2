
#![feature(lang_items, asm, start)]

// Compile without libstd
#![no_std]

#![crate_type = "staticlib"]
#![crate_name = "kernel"]

extern crate rlibc;

#[macro_use]
mod debug;
mod bare_bones;
mod machine;

/// This is the entry point to the kernel. It is the first rust code that runs.
#[no_mangle]
pub fn main() -> ! {
    // TODO: will need to enter from the bootloader...
    panic!("Hello, world");
}
