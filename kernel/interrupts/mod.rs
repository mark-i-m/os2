//! This module contains everything needed for interrupts

pub use self::idt::add_trap_handler;
pub use self::pic::pic_irq;
pub use self::pit::HZ as PIT_HZ;
pub use self::tss::init as tss_init;
pub use self::tss::rsp0;

mod pic;
mod pit;

mod idt;
mod tss;

/// Initialize interrupts.
pub fn init() {
    pic::init();
    pit::init();
}
