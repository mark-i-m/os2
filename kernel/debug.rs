//! This module allows the user to print to QEMU's serial console.
//!
//! I borrowed it from krzysz00/rust-kernel/kernel/console.rs

use core::fmt::{Error, Write};

use machine::{inb, outb};

/// Port to output to serial console
const PORT: u16 = 0x3F8;

/// A struct to write data to the console port
pub struct Debug;

impl Debug {
    /// Wait for the port, then write the given array of bytes
    pub fn write_bytes(&self, bytes: &[u8]) {
        for b in bytes {
            unsafe {
                while inb(PORT + 5) & 0x20 == 0 {}
                outb(PORT, *b);
            }
        }
    }
}

/// Implement `Write` so that we can use format strings
impl Write for Debug {
    /// Take a string slice and write to the serial console
    #[inline]
    fn write_str(&mut self, data: &str) -> Result<(), Error> {
        self.write_bytes(data.as_bytes());
        Result::Ok(())
    }
}

/// A macro for printing using format strings to the console
/// when interrupts are enabled
#[macro_export]
macro_rules! printk {
    ($($arg:tt)*) => ({
        use ::core::fmt::Write;
        let _ = write!($crate::debug::Debug, $($arg)*);
    })
}
