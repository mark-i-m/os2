//! This module contains some basic functionality that libstd would normally
//! otherwise provide. Most importantly, it defines `rust_begin_unwind` which is
//! used by `panic!`.

use core::panic::PanicInfo;

extern "C" {
    fn main() -> isize;
}

#[no_mangle]
pub unsafe fn _start() -> ! {
    let code = main();
    super::exit(code)
}

#[macro_export]
macro_rules! panic_handler {
    () => {
        #[panic_handler]
        fn panic(info: &core::panic::PanicInfo) -> ! {
            $crate::bare_bones::panic(info)
        }
    };
}

/// This function is used by `panic!` to display an error message.
pub fn panic(_pi: &PanicInfo) -> ! {
    // TODO: maybe implement an error message one day?
    super::exit(-100);
}
