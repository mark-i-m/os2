//! System calls and kernel <-> user mode switching...

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use elfloader::{ElfBinary, ElfLoader, LoadableHeaders, Rela, TypeRela64, VAddr, P64};

use x86_64::{
    registers::{
        model_specific::{Efer, EferFlags, Msr},
        rflags::{self, RFlags},
    },
    structures::paging::{PageSize, PageTableFlags, Size4KiB},
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

/// An ELF loader that loads binaries for execution in userspace.
struct KElfLoader {
    /// Base virtual address for the binary. All addresses in the binary are offset by this
    /// address.
    vbase: u64,

    /// Resource handles for all code sections loaded, indexed by starting address of the ELF
    /// region in memory.
    user_code_sections: BTreeMap<u64, ResourceHandle>,
}

impl KElfLoader {
    pub fn new() -> Self {
        KElfLoader {
            vbase: crate::memory::AVAILABLE_VADDR_START,
            user_code_sections: BTreeMap::new(),
        }
    }

    /// Get the address at which `raw_address` has been loaded.
    pub fn compute_loaded_address(&self, address: u64) -> u64 {
        let (base, loaded_base) = self
            .user_code_sections
            .range(((address >> 12) << 12)..=address)
            .next()
            .map(|(base, address)| {
                (
                    base,
                    address.with(|cap| unsafe { cap_unwrap!(VirtualMemoryRegion(cap)).start() }),
                )
            })
            .unwrap();

        let diff = address - base;

        let start = unsafe { loaded_base.add(diff as usize) };

        start as u64
    }
}

impl ElfLoader for KElfLoader {
    fn allocate(&mut self, load_headers: LoadableHeaders) -> Result<(), &'static str> {
        for header in load_headers {
            let size = header.mem_size();
            let size = if size % Size4KiB::SIZE == 0 {
                size >> 12
            } else {
                (size >> 12) + 1
            };
            let user_code_section = VirtualMemoryRegion::alloc_with_guard(size as usize).register();

            // Map the code section.
            map_region(
                user_code_section,
                PageTableFlags::PRESENT
                    | PageTableFlags::WRITABLE
                    | PageTableFlags::USER_ACCESSIBLE,
            );

            self.user_code_sections
                .insert(header.virtual_addr(), user_code_section);
        }

        Ok(())
    }

    fn relocate(&mut self, entry: &Rela<P64>) -> Result<(), &'static str> {
        let typ = TypeRela64::from(entry.get_type());
        let addr: *mut u64 = (self.vbase + entry.get_offset()) as *mut u64;

        match typ {
            TypeRela64::R_RELATIVE => {
                // This is a relative relocation, add the offset (where we put our
                // binary in the vspace) to the addend and we're done.
                todo!(
                    "R_RELATIVE *{:p} = {:#x}",
                    addr,
                    self.vbase + entry.get_addend()
                );
                Ok(())
            }
            _ => Err("Unexpected relocation encountered"),
        }
    }

    fn load(&mut self, base: VAddr, region: &[u8]) -> Result<(), &'static str> {
        let user_code_section = self.user_code_sections[&base];

        // Load the segment at base + self.vbase
        user_code_section.with(|cap| unsafe {
            let start = cap_unwrap!(VirtualMemoryRegion(cap)).start();
            for (i, b) in region.iter().enumerate() {
                start.offset(i as isize).write(*b);
            }
        });

        Ok(())
    }
}

/// Allocates virtual address space(s), adds appropriate page table mappings, loads the given ELF
/// binary into the allocated memory. `binary` should be the bytes of an ELF file, including the
/// magic bytes, headers, text, etc.
///
/// Returns the virtual address regions where the code has been loaded and the first RIP to start
/// executing.
pub fn load_user_elf(binary: &[u8]) -> (Vec<ResourceHandle>, u64) {
    let mut loader = KElfLoader::new();
    let bin = ElfBinary::new("user", binary).expect("Not an ELF binary");
    bin.load(&mut loader).expect("Unable to load ELF binary");

    let entry = loader.compute_loaded_address(bin.entry_point());

    (
        loader
            .user_code_sections
            .into_iter()
            .map(|(_, rh)| rh)
            .collect(),
        entry,
    )
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

pub fn start_user_task(start_rip: u64, start_rsp: u64) -> ! {
    // Enable interrupts for user mode.
    let rflags = (rflags::read() | rflags::RFlags::INTERRUPT_FLAG).bits();

    printk!(
        "Starting user task at rip={:x}, rsp={:x}\n",
        start_rip,
        start_rsp
    );

    // Initial registers zeroed except for the specified ones.
    let registers = SavedRegs {
        rip: start_rip,
        rsp: start_rsp,
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
        todo!("Enable interrupts here");
        // x86_64::instructions::interrupts::enable();

        // Handle the system call. The syscall number is passed in %rax.
        match saved_regs.rax {
            0 => {
                printk!("Task completed.\n");

                crate::sched::sched();
            }
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
                # load address of `registers` to `rcx` in inline asm

                # restore registers
                movq     (%rcx), %rax
                movq  0x8(%rcx), %rbx

                movq 0x18(%rcx), %rdx
                movq 0x20(%rcx), %rdi
                movq 0x28(%rcx), %rsi
                movq 0x30(%rcx), %rbp
                movq 0x38(%rcx), %r8
                movq 0x40(%rcx), %r9
                movq 0x48(%rcx), %r10

                movq 0x58(%rcx), %r12
                movq 0x60(%rcx), %r13
                movq 0x68(%rcx), %r14
                movq 0x70(%rcx), %r15

                # user rflags
                movq 0x78(%rcx), %r11

                # disable interrupts before loading the user stack; otherwise, an interrupt may be
                # serviced on the wrong stack.
                cli

                # no more stack refs until sysret
                movq 0x88(%rcx), %rsp

                # user rip
                movq 0x80(%rcx), %rcx

                # return to usermode (ring 3)
                sysretq
                "
                : /* no outputs */
                : "{rcx}"(registers)
                : "memory", "rax", "rbx", "rcx", "rdx", "rdi", "rsi", "r8", "r9", "r10", "r11",
                  "r12", "r13", "r14", "r15", "rbp", "rsp", "stack"
                : "volatile"
            );
        }

        unreachable!();
    }
}
