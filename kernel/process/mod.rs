//! Module for all things processes

pub mod sched;

use continuation::{ContResult, Event};

/// Initialize the process/scheduling subsystem with the initial continuation.
pub fn init<F>(init: F)
where
    F: 'static + Send + FnMut(Event) -> ContResult,
{
    sched::init(init)
}

/// Start the first task. This is only called by `kernel_main`!
pub fn start() -> ! {
    sched::sched()
}
