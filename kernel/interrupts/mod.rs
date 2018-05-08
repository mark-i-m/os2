//! This module contains everything needed for interrupts

use x86_64::structures::idt::{ExceptionStackFrame, Idt};

pub use self::pit::HZ as PIT_HZ;
pub use self::tss::init as tss_init;
pub use self::tss::rsp0;

mod pic;
mod pit;

mod tss;

/// Imports that are defined at boot
#[allow(improper_ctypes)]
extern "C" {
    pub static mut idt64: Idt;
}

/// Initialize interrupts (and exceptions).
pub fn init() {
    // Initialize the Programmable Interrupt Controler
    pic::init();

    // Add a handler for GPF
    unsafe {
        idt64.general_protection_fault.set_handler_fn(handle_gpf);
    }

    // Initialize the Programmable Interrupt Timer
    pit::init();
}

/// Handle a GPF fault
extern "x86-interrupt" fn handle_gpf(esf: &mut ExceptionStackFrame, error: u64) {
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

/// Disable interrupts
pub unsafe fn disable() {
    /*
    cli
    */

    asm!{
        "cli"
        : /* No outputs */
        : /* No inputs*/
        : /* No clobbers */
        : /* No options */
    };
}

/// Enable interrupts
pub unsafe fn enable() {
    /*
    sti
    */

    asm!{
        "sti"
        : /* No outputs */
        : /* No inputs*/
        : /* No clobbers */
        : /* No options */
    };
}
