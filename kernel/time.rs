//! A module for dealing with system time and the passage of time.

use core::sync::atomic::{AtomicUsize, Ordering};

use interrupts::PIT_HZ;

/// Counts interrupts. This can be used as a source of time.
static TICKS: AtomicUsize = AtomicUsize::new(0);

/// Opaquely represents a system time
#[derive(Copy, Clone, Eq, Ord, PartialEq, PartialOrd)]
pub struct SysTime(usize);

impl SysTime {
    /// Get the system time without synchronizing. This has better performance but potentially misses a
    /// tick every once in a while.
    pub fn now() -> Self {
        // safe because we are only reading and we don't mind missing some synchronous op
        let time = unsafe {
            // we are guaranteed by the standard library that `AtomicUsize` has the same memory layout
            // as `usize`.
            *(&TICKS as *const AtomicUsize as *const usize)
        };

        SysTime(time)
    }

    /*
    /// Get the system time atomically. This has worse performance but will get the most recent
    /// timestamp.
    pub fn now_atomic() -> Self {
        SysTime(TICKS.load(Ordering::Relaxed))
    }
    */

    /// Get the time `secs` seconds after `self`.
    pub fn after(&self, secs: usize) -> Self {
        SysTime(self.0 + secs * PIT_HZ)
    }
}

/// Tick the clock atomically.
///
/// # NOTE
///
/// This should only be called from the timer interrupt handler.
pub fn tick() {
    TICKS.fetch_add(1, Ordering::Relaxed);
}
