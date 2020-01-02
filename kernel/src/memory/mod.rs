//! The memory management subsystem.

use bootloader::BootInfo;

use x86_64::structures::idt::InterruptDescriptorTable;

use crate::interrupts::IRQ_IST_FRAME_INDEX;

pub use self::heap::KernelAllocator;
pub use self::paging::{map_region, VirtualMemoryRegion};

mod heap;
mod paging;

/// Initialize memory-related subsystems
pub fn init(allocator: &mut KernelAllocator, boot_info: &'static BootInfo) {
    // Set up a bare-bones heap so we can start initializing everything.
    heap::early::init(allocator);

    // Early paging init... just enough to set up the heap...
    paging::early_init(boot_info);

    // init the heap
    heap::init(
        allocator,
        paging::KERNEL_HEAP_START as usize,
        paging::KERNEL_HEAP_SIZE as usize,
    );

    // Setup paging
    paging::init(boot_info);
}

/// Initialize the page fault handler entry in the IDT.
pub unsafe fn init_pf_handler(idt: &mut InterruptDescriptorTable) {
    idt.page_fault
        .set_handler_fn(crate::memory::paging::handle_page_fault)
        .set_stack_index(IRQ_IST_FRAME_INDEX);
}
