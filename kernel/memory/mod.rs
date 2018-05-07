//! The memory management subsystem.

use x86_64::structures::idt::{ExceptionStackFrame, PageFaultErrorCode};

use interrupts::idt;

pub use self::heap::KernelAllocator;

mod heap;

/// Initialize memory-related subsystems
pub fn init(allocator: &mut KernelAllocator, kheap_start: usize, kheap_size: usize) {
    // init the heap
    heap::init(allocator, kheap_start, kheap_size);

    // Register page fault handler
    unsafe {
        idt.page_fault.set_handler_fn(handle_page_fault);
    }
}

/// Handle a page fault
extern "x86-interrupt" fn handle_page_fault(
    _esf: &mut ExceptionStackFrame,
    _error: PageFaultErrorCode,
) {
    // Read CR2 to get the page fault address
    let cr2: usize;
    unsafe {
        asm!{
            "movq %cr2, $0"
             : "=r"(cr2)
             : /* no input */
             : /* no clobbers */
             : "volatile"
        };
    }

    // TODO
    panic!("Page fault at {:x}", cr2);
}
