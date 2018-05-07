//! The scheduler

use alloc::boxed::Box;

use core::{borrow::Borrow, mem};

use spin::Mutex;

use time::SysTime;

use continuation::{Continuation, Event, EventKind};

/// The size of a stack in words
const STACK_WORDS: usize = 1 << 12; // 16KB

/// The kernel task scheduler instance
static SCHEDULER: Mutex<Option<Scheduler>> = Mutex::new(None);

/// The kernel task scheduler
struct Scheduler {
    /// The next continuation to be run. Notice that since each task is single threaded, there can
    /// be at most one.
    next: Option<(EventKind, Continuation)>,

    // Because every core is single-threaded, we only need one stack. After a task executes, we can
    // just clean it up and reuse it. However, to make life a bit easier, we just allocate two
    // stacks: one for the current task and one for the next task.
    /// The stack of the current task
    current_stack: Stack,

    /// A clean stack for the next task
    clean_stack: Stack,
}

impl Scheduler {
    /// Get the next continuation to run along with the `Event` that it was waiting for. If no
    /// continuation exists or no continuation is ready, return None.
    pub fn next(&mut self) -> Option<(Event, Continuation)> {
        // No continuation
        if self.next.is_none() {
            return None;
        }

        // There is a continuation, but is it ready?
        let desired_eventkind = self.next.as_ref().unwrap().0;

        // Depending on the type of event, we do different things to determine if it is ready
        match desired_eventkind {
            // Not waiting? Great!
            EventKind::Now => Some((Event::Now, self.next.take().unwrap().1)),

            // Timer events? Is the requested time here?
            EventKind::Until(time) => if SysTime::now() >= time {
                // ready!
                Some((Event::Timer, self.next.take().unwrap().1))
            } else {
                None
            },

            // Waiting for kbd input?
            EventKind::Keyboard => unimplemented!(), // TODO
        }
    }

    /// Set the next continuation to run along with the event kind it is waiting for.
    ///
    /// # Panics
    ///
    /// If there is already a continuation scheduled.
    pub fn set_next(&mut self, eventkind: EventKind, cont: Continuation) {
        assert!(self.next.is_none());
        self.next = Some((eventkind, cont));
    }
}

/// An stack for execution of continuations
struct Stack(Box<[usize; STACK_WORDS]>);

impl Stack {
    /// Returns a new clean stack
    pub fn new() -> Self {
        Stack(box [0; STACK_WORDS]) // initialize in place
    }

    /// Returns the stack pointer to use for this stack
    pub fn first_rsp(&self) -> usize {
        /// Add a little padding in case a bug causes us to unwind too far.
        const PADDING: usize = 400; // words

        // The end of the array is the "bottom" (highest address) in the stack.
        let stack: &[usize; STACK_WORDS] = self.0.borrow();
        let bottom = stack.as_ptr();
        unsafe { bottom.add(STACK_WORDS - PADDING) as usize }
    }

    /// Clear the contents of this stack
    pub fn clear(&mut self) {
        for word in self.0.iter_mut() {
            *word = 0xDEADBEEF_DEADBEEF;
        }
    }
}

/// Initialize the scheduler
pub fn init(init: Continuation) {
    let mut s = SCHEDULER.lock();

    // Create the scheduler
    *s = Some(Scheduler {
        next: Some((EventKind::Now, init)),
        current_stack: Stack::new(),
        clean_stack: Stack::new(),
    });
}

/// Run the scheduler to choose a task. Then switch to that task, discarding the current task as
/// complete. This should be called after all clean up has been completed. If no next task exists,
/// the idle continuation is used.
pub fn sched() -> ! {
    let rsp = {
        // Get the scheduler
        let mut s = SCHEDULER.lock();
        let s = s.as_mut().unwrap();

        // Make the clean stack the current stack
        mem::swap(&mut s.current_stack, &mut s.clean_stack);

        // switch to clean stack.
        s.current_stack.first_rsp()

        // Lock dropped, borrows end, etc. when we call `part_2_thunk`
    };

    unsafe {
        sched_part_2_thunk(rsp);
    }
}

/// Part 2 of `sched`. This actually switches to the new stack. Then, it calls `part_3`, having
/// already switched to the new stack. This is done so that the compiler knows that no state should
/// be carried over, so we cannot lose any important stack variables (e.g. locks).
unsafe fn sched_part_2_thunk(rsp: usize) -> ! {
    asm!{
        "
        movq $0, %rsp
        movq $0, %rbp
        "
         : /* no outputs */
         : "r"(rsp)
         : "rbp", "rsp"
         : "volatile"
    };
    sched_part_3();
}

/// Now that we are running on the new stack, we can clean the old one. Then, switch to the next
/// task and start running it.
unsafe fn sched_part_3() -> ! {
    let (event, next) = {
        // Get the scheduler
        let mut s = SCHEDULER.lock();
        let s = s.as_mut().unwrap();

        // clean old stack
        s.clean_stack.clear();

        // get the next task
        if let Some(next) = s.next() {
            next
        } else {
            (Event::Now, make_idle_cont())
        }

        // Lock dropped, borrows end, etc. when we call `part_2_thunk`
    };

    // run the task
    next.run(event)
}

/// Enqueue the given continuation in the scheduler.
pub fn enqueue(eventkind: EventKind, cont: Continuation) {
    SCHEDULER.lock().as_mut().unwrap().set_next(eventkind, cont);
}

/// Returns the idle continuation.
pub fn make_idle_cont() -> Continuation {
    Continuation::new(|_| {
        sched();
    })
}

/// Enqueue the idle continuation. This continuation just calls the scheduler to schedule something
/// else if possible.
pub fn idle() {
    let cont = make_idle_cont();
    enqueue(EventKind::Now, cont);
}
