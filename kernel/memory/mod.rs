//! The memory management subsystem.

use interrupts::idt64;

pub use self::heap::KernelAllocator;

mod heap;
mod paging;

/// Initialize memory-related subsystems
pub fn init(allocator: &mut KernelAllocator, kheap_start: usize, kheap_size: usize) {
    // init the heap
    heap::init(allocator, kheap_start, kheap_size);

    // Register page fault handler
    unsafe {
        idt64.page_fault.set_handler_fn(paging::handle_page_fault);
    }

    // Setup paging
    paging::init();
}
