//! Module for all things processes

mod sched;

// For use by `kernel_main` to start the first continuation
pub use self::sched::sched;

use alloc::boxed::Box;

#[allow(dead_code)]
pub enum ProcessResult {
    Success(Continuation),
    Error(Continuation),
    Done,
}

/// Represents a single Process in the system
pub struct Continuation {
    routine: Option<Box<FnMut() -> ProcessResult + Send>>,
}

impl Continuation {
    /// Create a new `Process` struct whose entry point is the `main_fn` function
    pub fn new<F>(routine: F) -> Continuation
    where
        F: 'static + Send + FnMut() -> ProcessResult,
    {
        Continuation {
            routine: Some(Box::new(routine)),
        }
    }

    /// Execute this continuation. Enqueue any resulting continuation in the scheduler. Then, cede
    /// control to the scheduler.
    pub fn run(mut self) -> ! {
        // run this continuation, and enqueue the result
        match (self.routine.take().unwrap())() {
            // if we have a continuation, run that
            ProcessResult::Success(cont) | ProcessResult::Error(cont) => sched::enqueue(cont),

            // if they are done, the continuation is the idle continuation
            ProcessResult::Done => sched::idle(),
        }

        // TODO: do any necessary cleanup here
        // NOTE: we cannot cleanup anything that we are currently using

        // Drop the current continuation
        drop(self);

        // cede control to the scheduler
        sched::sched();
    }
}

/// Initialize the process/scheduling subsystem with the initial continuation.
pub fn init<F>(init: F)
where
    F: 'static + Send + FnMut() -> ProcessResult,
{
    sched::init(init)
}
