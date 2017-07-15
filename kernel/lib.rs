
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
pub fn kernel_main() -> ! {
    printk!("\n");
    printk!("Yo Yo Yo! Made it to `kernel_main`! Hooray!\n");

    panic!("Hello, world");
}
