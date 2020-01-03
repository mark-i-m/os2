//! System calls and kernel <-> user mode switching...

use x86_64::{
    registers::{
        model_specific::{Efer, EferFlags, Msr},
        rflags::{self, RFlags},
    },
    structures::paging::PageTableFlags,
};

use crate::{
    cap::ResourceHandle,
    interrupts::SELECTORS,
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

#[derive(Debug, Default)]
#[repr(C)]
struct SavedRegs {
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rbp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,

    pub rflags: u64,
    pub rip: u64,

    pub rsp: u64,
}

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
            0x0f, 0x05, // syscall
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
        LSTAR.write(syscall::entry as u64);

        // FMASK: rflags mask: any set bits are cleared on syscall
        //
        // Want to disable interrupt until we switch to the kernel stack.
        FMASK.write(RFlags::INTERRUPT_FLAG.bits());
    }
}

pub fn start_user_task(code: (ResourceHandle, usize), stack: ResourceHandle) -> ! {
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

    let registers = SavedRegs {
        rip: rip as u64,
        rsp: rsp as u64,
        rflags,
        ..SavedRegs::default()
    };

    syscall::switch_to_user(&registers)
}

mod syscall {
    //! System call handling.

    use super::SavedRegs;

    /// Handle a `syscall` instruction from userspace.
    ///
    /// This is not to be called from kernel mode! And it should never be called more than once at a
    /// time.
    ///
    /// Interrupts are disabled on entry.
    ///
    /// Contract with userspace (beyond what the ISA does):
    /// - System call argument is passed in %rax
    /// - We may clobber %rdx
    /// - We will save and restore all other registers, including the stack pointer
    /// - We will return values in %rax
    #[naked]
    pub(super) unsafe extern "C" fn entry() {
        // Switch to tmp stack, save user regs
        asm!(
            "
            # save the user stack pointer to %rdx before we switch stacks.
            mov %rsp, %rdx

            # switch to the tmp stack
            mov $0, %rsp
            mov (%rsp), %rsp

            # start saving stuff
            pushq %rdx # user rsp
            pushq %rcx # user rip
            pushq %r11 # user rflags

            pushq %r15
            pushq %r14
            pushq %r13
            pushq %r12
            pushq %r11
            pushq %r10
            pushq %r9
            pushq %r8
            pushq %rbp
            pushq %rsi
            pushq %rdi
            pushq %rdx
            pushq %rcx
            pushq %rbx
            pushq %rax

            # handle the system call. The saved registers are passed at the top of the stack where
            # we just pushed them.
            mov %rsp, %rdi
            call handle_syscall
            "
            : /* no outputs */
            : "i"(&super::super::CURRENT_STACK_HEAD)
            : "memory", "rax", "rbx", "rcx", "rdx", "rdi", "rsi", "r8", "r9", "r10", "r11", "r12",
              "r13", "r14", "r15", "rbp", "stack"
            : "volatile"
        );

        unreachable!();
    }

    /// Does the actual work of handling a syscall. Should only be called by `syscall_entry`. This
    /// assumes we are still running on the tmp stack. It switches to the saved kernel stack.
    #[no_mangle]
    unsafe extern "C" fn handle_syscall(saved_regs: &mut SavedRegs) {
        // TODO: can probably enable interrupts here...

        // Handle the system call. The syscall number is passed in %rax.
        match saved_regs.rax {
            n => printk!("syscall #{:#x?}\n", n),
        }

        // Return to usermode
        switch_to_user(saved_regs)
    }

    /// Switch to user mode with the given registers.
    pub(super) fn switch_to_user(registers: &SavedRegs) -> ! {
        // https://software.intel.com/sites/default/files/managed/39/c5/325462-sdm-vol-1-2abcd-3abcd.pdf#G43.25974
        //
        // Set the following and execute the `sysret` instruction:
        // - user rip: load into rcx before sysret
        // - rflags: load into r11 before sysret
        // - also want to set any register values to be given to the user
        //      - user rsp
        //      - clear all other regs
        unsafe {
            asm!(
                "
                # restore registers
                movq $0, %rax
                movq $1, %rbx
                movq $2, %rdx
                movq $3, %rdi
                movq $4, %rsi
                movq $5, %rbp
                movq $6, %r8
                movq $7, %r9
                movq $8, %r10
                movq $9, %r12
                movq $10, %r13
                movq $11, %r14
                movq $12, %r15

                # user rflags
                movq $13, %r11

                # user rip
                movq $14, %rcx

                # disable interrupts before loading the user stack; otherwise, an interrupt may be
                # serviced on the wrong stack.
                cli

                # no more stack refs until sysret
                movq $15, %rsp

                # return to usermode (ring 3)
                sysretq
                "
                : /* no outputs */
                : "m"(registers.rax)
                , "m"(registers.rbx)
                , "m"(registers.rdx)
                , "m"(registers.rdi)
                , "m"(registers.rsi)
                , "m"(registers.rbp)
                , "m"(registers.r8)
                , "m"(registers.r9)
                , "m"(registers.r10)
                , "m"(registers.r12)
                , "m"(registers.r13)
                , "m"(registers.r14)
                , "m"(registers.r15)
                , "m"(registers.rflags)
                , "m"(registers.rip)
                , "m"(registers.rsp)
                : "memory", "rax", "rbx", "rcx", "rdx", "rdi", "rsi", "r8", "r9", "r10", "r11", "r12",
                  "r13", "r14", "r15", "rbp"
                : "volatile"
            );
        }

        unreachable!();
    }
}
