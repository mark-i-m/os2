//! Switch to usermode

use crate::cap::{Capability, VirtualMemoryRegion};

/// Allocates virtual address space, adds appropriate page table mappings, loads the specified code
/// section into the allocated memory.
pub fn load_user_code_section() -> Capability<VirtualMemoryRegion> {
    unimplemented!();
    // TODO
}

/// Allocates virtual address space for the user stack (fixed size). Adds appropriate page table
/// mappings (read/write, not execute).
pub fn allocate_user_stack() -> Capability<VirtualMemoryRegion> {
    unimplemented!();
    // TODO
}

/// Switch to user mode, executing the given code with the given address.
pub fn switch_to_user(
    code: Capability<VirtualMemoryRegion>,
    stack: Capability<VirtualMemoryRegion>,
) -> ! {
    // TODO
    unimplemented!();
}
