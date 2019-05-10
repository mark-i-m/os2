//! The memory management subsystem.

pub use self::heap::KernelAllocator;
pub use self::paging::valloc;

mod heap;
mod paging;

/// The first page of the kernel heap
const KERNEL_HEAP_START: usize = (1 << 20) + (1 << 12);

/// The guard page of the kernel heap
const KERNEL_HEAP_GUARD: u64 = (1 << 20);

/// The initial size of the kernel heap
const KERNEL_HEAP_SIZE: usize = (1 << 20) - (1 << 12);

/// Initialize memory-related subsystems
pub fn init(allocator: &mut KernelAllocator) {
    // init the heap
    heap::init(allocator, KERNEL_HEAP_START, KERNEL_HEAP_SIZE);

    // Setup paging
    paging::init();
}

/// Initialize the page fault handler entry in the IDT.
pub unsafe fn init_pf_handler() {
    crate::interrupts::idt64
        .page_fault
        .set_handler_fn(crate::memory::paging::handle_page_fault);
}
