//! This module contains everything needed for interrupts

use spin::Mutex;

use x86_64::{
    instructions::{segmentation::set_cs, tables::load_tss},
    structures::{
        gdt::{Descriptor, DescriptorFlags, GlobalDescriptorTable},
        idt::{InterruptDescriptorTable, InterruptStackFrame},
        tss::TaskStateSegment,
    },
    VirtAddr,
};

pub use self::pit::HZ as PIT_HZ;

mod pic;
mod pit;

/// The index in the TSS of the first Interrupt stack frame.
const DOUBLE_FAULT_IST_INDEX: u16 = 0;

/// The number of bytes of the Interrupt Stack Frame.
const IST_FRAME_SIZE: usize = 4096;

/// The Task State Segment.
static TSS: Mutex<Option<TaskStateSegment>> = Mutex::new(None);

/// Interrupt Descriptor Table.
pub static IDT: Mutex<Option<InterruptDescriptorTable>> = Mutex::new(None);

/// Global Descriptor Table.
static GDT: Mutex<Option<GlobalDescriptorTable>> = Mutex::new(None);

/// Initialize interrupts (and exceptions).
pub fn init() {
    let mut tss = TaskStateSegment::new();
    let mut gdt = GlobalDescriptorTable::new();
    let mut idt = InterruptDescriptorTable::new();

    // Create TSS (but don't load yet).
    tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
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

    *TSS.lock() = Some(tss);

    let tss_ref = unsafe {
        // We know that the TSS will last forever...
        &*(TSS.lock().as_ref().unwrap() as *const TaskStateSegment)
    };

    // Initalize GDT
    let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
    let _user_code_seg = gdt.add_entry(Descriptor::UserSegment(
        (DescriptorFlags::USER_SEGMENT
            | DescriptorFlags::PRESENT
            | DescriptorFlags::EXECUTABLE
            | DescriptorFlags::LONG_MODE)
            .bits()
            | (3 << 45), // FIXME: the 3<<45 is the DPL (ring 3)
    ));
    let tss_selector = gdt.add_entry(Descriptor::tss_segment(tss_ref));

    *GDT.lock() = Some(gdt);

    // Load the GDT and TSS
    let gdt_ref = unsafe {
        // We know that the TSS will last forever...
        &*(GDT.lock().as_ref().unwrap() as *const GlobalDescriptorTable)
    };
    gdt_ref.load();
    unsafe {
        set_cs(code_selector);
        load_tss(tss_selector);
    }

    // Initialize the IDT
    pic::init_irqs(&mut idt);
    unsafe {
        idt.double_fault
            .set_handler_fn(handle_double_fault)
            .set_stack_index(DOUBLE_FAULT_IST_INDEX);
        idt.general_protection_fault.set_handler_fn(handle_gpf);
        crate::memory::init_pf_handler(&mut idt);
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
