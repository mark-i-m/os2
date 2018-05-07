//! This module contains everything needed for interrupts

pub use self::idt::add_trap_handler;
pub use self::pic::pic_irq;
pub use self::pit::HZ as PIT_HZ;
pub use self::tss::init as tss_init;
pub use self::tss::rsp0;

use machine::gpf_handler;

mod pic;
mod pit;

mod idt;
mod tss;

/// Initialize interrupts (and exceptions).
pub fn init() {
    pic::init();
    pit::init();
    gfp_init();
}

/// Initialize the General Protection Fault handler.
fn gfp_init() {
    add_trap_handler(13, gpf_handler, 0);
}

/// Handle a GPF fault
pub fn handle_gpf(error: usize, cs: usize, rip: usize, flags: usize) {
    panic!(
        "General Protection Fault
            error: {:x}\n
            CS:RIP: {:x}:{:x}\n
            flags: {:b}",
        error, cs, rip, flags
    );
}
