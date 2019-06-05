#![feature(
    lang_items,
    asm,
    alloc_error_handler,
    box_syntax,
    abi_x86_interrupt,
    panic_info_message,
    drain_filter,
    naked_functions
)]
// Compile without libstd
#![no_std]
#![no_main]
#![crate_type = "staticlib"]
#![crate_name = "kernel"]

extern crate alloc;

#[macro_use]
mod debug;
mod bare_bones;
#[macro_use]
mod cap;
mod continuation;
mod interrupts;
mod io;
mod memory;
mod sched;
mod time;

use alloc::vec;

use bootloader::BootInfo;

use crate::continuation::{ContResult, Continuation, Event, EventKind};
use crate::time::SysTime;

/// The kernel heap
#[global_allocator]
static mut ALLOCATOR: memory::KernelAllocator = memory::KernelAllocator::new();

bootloader::entry_point!(kernel_main);

/// This is the entry point to the kernel. It is the first rust code that runs.
#[no_mangle]
fn kernel_main(boot_info: &'static BootInfo) -> ! {
    use crate::sched::user;

    // At this point we are still in the provisional environment with
    // - the temporary page tables (first 2MiB of memory direct mapped)
    // - no IDT
    // - no current task

    // Make sure interrupts are off
    x86_64::instructions::interrupts::disable();

    // Let everyone know we are here
    printk!("\nYo Yo Yo! Made it to `kernel_main`! Hooray!\n");

    // Initialize memory
    // make the kernel heap 1MiB - 4KiB starting at 1MiB + 4KiB. This extra page will be unmapped
    // later to protect against heap overflows (unlikely as that is)...
    printk!("Memory ...\n");
    memory::init(unsafe { &mut ALLOCATOR }, boot_info);
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

    // TODO: remove testing code
    {
        use x86_64::structures::paging::PageTableFlags;

        let user_code_section = crate::memory::VirtualMemoryRegion::alloc_with_guard(1); // TODO
        printk!("test1 {:?}\n", user_code_section);

        let user_code_section = user_code_section.register();

        printk!("test3 {:?}\n", user_code_section);

        // Map the code section.
        crate::memory::map_region(
            user_code_section,
            PageTableFlags::PRESENT
                | PageTableFlags::WRITABLE
                | PageTableFlags::USER_ACCESSIBLE
                | PageTableFlags::NO_EXECUTE,
        );

        panic!("success");
    }

    // Create the init task
    printk!("Taskes");
    sched::init(Continuation::new(|_| {
        printk!("Init task running!\n");
        ContResult::Success(vec![(
            EventKind::Until(SysTime::now().after(4)),
            Continuation::new(|_| {
                printk!("Init waited for 4 seconds! Success 🎉\n");
                ContResult::Success(vec![(
                    EventKind::Keyboard,
                    Continuation::new(|ev| {
                        if let Event::Keyboard(c) = ev {
                            printk!("User typed '{}'\n", c as char);
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
    sched::start();

    // We never return...
}
