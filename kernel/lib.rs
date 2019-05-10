#![feature(
    lang_items,
    asm,
    alloc_error_handler,
    box_syntax,
    abi_x86_interrupt,
    panic_info_message,
    drain_filter
)]
// Compile without libstd
#![no_std]
#![crate_type = "staticlib"]
#![crate_name = "kernel"]

extern crate alloc;
extern crate buddy;
extern crate os_bootinfo;
extern crate rand;
extern crate rlibc;
extern crate smallheap;
extern crate spin;
extern crate x86_64;

#[macro_use]
mod debug;
mod bare_bones;
mod cap;
mod continuation;
mod interrupts;
mod io;
mod memory;
mod process;
mod time;

use alloc::vec;

use continuation::{ContResult, Continuation, Event, EventKind};
use time::SysTime;

/// The kernel heap
#[global_allocator]
static mut ALLOCATOR: memory::KernelAllocator = memory::KernelAllocator::new();

/// This is the entry point to the kernel. It is the first rust code that runs.
#[no_mangle]
pub fn kernel_main() -> ! {
    use crate::process::user;

    // At this point we are still in the provisional environment with
    // - the temporary page tables (first 2MiB of memory direct mapped)
    // - no IDT
    // - no current process

    // Make sure interrupts are off
    x86_64::instructions::interrupts::disable();

    // Let everyone know we are here
    printk!("\nYo Yo Yo! Made it to `kernel_main`! Hooray!\n");

    // Initialize memory
    // make the kernel heap 1MiB - 4KiB starting at 1MiB + 4KiB. This extra page will be unmapped
    // later to protect against heap overflows (unlikely as that is)...
    printk!("Memory ...\n\t");
    memory::init(unsafe { &mut ALLOCATOR });
    printk!("Memory ✔\n");

    // Set up interrupt/exception handling
    printk!("Interrupts...\n\t");
    interrupts::init();
    printk!("Interrupts ✔\n");

    // I/O
    printk!("I/O ...\n");
    io::init();
    printk!("I/O ✔\n");

    // Capabilities
    printk!("Capabilities ...\n");
    cap::init();
    printk!("Capabilities ✔\n");

    // Create the init task
    printk!("Taskes");
    process::init(Continuation::new(|_| {
        printk!("Init task running!\n");
        ContResult::Success(vec![(
            EventKind::Until(SysTime::now().after(4)),
            Continuation::new(|_| {
                printk!("Init waited for 4 seconds! Success 🎉\n");
                ContResult::Success(vec![(
                    EventKind::Keyboard,
                    Continuation::new(|ev| {
                        if let Event::Keyboard(c) = ev {
                            printk!("User typed '{}'", c as char);
                        } else {
                            unreachable!();
                        }

                        ContResult::Success(vec![(
                            EventKind::Now,
                            Continuation::new(|_| {
                                printk!("Attempting to switch to user!");

                                let code = user::load_user_code_section();
                                let stack = user::allocate_user_stack();
                                user::switch_to_user(code, stack);
                            }),
                        )])
                    }),
                )])
            }),
        )])
    }));
    printk!(" ✔\n");

    // We can turn on interrupts now.
    x86_64::instructions::interrupts::enable();

    // Start the first task
    process::start();

    // We never return...
}
