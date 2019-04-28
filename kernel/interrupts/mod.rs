//! This module contains everything needed for interrupts

use x86_64::{
    instructions::{segmentation::set_cs, tables::load_tss},
    structures::{
        gdt::{Descriptor, GlobalDescriptorTable},
        idt::{InterruptDescriptorTable, InterruptStackFrame},
        tss::TaskStateSegment,
    },
    VirtAddr,
};

pub use self::pit::HZ as PIT_HZ;

mod pic;
mod pit;

/// Imports that are defined at boot
#[allow(improper_ctypes)]
extern "C" {
    pub static mut idt64: InterruptDescriptorTable;
    pub static mut gdt64: GlobalDescriptorTable;
    pub static mut tss64: TaskStateSegment;
}

/// The index in the TSS of the first Interrupt stack frame.
const DOUBLE_FAULT_IST_INDEX: u16 = 0;

const IST_FRAME_SIZE: usize = 4096;

/// Initialize interrupts (and exceptions).
pub fn init() {
    // Initialize the TSS, update the GDT and IDT
    unsafe {
        tss64 = TaskStateSegment::new();
        tss64.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
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

        gdt64 = GlobalDescriptorTable::new();
        let code_seg = gdt64.add_entry(Descriptor::kernel_code_segment());
        let tss_seg = gdt64.add_entry(Descriptor::tss_segment(&tss64));

        gdt64.load();
        set_cs(code_seg);
        load_tss(tss_seg);
    }

    // Initialize the Programmable Interrupt Controler
    pic::init();

    // Add a few exception handlers.
    unsafe {
        idt64
            .double_fault
            .set_handler_fn(handle_double_fault)
            .set_stack_index(DOUBLE_FAULT_IST_INDEX);
        idt64.general_protection_fault.set_handler_fn(handle_gpf);
        crate::memory::init_pf_handler();
    }

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
extern "x86-interrupt" fn handle_double_fault(esf: &mut InterruptStackFrame, error: u64) {
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
