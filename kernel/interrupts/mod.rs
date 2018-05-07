//! This module contains everything needed for interrupts

use core::sync::atomic::AtomicUsize;

pub use self::idt::add_trap_handler;
pub use self::tss::init as tss_init;
pub use self::tss::rsp0;

pub mod pic;

mod idt;
mod tss;

/// Counts interrupts. This can be used as a source of time.
static TICKS: AtomicUsize = AtomicUsize::new(0);

/// Initialize interrupts.
pub fn init() {
    pic::init();
}
