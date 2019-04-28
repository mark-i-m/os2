//! This module contains everything needed for interrupts

use x86_64::structures::{
    gdt::GlobalDescriptorTable,
    idt::{InterruptDescriptorTable, InterruptStackFrame},
    tss::TaskStateSegment,
};

pub use self::pit::HZ as PIT_HZ;

mod pic;
mod pit;

/// Imports that are defined at boot
#[allow(improper_ctypes)]
extern "C" {
    pub static mut idt64: InterruptDescriptorTable;
    pub static mut gdb64: GlobalDescriptorTable;
    pub static mut tss64: TaskStateSegment;
}

/// Initialize interrupts (and exceptions).
pub fn init() {
    // Initialize the Programmable Interrupt Controler
    pic::init();

    // Add a handler for GPF
    unsafe {
        idt64.double_fault.set_handler_fn(handle_double_fault);
        idt64.general_protection_fault.set_handler_fn(handle_gpf);
    }

    // Initialize the Programmable Interrupt Timer
    pit::init();
}

/// Handle a GPF fault
extern "x86-interrupt" fn handle_gpf(esf: &mut InterruptStackFrame, error: u64) {
    panic!(
        "General Protection Fault
            error: {:x}\n
            CS:RIP: {:x}:{:x}\n
            flags: {:b}",
        error,
        esf.code_segment,
        esf.instruction_pointer.as_u64(),
        esf.cpu_flags
    );
}

/// Handle a double fault
extern "x86-interrupt" fn handle_double_fault(esf: &mut InterruptStackFrame, _error: u64) {
    panic!("Double Fault\n{:#?}", esf);
}
