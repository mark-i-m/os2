//! Module for all things processes

use core::cell::Cell;
use core::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT, Ordering};

use spin::Mutex;

// Some global state is necessary...

/// A handle on the current process
static CURRENT_PROCESS: Mutex<Cell<Option<Process>>> = Mutex::new(Cell::new(None));

/// The next PID
static NEXT_PID: AtomicUsize = ATOMIC_USIZE_INIT;


/// Represents a single Process in the system
pub struct Process {
    pid: usize,

    main_fn: fn() -> Result<(), usize>,
}

impl Process {
    /// Create a new `Process` struct whose entry point is the `main_fn` function
    pub fn new(main_fn: fn() -> Result<(), usize>) -> Process {
        Process {
            pid: NEXT_PID.fetch_add(1, Ordering::SeqCst),
            main_fn,
        }
    }
}

/// The main_fn of the `init` process!
pub fn main_fn_init() -> Result<(), usize> {
    printk!("Init!");

    // TODO
    Ok(())
}
