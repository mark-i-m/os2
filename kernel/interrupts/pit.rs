//! A module for the programmable interrupt timer

use x86_64::{instructions::port::Port,
             registers::flags::{flags, set_flags}};

/// Max frequency of the PIT
const MAX_HZ: usize = 1193182;

/// The frequency of the PIT
pub const HZ: usize = 1000;

/// The command port of the PIT
const PIT_CMD: Port<u8> = Port::new(0x43);

/// The data port of the PIT
const PIT_DATA: Port<u8> = Port::new(0x40);

/// Initialize the PIT to the given frequency
pub fn init() {
    let divide = MAX_HZ / HZ;

    if (divide & 0xffff) != divide {
        panic!("PIT init divide={} doesn't fit in 16 bits", divide);
    }

    printk!("pit inited - {} hz\n", HZ);

    unsafe {
        // save flags
        let saved_flags = flags();

        // disable interrupts
        super::disable();

        // command
        // 00 (channel 0)
        // 110 (lobyte/hibyte)
        // 100 (rate generator)
        let cmd = 0b00110100u8;

        // write commmand
        PIT_CMD.write(cmd);

        // Set the divide, one byte at a time
        let first_byte = (divide & 0xFF) as u8;
        let second_byte = ((divide & 0xFF00) >> 8) as u8;
        PIT_DATA.write(first_byte);
        PIT_DATA.write(second_byte);

        // restore flags
        set_flags(saved_flags);
    }
}
