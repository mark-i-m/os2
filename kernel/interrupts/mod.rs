//! This module contains everything needed for interrupts

pub use self::idt::add_trap_handler;
pub use self::tss::esp0;
pub use self::tss::init as tss_init;

pub mod pic;

mod idt;
mod tss;

/// Initialize interrupts.
pub fn init() {
    pic::init();
}
