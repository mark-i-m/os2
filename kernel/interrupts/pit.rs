//! A module for the programmable interrupt timer

use machine::pit_do_init;

/// Max frequency of the PIT
const MAX_HZ: usize = 1193182;

/// The frequency of the PIT
pub const HZ: usize = 1000;

/// Initialize the PIT to the given frequency
pub fn init() {
    let d = MAX_HZ / HZ;

    if (d & 0xffff) != d {
        panic!("PIT init d={} doesn't fit in 16 bits", d);
    }

    printk!("pit inited - {} hz\n", HZ);

    unsafe {
        pit_do_init(d);
    }
}
