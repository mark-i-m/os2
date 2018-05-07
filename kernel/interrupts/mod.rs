//! This module contains everything needed for interrupts

use core::sync::atomic::{AtomicUsize, Ordering};

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

/// Opaquely represents a system time
#[derive(Copy, Clone, Eq, Ord, PartialEq, PartialOrd)]
pub struct SysTime(usize);

/// Atomically get the system time.
pub fn get_time_atomic() -> SysTime {
    SysTime(TICKS.load(Ordering::Relaxed))
}

/// Get the system time without synchronizing. This has better performance but potentially misses a
/// tick every once in a while.
pub fn get_time() -> SysTime {
    // safe because we are only reading and we don't mind missing some synchronous op
    let time = unsafe {
        // we are guaranteed by the standard library that `AtomicUsize` has the same memory layout
        // as `usize`.
        *(&TICKS as *const AtomicUsize as *const usize)
    };

    SysTime(time)
}
