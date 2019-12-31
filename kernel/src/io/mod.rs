//! All things I/O related.

pub mod kbd;

pub fn init() {
    kbd::init();
}
