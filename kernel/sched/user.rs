//! Switch to usermode

use crate::{
    cap::{ResourceHandle, VirtualMemoryRegion},
    memory::valloc,
};

/// Allocates virtual address space, adds appropriate page table mappings, loads the specified code
/// section into the allocated memory.
///
/// Returns the virtual address region where the code has been loaded and the first RIP to start
/// executing.
pub fn load_user_code_section() -> (ResourceHandle<VirtualMemoryRegion>, usize) {
    let user_code_section = valloc(3); // TODO: guard pages + size of text

    unimplemented!();
    // TODO
}

/// Allocates virtual address space for the user stack (fixed size). Adds appropriate page table
/// mappings (read/write, not execute).
///
/// Returns the virtual address region of the stack. The first and last pages are left unmapped as
/// guard pages. The stack should be used from the end (high-addresses) of the region (top of
/// stack), since it grows downward.
pub fn allocate_user_stack() -> ResourceHandle<VirtualMemoryRegion> {
    let user_stack = valloc(3); // TODO: guard pages + stack size

    unimplemented!();
    // TODO
}

/// Switch to user mode, executing the given code with the given address.
pub fn switch_to_user(
    code: (ResourceHandle<VirtualMemoryRegion>, usize),
    stack: ResourceHandle<VirtualMemoryRegion>,
) -> ! {
    // TODO
    unimplemented!();
}
