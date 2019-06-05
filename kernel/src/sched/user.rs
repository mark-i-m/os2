//! Switch to usermode

use x86_64::structures::paging::PageTableFlags;

use crate::{
    cap::{Capability, ResourceHandle},
    memory::{map_region, VirtualMemoryRegion},
};

const USER_STACK_SIZE: usize = 1; // pages

/// Allocates virtual address space, adds appropriate page table mappings, loads the specified code
/// section into the allocated memory.
///
/// Returns the virtual address region where the code has been loaded and the first RIP to start
/// executing.
pub fn load_user_code_section() -> (ResourceHandle, usize) {
    let user_code_section = VirtualMemoryRegion::alloc_with_guard(1).register(); // TODO

    // Map the code section.
    map_region(
        user_code_section,
        PageTableFlags::PRESENT
            | PageTableFlags::WRITABLE
            | PageTableFlags::USER_ACCESSIBLE
            | PageTableFlags::NO_EXECUTE,
    );

    // TODO: load the code EBFE

    // TODO: test
    user_code_section.with(|cap| {
        let user_code_section = cap_unwrap!(VirtualMemoryRegion(cap));
        unsafe {
            *user_code_section.start() = 0;
        }
    });

    unimplemented!();
}

/// Allocates virtual address space for the user stack (fixed size). Adds appropriate page table
/// mappings (read/write, not execute).
///
/// Returns the virtual address region of the stack. The first and last pages are left unmapped as
/// guard pages. The stack should be used from the end (high-addresses) of the region (top of
/// stack), since it grows downward.
pub fn allocate_user_stack() -> ResourceHandle {
    // Allocate the stack the user will run on.
    let user_stack = VirtualMemoryRegion::alloc_with_guard(USER_STACK_SIZE).register();

    // Map the stack into the address space.
    map_region(
        user_stack,
        PageTableFlags::PRESENT
            | PageTableFlags::WRITABLE
            | PageTableFlags::USER_ACCESSIBLE
            | PageTableFlags::NO_EXECUTE,
    );

    user_stack
}

/// Switch to user mode, executing the given code with the given address.
pub fn switch_to_user(code: (ResourceHandle, usize), stack: ResourceHandle) -> ! {
    // TODO: setup the stack to do iret

    // TODO: smash the stack

    unimplemented!();
}
