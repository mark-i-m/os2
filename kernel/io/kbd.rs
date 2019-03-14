//! For simplicity, we just buffer all keyboard input and let continuations waiting on keyboard
//! events dequeue from the front. Not efficient and kind of weird, but keyboard handling is a bit
//! boring IMHO, and just need something that works.

use alloc::collections::linked_list::LinkedList;

use spin::Mutex;

use x86_64::instructions::port::Port;

/// The difference between a capital and lowercase
const CAP: u8 = ('a' as u8) - ('A' as u8);

/// Keyboard command port
const KBD_CMD: Port<u8> = Port::new(0x64);

/// Keyboard data port
const KBD_DATA: Port<u8> = Port::new(0x60);

/// Buffered keyboard input.
static KBD_BUFFER: Mutex<Option<LinkedList<u8>>> = Mutex::new(None);

/// Is this character capital? Safe because we really don't care too much...
static mut SHIFT: bool = false;

/// The keyboard interrupt handler
///
/// Get a character from the keyboard and place it in the buffer.
pub unsafe fn handler() {
    if let Some(key) = read() {
        KBD_BUFFER.lock().as_mut().unwrap().push_back(key);
    }
}

/// Determine if this character is capital or not
unsafe fn ul(c: u8) -> u8 {
    if SHIFT {
        c - CAP
    } else {
        c
    }
}

/// Get a character from the keyboard. This should be called exactly once after a keyboard
/// interrupt and nowhere else.
unsafe fn read() -> Option<u8> {
    while KBD_CMD.read() & 1 == 0 {}
    let b: u8 = KBD_DATA.read();
    match b {
        0x02...0x0a => Some(b'0' + b - 1),
        0x0b => Some(b'0'),

        0x10 => Some(ul(b'q')),
        0x11 => Some(ul(b'w')),
        0x12 => Some(ul(b'e')),
        0x13 => Some(ul(b'r')),
        0x14 => Some(ul(b't')),
        0x15 => Some(ul(b'y')),
        0x16 => Some(ul(b'u')),
        0x17 => Some(ul(b'i')),
        0x18 => Some(ul(b'o')),
        0x19 => Some(ul(b'p')),
        0x1e => Some(ul(b'a')),
        0x1f => Some(ul(b's')),
        0x20 => Some(ul(b'd')),
        0x21 => Some(ul(b'f')),
        0x22 => Some(ul(b'g')),
        0x23 => Some(ul(b'h')),
        0x24 => Some(ul(b'j')),
        0x25 => Some(ul(b'k')),
        0x26 => Some(ul(b'l')),
        0x2c => Some(ul(b'z')),
        0x2d => Some(ul(b'x')),
        0x2e => Some(ul(b'c')),
        0x2f => Some(ul(b'v')),
        0x30 => Some(ul(b'b')),
        0x31 => Some(ul(b'n')),
        0x32 => Some(ul(b'm')),

        0x1c => Some(b'\n'),
        0x39 => Some(b' '),

        0x0e => Some(8),

        // Handle shift
        0x2a | 0x36 => {
            SHIFT = true;
            None
        }
        0xaa | 0xb6 => {
            SHIFT = false;
            None
        }

        // TODO: map other ascii characters
        _ => None,
    }
}

/// Initialize the buffer.
pub fn init() {
    *KBD_BUFFER.lock() = Some(LinkedList::new());
}

/// Return the first buffered character.
pub fn kbd_next() -> Option<u8> {
    KBD_BUFFER.lock().as_mut().unwrap().pop_front()
}
