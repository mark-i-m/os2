//! The scheduler

use alloc::Vec;

use spin::Mutex;

use super::{Continuation, ProcessResult};

static scheduler: Mutex<Option<Scheduler>> = Mutex::new(None);

struct Scheduler<'c> {
    todo: Vec<Continuation<'c>>,
}

/// Initialize the scheduler
pub fn init<'c, F>(init: F)
where
    F: 'static + FnOnce() -> ProcessResult<'c>,
{
    let s = scheduler.lock();

    // Create the scheduler
    *s = Some(Scheduler { todo: Vec::new() });

    // Add `init` to the scheduler queue
    scheduler.lock().unwrap().todo.push(Continuation::new(init));
}

/// Run the scheduler to choose a task. Then switch to that task, discarding the current task as
/// complete. This should be called after all clean up has been completed. If no next task exists,
/// the idle continuation is used.
pub fn sched() -> ! {
    // TODO
    unimplemented!();
}

/// Enqueue the given continuation in the scheduler.
pub fn enqueue<'c>(cont: Continuation<'c>) {
    // TODO
}

/// Enqueue the idle continuation. This continuation just calls the scheduler to schedule something
/// else if possible.
pub fn idle() {
    enqueue(Continuation::new(|| sched()))
}
