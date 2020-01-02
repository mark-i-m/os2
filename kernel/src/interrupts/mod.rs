//! This module contains everything needed for interrupts

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
const EMERGENCY_IST_FRAME_INDEX: u16 = 0;

/// The index in the TSS of the Interrupt stack frame where the kernel stack pointer is saved.
pub const SAVED_KERNEL_RSP_IST_FRAME_INDEX: u16 = EMERGENCY_IST_FRAME_INDEX + 1;

/// Global Descriptor Table.
static GDT: Mutex<Option<GlobalDescriptorTable>> = Mutex::new(None);

/// The Task State Segment.
pub static TSS: Mutex<Option<TaskStateSegment>> = Mutex::new(None);

/// Interrupt Descriptor Table.
pub static IDT: Mutex<Option<InterruptDescriptorTable>> = Mutex::new(None);

#[derive(Debug)]
pub struct Selectors {
    pub kernel_cs: SegmentSelector,
    pub kernel_ss: SegmentSelector,
    pub user_cs: SegmentSelector,
    pub user_ss: SegmentSelector,
    pub tss: SegmentSelector,
}

pub static SELECTORS: Mutex<Selectors> = Mutex::new(Selectors {
    kernel_cs: SegmentSelector::new(0, PrivilegeLevel::Ring0),
    kernel_ss: SegmentSelector::new(0, PrivilegeLevel::Ring0),
    user_cs: SegmentSelector::new(0, PrivilegeLevel::Ring3),
    user_ss: SegmentSelector::new(0, PrivilegeLevel::Ring3),
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
        let stack_start = VirtAddr::from_ptr(&stack);
        let stack_end = stack_start + IST_FRAME_SIZE;
        printk!("double fault stack @ {:?}, {:?}\n", stack_start, stack_end);
        stack_end
    };

    // Initially we will make the saved stack the emergency frame. We shouldn't be taking many page
    // faults or interrupt early on anyway.
    tss.interrupt_stack_table[SAVED_KERNEL_RSP_IST_FRAME_INDEX as usize] =
        tss.interrupt_stack_table[EMERGENCY_IST_FRAME_INDEX as usize];

    *TSS.lock() = Some(tss);

    let tss_ref = unsafe {
        // We know that the TSS will last forever...
        &*(TSS.lock().as_ref().unwrap() as *const TaskStateSegment)
    };

    // Initalize GDT
    let mut selectors = SELECTORS.lock();

    // NOTE: kernel CS must be the one before kernel SS
    selectors.kernel_cs = gdt.add_entry(Descriptor::kernel_code_segment());
    selectors.kernel_ss = gdt.add_entry(Descriptor::kernel_code_segment()); // TODO

    // NOTE: user SS must be the one before user CS
    selectors.user_ss = gdt.add_entry(Descriptor::UserSegment(
        (DescriptorFlags::USER_SEGMENT
            | DescriptorFlags::PRESENT
            | DescriptorFlags::WRITABLE
            | DescriptorFlags::LONG_MODE
            | DescriptorFlags::DPL_RING_3)
            .bits(),
    ));
    selectors.user_cs = gdt.add_entry(Descriptor::UserSegment(
        (DescriptorFlags::USER_SEGMENT
            | DescriptorFlags::PRESENT
            | DescriptorFlags::EXECUTABLE
            | DescriptorFlags::LONG_MODE
            | DescriptorFlags::DPL_RING_3)
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
