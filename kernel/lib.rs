#![feature(
    lang_items, asm, start, const_fn, naked_functions, alloc, global_allocator, allocator_api,
    box_syntax
)]
// Compile without libstd
#![no_std]
#![crate_type = "staticlib"]
#![crate_name = "kernel"]

extern crate alloc;
extern crate rlibc;
extern crate smallheap;
extern crate spin;

#[macro_use]
mod debug;
mod bare_bones;
mod machine;

mod interrupts;
mod memory;
mod process;

use process::ProcessResult;

/// The global allocator
#[global_allocator]
static mut ALLOCATOR: memory::KernelAllocator = memory::KernelAllocator::new();

/// This is the entry point to the kernel. It is the first rust code that runs.
#[no_mangle]
pub fn kernel_main() -> ! {
    // At this point we are still in the provisional environment with
    // - the temporary page tables (first 2MiB of memory direct mapped)
    // - no IDT
    // - no current process

    // Make sure interrupts are off
    unsafe {
        machine::cli();
    }

    // Let everyone know we are here
    printk!("\nYo Yo Yo! Made it to `kernel_main`! Hooray!\n");

    // Set up TSS
    printk!("TSS");
    interrupts::tss_init();
    printk!(" ✔\n");

    // Set up interrupt handling
    printk!("Interrupts");
    interrupts::init();
    printk!(" ✔\n");

    // Initialize memory
    // make the kernel heap 3MiB starting at 1MiB.
    printk!("Memory ...\n\t");
    memory::init(unsafe { &mut ALLOCATOR }, 1 << 20, 1 << 20);
    printk!("Memory ✔\n");

    // Create the init process
    printk!("Processes");
    process::init(|| {
        printk!("Init!");
        ProcessResult::Done
    });
    printk!(" ✔\n");

    process::sched();

    // We never return...
}
