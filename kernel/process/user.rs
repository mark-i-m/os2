//! Switch to usermode
//!
//! TODO: need to figure out a plan for permissions to access portions of the address space. Some
//! sort of capability system?

/// Allocates virtual address space, adds appropriate page table mappings, loads the specified code
/// section into the allocated memory.
pub fn load_user_code_section() -> EntryRIP {
    // TODO
}

/// Allocates virtual address space for the user stack (fixed size). Adds appropriate page table
/// mappings (read/write, not execute).
pub fn allocate_user_stack() -> StartRSP {
    // TODO
}

/// Switch to user mode, executing the given code with the given address.
pub fn switch_to_user(code: EntryRIP, stack: StartRSP) -> {
    // TODO
}
