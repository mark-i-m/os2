//! This module contains everything needed for interrupts
//!
//! Some notes:
//! - 64-bit x86 doesn't use the SS anymore.
//! - LDT, TSS are considered "system segments" -- segments with special meaning
//! - You can read about segment selectors and GDT format in Intel SDM Vol 3:
//!     - ch 3.4.3: Segement registers
//!     - ch 3.4.4: Segments in 64-bit/long/ia-32e mode
//!     - ch 3.4.5: Segment descriptors (i.e. GDT, TSS, LDT, IDT format)
//!          - This is the same for 32- and 64-bit mode for GDT
//!          - IDT, LDT, and TSS changed formats for 64-bit mode
//!     - ch 3.5: System Segment Descriptors
//! - In the GDT, bits that aren't clearly defined as needed can just remain 0. This includes:
//!     - G, D/B, Limit, AVL
//! - In the GDT, the L bit should only be set for CS. For DS, it should remain 0, even if 64-bit.
//!
//! Here are the bits we want set for the Kernel and user space code and data segment descriptors
//! in the GDT:
//!
//! ```txt
//! /------ Kernel or User segment
//! | /---- Code or Data segment
//! | |  63                                                                        0
//! | |  bbbbbbbb G D L V llll P PL S tttt bbbbbbbbbbbbbbbbbbbbbbbb llllllllllllllll
//! v v
//! K CS 00000000   0 1        1 00 1 101  000000000000000000000000
//! K DS 00000000     0        1 00 1 001  000000000000000000000000
//! U CS 00000000   0 1        1 11 1 101  000000000000000000000000
//! U DS 00000000     0        1 11 1 001  000000000000000000000000
//! ```
//!
//! Any bits that are not specified above are "don't care" and should just be set to 0. See the SDM
//! chapters mentioned above for the meanings of these bits.

use alloc::boxed::Box;

use spin::Mutex;

use x86_64::{
    instructions::{segmentation::set_cs, tables::load_tss},
    structures::{
        gdt::{Descriptor, DescriptorFlags, GlobalDescriptorTable, SegmentSelector},
        idt::{InterruptDescriptorTable, InterruptStackFrame},
        tss::TaskStateSegment,
    },
    PrivilegeLevel, VirtAddr,
};

pub use self::pit::HZ as PIT_HZ;

mod pic;
mod pit;

/// Number of bytes of the IST stack frame.
const IST_FRAME_SIZE: usize = 4096;

/// The index in the TSS of the first Interrupt stack frame, used for fault handlers in emergency
/// (e.g. double faults).
pub const EMERGENCY_IST_FRAME_INDEX: u16 = 0;

/// The index in the TSS of the Interrupt stack frame where the interrupts, page faults, etc are
/// handled. Note that system calls are not handled on this stack frame. They use the main kernel
/// stacks in the scheduler.
pub const IRQ_IST_FRAME_INDEX: u16 = EMERGENCY_IST_FRAME_INDEX + 1;

// See notes at top of file regarding descriptor tables and segments.

/// Global Descriptor Table.
static GDT: Mutex<Option<GlobalDescriptorTable>> = Mutex::new(None);

/// The Task State Segment.
pub static TSS: Mutex<Option<TaskStateSegment>> = Mutex::new(None);

/// Interrupt Descriptor Table.
pub static IDT: Mutex<Option<InterruptDescriptorTable>> = Mutex::new(None);

#[derive(Debug)]
pub struct Selectors {
    pub kernel_cs: SegmentSelector,
    pub kernel_ds: SegmentSelector,
    pub user_cs: SegmentSelector,
    pub user_ds: SegmentSelector,
    pub tss: SegmentSelector,
}

pub static SELECTORS: Mutex<Selectors> = Mutex::new(Selectors {
    kernel_cs: SegmentSelector::new(0, PrivilegeLevel::Ring0),
    kernel_ds: SegmentSelector::new(0, PrivilegeLevel::Ring0),
    user_cs: SegmentSelector::new(0, PrivilegeLevel::Ring3),
    user_ds: SegmentSelector::new(0, PrivilegeLevel::Ring3),
    tss: SegmentSelector::new(0, PrivilegeLevel::Ring0),
});

/// Initialize interrupts (and exceptions).
pub fn init() {
    let mut tss = TaskStateSegment::new();
    let mut gdt = GlobalDescriptorTable::new();
    let mut idt = InterruptDescriptorTable::new();

    // Create TSS (but don't load yet).
    tss.interrupt_stack_table[EMERGENCY_IST_FRAME_INDEX as usize] = {
        // We create a struct to force the alignment to 16.
        #[repr(align(16))]
        struct Stack {
            _data: [u8; IST_FRAME_SIZE],
        }

        let stack = box Stack {
            _data: [0; IST_FRAME_SIZE],
        };
        let stack_start = VirtAddr::from_ptr(Box::into_raw(stack));
        let stack_end = stack_start + IST_FRAME_SIZE;
        printk!("double fault stack @ {:?}, {:?}\n", stack_start, stack_end);
        stack_end
    };

    tss.interrupt_stack_table[IRQ_IST_FRAME_INDEX as usize] = {
        // We create a struct to force the alignment to 16.
        #[repr(align(16))]
        struct Stack {
            _data: [u8; IST_FRAME_SIZE],
        }

        let stack = box Stack {
            _data: [0; IST_FRAME_SIZE],
        };
        let stack_start = VirtAddr::from_ptr(Box::into_raw(stack));
        let stack_end = stack_start + IST_FRAME_SIZE;
        printk!("irq stack @ {:?}, {:?}\n", stack_start, stack_end);
        stack_end
    };

    *TSS.lock() = Some(tss);

    let tss_ref = unsafe {
        // We know that the TSS will last forever...
        &*(TSS.lock().as_ref().unwrap() as *const TaskStateSegment)
    };

    // Initalize GDT
    let mut selectors = SELECTORS.lock();

    // NOTE: In the descriptors below, the names of the flags are aweful. I have added some
    // comments to explain their actual meanings.

    // NOTE: kernel CS must be the one before kernel DS
    selectors.kernel_cs = gdt.add_entry(Descriptor::UserSegment(
        (
            DescriptorFlags::LONG_MODE // 64-bit CS (should not be added to DS)
            | DescriptorFlags::PRESENT
            | DescriptorFlags::USER_SEGMENT // Not a system-segment (e.g. TSS)
            | DescriptorFlags::EXECUTABLE // CS rather than DS
            | DescriptorFlags::WRITABLE
            // For CS this bit actually means "readable", not execute-only (this comment applies to
            // the previous line, but rustfmt keeps moving it...)
        )
            .bits(),
    ));
    selectors.kernel_ds = gdt.add_entry(Descriptor::UserSegment(
        (
            DescriptorFlags::PRESENT
            | DescriptorFlags::USER_SEGMENT // Not a system-segment (e.g. TSS)
            | DescriptorFlags::WRITABLE
            // Makes the DS read/write
        )
            .bits(),
    ));

    // NOTE: user DS must be the one before user CS
    selectors.user_ds = gdt.add_entry(Descriptor::UserSegment(
        (
            DescriptorFlags::PRESENT
            | DescriptorFlags::DPL_RING_3 // This is a user-space segment
            | DescriptorFlags::USER_SEGMENT // Not a system-segment (e.g. TSS)
            | DescriptorFlags::WRITABLE
            // Makes the DS read/write
        )
            .bits(),
    ));
    selectors.user_cs = gdt.add_entry(Descriptor::UserSegment(
        (
            DescriptorFlags::LONG_MODE // 64-bit CS (should not be added to DS)
            | DescriptorFlags::PRESENT
            | DescriptorFlags::DPL_RING_3 // This is a user-space segment
            | DescriptorFlags::USER_SEGMENT // Not a system-segment (e.g. TSS)
            | DescriptorFlags::EXECUTABLE // CS rather than DS
            | DescriptorFlags::WRITABLE
            // For CS this bit actually means "readable", not execute-only
        )
            .bits(),
    ));

    selectors.tss = gdt.add_entry(Descriptor::tss_segment(tss_ref));

    *GDT.lock() = Some(gdt);

    // Load the GDT and TSS
    let gdt_ref = unsafe {
        // We know that the TSS will last forever...
        &*(GDT.lock().as_ref().unwrap() as *const GlobalDescriptorTable)
    };
    gdt_ref.load();
    unsafe {
        set_cs(selectors.kernel_cs);
        load_tss(selectors.tss);
    }

    // Initialize the IDT

    // Reset the IDT (this sets a few critical bits, too)
    //
    // We need to be careful not to overflow the stack, though...
    idt.reset();

    unsafe {
        pic::init_irqs(&mut idt);

        crate::memory::init_pf_handler(&mut idt);

        // Handle errors in weird states
        idt.general_protection_fault
            .set_handler_fn(handle_gpf)
            .set_stack_index(EMERGENCY_IST_FRAME_INDEX);

        idt.double_fault
            .set_handler_fn(handle_double_fault)
            .set_stack_index(EMERGENCY_IST_FRAME_INDEX);

        idt.non_maskable_interrupt
            .set_handler_fn(handle_nmi)
            .set_stack_index(EMERGENCY_IST_FRAME_INDEX);

        idt.invalid_opcode
            .set_handler_fn(handle_invalid_opcode)
            .set_stack_index(EMERGENCY_IST_FRAME_INDEX);
    }

    *IDT.lock() = Some(idt);

    let idt_ref = unsafe {
        // We know that the TSS will last forever...
        &*(IDT.lock().as_ref().unwrap() as *const InterruptDescriptorTable)
    };
    idt_ref.load();

    // Initialize the Programmable Interrupt Controler
    pic::init();

    // Initialize the Programmable Interrupt Timer
    pit::init();
}

/// Handle invalid opcode
extern "x86-interrupt" fn handle_invalid_opcode(esf: &mut InterruptStackFrame) {
    let opcode: u32 = unsafe { *esf.instruction_pointer.as_ptr() };

    panic!(
        "Invalid opcode
            CS:RIP: *({:#x}:{:#x}) = {:#x}
            flags: {:#b}",
        esf.code_segment,
        esf.instruction_pointer.as_u64(),
        opcode,
        esf.cpu_flags,
    );
}

/// Handle NMI
extern "x86-interrupt" fn handle_nmi(esf: &mut InterruptStackFrame) {
    let opcode: u32 = unsafe { *esf.instruction_pointer.as_ptr() };

    panic!(
        "Invalid opcode
            CS:RIP: *({:#x}:{:#x}) = {:#x}
            flags: {:#b}",
        esf.code_segment,
        esf.instruction_pointer.as_u64(),
        opcode,
        esf.cpu_flags,
    );
}

/// Handle a GPF fault
extern "x86-interrupt" fn handle_gpf(esf: &mut InterruptStackFrame, error: u64) {
    panic!(
        "General Protection Fault
            error: {:#x}
            CS:RIP: {:#x}:{:#x}
            flags: {:#b}",
        error,
        esf.code_segment,
        esf.instruction_pointer.as_u64(),
        esf.cpu_flags
    );
}

/// Handle a double fault
extern "x86-interrupt" fn handle_double_fault(esf: &mut InterruptStackFrame, error: u64) -> ! {
    panic!(
        "Double Fault
            error: {:#x}
            CS:RIP: {:#x}:{:#x}
            flags: {:#b}",
        error,
        esf.code_segment,
        esf.instruction_pointer.as_u64(),
        esf.cpu_flags
    );
}
