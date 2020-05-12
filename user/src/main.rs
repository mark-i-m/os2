#![no_std]
#![no_main]

rs::panic_handler!();

#[no_mangle]
pub unsafe extern "C" fn main() -> isize {
    0xDEADBEEF
}
