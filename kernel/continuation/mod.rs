//! A module for defining continuations and events

use alloc::{boxed::Box, vec, vec::Vec};

use process::sched;
use time::SysTime;

/// Different kinds of events a continuation can wait for.
#[derive(Copy, Clone, Eq, Ord, PartialEq, PartialOrd)]
pub enum EventKind {
    /// Wait for "now" to occur. i.e. don't wait for anything.
    Now,

    // TODO: implement this
    /// Wait for keyboard input.
    Keyboard,

    /// Wait for the system "clock" to have a given reading.
    Until(SysTime),
}

/// The events corresponding to `EventKind`.
#[derive(Copy, Clone)]
pub enum Event {
    /// Wow! It's now!
    Now,

    /// The given character has been typed
    Keyboard(u8),

    /// A timer has expired
    Timer,
}

/// The possible results of running a continuation.
#[allow(dead_code)]
pub enum ContResult {
    /// The continuation suceeded and the next continuation and its precondition are given.
    Success(Vec<(EventKind, Continuation)>),

    /// The Continuation failed and here is the continuation to handle the error.
    Error(Continuation),

    /// The continuation suceeded and there is nothing left to be done.
    Done,
}

/// Represents a single Task in the system
pub struct Continuation {
    routine: Option<Box<FnMut(Event) -> ContResult + Send>>,
}

impl Continuation {
    /// Create a new `Task` struct whose entry point is the `main_fn` function
    pub fn new<F>(routine: F) -> Continuation
    where
        F: 'static + Send + FnMut(Event) -> ContResult,
    {
        Continuation {
            routine: Some(Box::new(routine)),
        }
    }

    /// Execute this continuation. Enqueue any resulting continuation in the scheduler. Then, cede
    /// control to the scheduler.
    ///
    /// # NOTE
    ///
    /// No funny stuff happens with the stack here, so this is safe to call from most places.
    /// However, the caller is responsible from making sure there is no stack overflow.
    ///
    /// Usually, this will be called just from the scheduler.
    pub fn run(mut self, event: Event) -> ! {
        // run this continuation, and enqueue the result
        match (self.routine.take().unwrap())(event) {
            // schedule the continuation
            ContResult::Success(cont) => sched::enqueue(cont),

            // schedule the error continuation with the error event
            ContResult::Error(cont) => sched::enqueue(vec![(EventKind::Now, cont)]),

            // if they are done, the continuation is the idle continuation
            ContResult::Done => sched::idle(),
        }

        // TODO: do any necessary cleanup here
        // NOTE: we cannot cleanup anything that we are currently using

        // Drop the current continuation
        drop(self);

        // cede control to the scheduler
        sched::sched()
    }
}
