//! Module for all things processes

mod sched;

// For use by `kernel_main` to start the first continuation
pub use self::sched::sched;

use alloc::boxed::Box;

pub enum ProcessResult<'c> {
    Success(Continuation<'c>),
    Error(Continuation<'c>),
    Done,
}

/// Represents a single Process in the system
pub struct Continuation<'c> {
    routine: Box<'c + FnOnce() -> ProcessResult<'c>>, // TODO: this is not a great design... we need to keep this continuation around so that the next continuation can access it's variables, but that means that we can never garbage collect anything...
}

impl<'ct> Continuation<'ct> {
    /// Create a new `Process` struct whose entry point is the `main_fn` function
    pub fn new<'c, F>(routine: F) -> Continuation<'c>
    where
        F: 'c + FnOnce() -> ProcessResult<'c>,
    {
        Continuation {
            routine: Box::new(routine),
        }
    }

    /// Execute this continuation. Enqueue any resulting continuation in the scheduler. Then, cede
    /// control to the scheduler.
    pub fn run(self) -> ! {
        // run this continuation, and enqueue the result
        match (self.routine)() {
            // if we have a continuation, run that
            ProcessResult::Success(cont) | ProcessResult::Error(cont) => sched::enqueue(cont),

            // if they are done, the continuation is the idle continuation
            ProcessResult::Done => sched::idle(),
        }

        // cede control to the scheduler
        sched::sched();
    }
}

/// Initialize the process/scheduling subsystem with the initial continuation.
pub fn init<'c, F>(init: F)
where
    F: 'static + FnOnce() -> ProcessResult<'c>,
{
    sched::init(init)
}
