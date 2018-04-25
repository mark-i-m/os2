//! The scheduler

use alloc::Vec;

use spin::Mutex;

use super::{Continuation, ProcessResult};

static SCHEDULER: Mutex<Option<Scheduler>> = Mutex::new(None);

struct Scheduler {
    todo: Vec<Continuation>,
}

/// Initialize the scheduler
pub fn init<F>(init: F)
where
    F: 'static + Send + FnMut() -> ProcessResult,
{
    let mut s = SCHEDULER.lock();

    // Create the scheduler
    *s = Some(Scheduler { todo: Vec::new() });

    // Add `init` to the scheduler queue
    SCHEDULER
        .lock()
        .as_mut()
        .unwrap()
        .todo
        .push(Continuation::new(init));
}

/// Run the scheduler to choose a task. Then switch to that task, discarding the current task as
/// complete. This should be called after all clean up has been completed. If no next task exists,
/// the idle continuation is used.
pub fn sched() -> ! {
    // TODO: get clean stack

    // TODO: set up clean stack

    // TODO: switch to clean stack

    // TODO: clean old stack

    // get the next task
    let next = if let Some(next) = SCHEDULER.lock().as_mut().unwrap().todo.pop() {
        next
    } else {
        Continuation::new(|| sched()) // idle
    };

    // run the task
    next.run();
}

/// Enqueue the given continuation in the scheduler.
pub fn enqueue(cont: Continuation) {
    SCHEDULER.lock().as_mut().unwrap().todo.push(cont);
}

/// Enqueue the idle continuation. This continuation just calls the scheduler to schedule something
/// else if possible.
pub fn idle() {
    enqueue(Continuation::new(|| sched()))
}
