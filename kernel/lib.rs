#![feature(
    lang_items, asm, start, const_fn, naked_functions, alloc, global_allocator, allocator_api,
    box_syntax, abi_x86_interrupt, panic_implementation, panic_info_message
)]
// Compile without libstd
#![no_std]
#![crate_type = "staticlib"]
#![crate_name = "kernel"]

extern crate alloc;
extern crate buddy;
extern crate rlibc;
extern crate smallheap;
extern crate spin;
extern crate x86_64;
extern crate os_bootinfo;

#[macro_use]
mod debug;
mod bare_bones;
mod continuation;
mod interrupts;
mod memory;
mod process;
mod time;

use continuation::{ContResult, Continuation, EventKind};
use time::SysTime;

/// The kernel heap
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
    x86_64::instructions::interrupts::disable();

    // Let everyone know we are here
    printk!("\nYo Yo Yo! Made it to `kernel_main`! Hooray!\n");

    // Set up TSS
    printk!("TSS");
    //interrupts::tss_init(); // TODO
    printk!(" ✔\n");

    // Set up interrupt/exception handling
    printk!("Interrupts...\n\t");
    interrupts::init();
    printk!("Interrupts ✔\n");

    // Initialize memory
    // make the kernel heap 3MiB starting at 1MiB.
    printk!("Memory ...\n\t");
    memory::init(unsafe { &mut ALLOCATOR }, 1 << 20, 1 << 20);
    printk!("Memory ✔\n");

    // Create the init task
    printk!("Taskes");
    process::init(Continuation::new(|_| {
        printk!("Init task running!\n");
        ContResult::Success(
            EventKind::Until(SysTime::now().after(4)),
            Continuation::new(|_| {
                printk!("Init waited for 4 seconds! Success 🎉\n");
                ContResult::Done
            }),
        )
    }));
    printk!(" ✔\n");

    // We can turn on interrupts now.
    x86_64::instructions::interrupts::enable();

    // Start the first task
    process::start();

    // We never return...
}
