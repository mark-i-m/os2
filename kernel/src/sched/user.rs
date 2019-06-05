//! Switch to usermode

use x86_64::structures::paging::PageTableFlags;

use crate::{
    cap::ResourceHandle,
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

    // TODO: load the code

    // TODO: this is test code that is an infinite loop followed by nops
    let start_addr = user_code_section.with(|cap| {
        const TEST_CODE: &[u8] = &[
            0xEB, 0xFE, // here: jmp here
            0x90, // nop
            0x90, // nop
            0x90, // nop
            0x90, // nop
            0x90, // nop
            0x90, // nop
            0x90, // nop
            0x90, // nop
        ];

        unsafe {
            let start = cap_unwrap!(VirtualMemoryRegion(cap)).start();
            for (i, b) in TEST_CODE.iter().enumerate() {
                start.offset(i as isize).write(*b);
            }
            start as usize
        }
    });

    (user_code_section, start_addr)
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
    // Get new rsp and rip values.
    let rsp = stack.with(|cap| {
        let region = cap_unwrap!(VirtualMemoryRegion(cap));
        let start = region.start();
        let len = region.len();
        unsafe { start.offset(len as isize) }
    });

    let (_handle, rip) = code;

    // TODO: use sysret
    //
    // https://software.intel.com/sites/default/files/managed/39/c5/325462-sdm-vol-1-2abcd-3abcd.pdf#G43.25974
    //
    // TODO: use WRMSR to set the following MSRs as needed. This should probably be done just once
    // at boot time. For sysret (kernel -> user):
    // - user code segment: IA32_STAR[63:48] + 16
    // - stack segment:     IA32_STAR[64:48] + 8
    //
    // TODO: and for syscall (user -> kernel):
    // - kernel code segment: IA32_STAR[47:32]
    // - stack segment:       IA32_STAR[47:32] + 8
    // - kernel rip:          IA32_LSTAR
    // - kernel rflags:       %rflags & !(IA32_FMASK)
    //
    // TODO: for syscall handling: see the warnings at the end of the above chapter in the Intel
    // SDM (e.g. regarding interrupts, user stack)
    //
    // TODO: so in this routine (switch_to_user), all we need to do is set the following and
    // execute the `sysret` instruction:
    // - user rip: load into rcx before sysret
    // - rflags: load into r11 before sysret
    // - also want to set any register values to be given to the user
    //      - user rsp
    //      - clear all other regs

    unreachable!();
}
