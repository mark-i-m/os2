//! The memory management subsystem.

pub use self::heap::KernelAllocator;

mod heap;
mod paging;

/// Initialize memory-related subsystems
pub fn init(allocator: &mut KernelAllocator, kheap_start: usize, kheap_size: usize) {
    // init the heap
    heap::init(allocator, kheap_start, kheap_size);

    // Setup paging
    paging::init();
}

/// Initialize the page fault handler entry in the IDT.
pub unsafe fn init_pf_handler() {
    crate::interrupts::idt64
        .page_fault
        .set_handler_fn(crate::memory::paging::handle_page_fault);
}
