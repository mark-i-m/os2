//! The scheduler

use alloc::Vec;

use spin::Mutex;

use super::{Continuation, ProcessResult};

static scheduler: Mutex<Option<Scheduler>> = Mutex::new(None);

struct Scheduler {
    todo: Vec<Continuation>,
}

/// Initialize the scheduler
pub fn init<F>(init: F)
where
    F: 'static + FnOnce() -> ProcessResult,
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
    // get the next task
    let next = if let Some(next) = scheduler.lock().unwrap().todo.pop() {
        next
    } else {
        Continuation::new(|| sched()) // idle
    };

    next.run();
}

/// Enqueue the given continuation in the scheduler.
pub fn enqueue(cont: Continuation) {
    scheduler.lock().unwrap().todo.push(cont);
}

/// Enqueue the idle continuation. This continuation just calls the scheduler to schedule something
/// else if possible.
pub fn idle() {
    enqueue(Continuation::new(|| sched()))
}
