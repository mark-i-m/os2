//! Module for all things processes

pub mod sched;

use continuation::Continuation;

/// Initialize the process/scheduling subsystem with the initial continuation.
pub fn init(init: Continuation) {
    sched::init(init)
}

/// Start the first task. This is only called by `kernel_main`!
pub fn start() -> ! {
    sched::sched()
}
