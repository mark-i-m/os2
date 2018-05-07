//! This module contains everything needed for interrupts

pub use self::idt::add_trap_handler;
pub use self::tss::init as tss_init;
pub use self::tss::rsp0;

pub mod pic;

mod idt;
mod tss;

/// Initialize interrupts.
pub fn init() {
    pic::init();
}

/// The number of timer interrupts per second.
pub fn pit_freq() -> usize {
    // TODO
    unimplemented!();
}
