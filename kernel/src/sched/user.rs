//! System calls and kernel <-> user mode switching...

use x86_64::{
    registers::{
        model_specific::{Efer, EferFlags, Msr},
        rflags,
    },
    structures::paging::PageTableFlags,
    VirtAddr,
};

use crate::{
    cap::ResourceHandle,
    interrupts::{SAVED_KERNEL_RSP_IST_FRAME_INDEX, SELECTORS, TSS},
    memory::{map_region, VirtualMemoryRegion},
};

const USER_STACK_SIZE: usize = 1; // pages

// Some MSRs used for system call handling.

/// Contains the stack and code segmets for syscall/sysret.
const STAR: Msr = Msr::new(0xC000_0081);

/// Contains the kernel rip for syscall handler.
const LSTAR: Msr = Msr::new(0xC000_0082);

/// Contains the kernel rflags mask for syscall.
const FMASK: Msr = Msr::new(0xC000_0084);

/// Allocates virtual address space, adds appropriate page table mappings, loads the specified code
/// section into the allocated memory.
///
/// Returns the virtual address region where the code has been loaded and the first RIP to start
/// executing.
pub fn load_user_code_section() -> (ResourceHandle, usize) {
    // TODO: Allocate enough space for the code we will load
    let user_code_section = VirtualMemoryRegion::alloc_with_guard(1).register();

    // Map the code section.
    map_region(
        user_code_section,
        PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE,
    );

    // TODO: load the code

    // TODO: this is test code that is an infinite loop followed by nops
    let start_addr = user_code_section.with(|cap| {
        const TEST_CODE: &[u8] = &[
            // here:
            0x54, // push %rsp
            0x58, // pop %rax
            //0x0f, 0x05, // syscall // TODO uncomment this when syscall handling is implemented
            0x90, 0x90, // nop; nop
            0xeb, 0xfa, // jmp here
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

/// Set some MSRs, registers to enable syscalls and user/kernel context switching.
pub fn init() {
    unsafe {
        // Need to set IA32_EFER.SCE
        Efer::update(|flags| *flags |= EferFlags::SYSTEM_CALL_EXTENSIONS);

        // STAR: Ring 0 and Ring 3 segments
        // - Kernel mode CS is bits 47:32
        // - Kernel mode SS is bits 47:32 + 8
        // - User mode CS is bits 63:48 + 16
        // - User mode SS is bits 63:48 + 8
        //
        // Each entry in the GDT is 8B...
        let selectors = SELECTORS.lock();
        let kernel_base: u16 = selectors.kernel_cs.index() * 8;
        let user_base: u16 = (selectors.user_ss.index() - 1) * 8;
        let star: u64 = ((kernel_base as u64) << 32) | ((user_base as u64) << 48);
        STAR.write(star);

        // LSTAR: Syscall Entry RIP
        LSTAR.write(handle_syscall as u64);

        // FMASK: rflags mask: any set bits are cleared on syscall
        // TODO: probably want to disable interrupts until we switch to kernel stack
        FMASK.write(0);
    }
}

/// Switch to user mode, executing the given code with the given address.
pub fn switch_to_user(code: (ResourceHandle, usize), stack: ResourceHandle) -> ! {
    // Compute new register values
    let rsp = stack.with(|cap| {
        let region = cap_unwrap!(VirtualMemoryRegion(cap));
        let start = region.start();
        let len = region.len();
        unsafe { start.offset(len as isize) }
    });

    let (_handle, rip) = code;

    // Enable interrupts for user mode.
    let rflags = (rflags::read() | rflags::RFlags::INTERRUPT_FLAG).bits();

    printk!(
        "Switching to user mode with rip={:x} rsp={:x} rflags={:b}\n",
        rip as u64,
        rsp as u64,
        rflags as u64,
    );

    // Save kernel stack location somewhere so that we can switch back to it during an interrupt.
    let mut kernel_rsp: u64;
    unsafe {
        asm!("
            mov %rsp, $0
            "
            : "=r"(kernel_rsp)
            : /* no inputs */
            : /* no clobbers */
            : "volatile"
        );
    }

    TSS.lock().as_mut().unwrap().interrupt_stack_table[SAVED_KERNEL_RSP_IST_FRAME_INDEX as usize] =
        VirtAddr::new(kernel_rsp);

    // https://software.intel.com/sites/default/files/managed/39/c5/325462-sdm-vol-1-2abcd-3abcd.pdf#G43.25974
    //
    // Set the following and execute the `sysret` instruction:
    // - user rip: load into rcx before sysret
    // - rflags: load into r11 before sysret
    // - also want to set any register values to be given to the user
    //      - user rsp
    //      - clear all other regs
    //
    // TODO: eventually we may want to have a general mechanism for restoring registers to know
    // values from a struct or something. For now, we just clear all registers.
    unsafe {
        asm!(
            "
            # clear other regs
            xor %rbx, %rbx
            xor %rdx, %rdx
            xor %rdi, %rdi
            xor %rsi, %rsi
            xor %r8 , %r8
            xor %r9 , %r9
            xor %r10, %r10
            xor %r12, %r12
            xor %r13, %r13
            xor %r14, %r14
            xor %r15, %r15

            # disable interrupts before loading the user stack; otherwise, an interrupt may be
            # serviced on the wrong stack.
            cli

            # no more stack refs until sysret
            mov %rax, %rsp

            # clear rax for simpler debugging
            xor %rax, %rax

            # return to usermode (ring 3)
            sysretq
            "
            : /* no outputs */
            : "{rcx}"(rip), "{r11}"(rflags), "{rax}"(rsp)
            : "memory", "rax", "rbx", "rcx", "rdx", "rdi", "rsi", "r8", "r9", "r10", "r11", "r12",
              "r13", "r14", "r15"
            : "volatile"
        );
    }

    unreachable!();
}

/// Handle a `syscall` instruction
#[naked]
extern "C" fn handle_syscall() {
    // TODO: switch to kernel stack, save user regs
    //
    // https://software.intel.com/sites/default/files/managed/39/c5/325462-sdm-vol-1-2abcd-3abcd.pdf#G43.25974
    //
    // TODO: for syscall handling: see the warnings at the end of the above chapter in the Intel
    // SDM (e.g. regarding interrupts, user stack)

    todo!("syscall");
}
