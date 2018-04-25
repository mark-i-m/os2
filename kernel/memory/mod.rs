pub use self::heap::KernelAllocator;

use interrupts::add_trap_handler;
use machine::page_fault_handler;

mod heap;

/// Initialize memory-related subsystems
pub fn init(allocator: &mut KernelAllocator, kheap_start: usize, kheap_size: usize) {
    // init the heap
    heap::init(allocator, kheap_start, kheap_size);

    // Register page fault handler
    add_trap_handler(14, page_fault_handler, 0);
}

/// Placeholder... TODO
pub fn handle_page_fault(_: usize) {
    // TODO: replace this
}
