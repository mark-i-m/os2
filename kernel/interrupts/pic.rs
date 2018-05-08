//! A module for programmable interrupt controller

use x86_64::{instructions::port::Port, structures::idt::ExceptionStackFrame};

use time; // the most epic import statement ever written!

use super::idt64;

/// Command port for PIC1
const C1: Port<u8> = Port::new(0x20);

/// Data port for PIC1
const D1: Port<u8> = Port::new(0x21);

/// Command port for PIC2
const C2: Port<u8> = Port::new(0xA0);

/// Data port for PIC2
const D2: Port<u8> = Port::new(0xA1);

/// The first entries of the IDT are reserved for traps and exceptions. So the first
/// _interrupt_ is at vector 0x30.
const FIRST_IDT: u8 = 0x30;

/// Initialize the PIC, but leave interrupts disabled
pub fn init() {
    // Configure the PIC
    unsafe {
        // ICW1
        C1.write(0x11); /* init with ICW4, not single */
        C2.write(0x11); /* init with ICW4, not single */

        // ICW2
        D1.write(FIRST_IDT); /* IDT index for IRQ0 */
        D2.write(FIRST_IDT + 8); /* IDT index for IRQ8 */

        // ICW3
        D1.write(1 << 2); /* tells master that the slave is at IRQ2 */
        D2.write(2); /* tells salve that it's connected at IRQ2 */

        // ICW4
        D1.write(1); /* 8086 mode */
        D2.write(1); /* 8086 mode */

        // enable all
        D1.write(0);
        D2.write(0);
    };

    // Add some interrupt handlers
    let idt_mut = unsafe { &mut idt64 };

    // Reset the IDT (this sets a few critical bits, too)
    //
    // We need to be careful not to overflow the stack, though...
    idt_mut.reset();

    // Set up basic interrupts
    idt_mut[FIRST_IDT as usize + 0x0].set_handler_fn(irq_0);
    idt_mut[FIRST_IDT as usize + 0x1].set_handler_fn(irq_1);
    idt_mut[FIRST_IDT as usize + 0x2].set_handler_fn(irq_2);
    idt_mut[FIRST_IDT as usize + 0x3].set_handler_fn(irq_3);
    idt_mut[FIRST_IDT as usize + 0x4].set_handler_fn(irq_4);
    idt_mut[FIRST_IDT as usize + 0x5].set_handler_fn(irq_5);
    idt_mut[FIRST_IDT as usize + 0x6].set_handler_fn(irq_6);
    idt_mut[FIRST_IDT as usize + 0x7].set_handler_fn(irq_7);
    idt_mut[FIRST_IDT as usize + 0x8].set_handler_fn(irq_8);
    idt_mut[FIRST_IDT as usize + 0x9].set_handler_fn(irq_9);
    idt_mut[FIRST_IDT as usize + 0xa].set_handler_fn(irq_a);
    idt_mut[FIRST_IDT as usize + 0xb].set_handler_fn(irq_b);
    idt_mut[FIRST_IDT as usize + 0xc].set_handler_fn(irq_c);
    idt_mut[FIRST_IDT as usize + 0xd].set_handler_fn(irq_d);
    idt_mut[FIRST_IDT as usize + 0xe].set_handler_fn(irq_e);
    idt_mut[FIRST_IDT as usize + 0xf].set_handler_fn(irq_f);

    // Good for debugging
    idt_mut.breakpoint.set_handler_fn(breakpoint_handler);
}

/// End of interrupt: send the next irq, but interrupts still disabled
fn pic_eoi(irq: u8) {
    unsafe {
        if irq >= 8 {
            // let PIC2 know
            C2.write(0x20);
        }
        // we always let PIC1 know because PIC2 is routed though PIC1
        C1.write(0x20);
    }
}

/// IRQ handler
///
/// For more info on IRQ handlers: https://wiki.osdev.org/Interrupts
///
/// Note that this should _not_ be confused with _exceptions_. For more info on x86 exceptions, see
/// https://wiki.osdev.org/Exceptions
fn pic_irq(irq: usize, _: &mut ExceptionStackFrame) {
    // execute handler
    match irq {
        // PIT interrupts
        0 => {
            // tick the clock
            time::tick();
        }

        // Keyboard interrupts
        1 => {
            unimplemented!();
        }

        // Processor and FPU interrupts
        13 => {}

        // IDE interrupts
        15 => {}

        // Other (unknown) interrupts
        _ => {
            unsafe {
                super::disable();
            }
            panic!("unknown interrupt {}\n", irq)
        }
    }

    // the PIC can deliver the next interrupt, but interrupts are still disabled
    pic_eoi(irq as u8);
}

////////////////////////////////////////////////////////////////////////////////
// The interrupt handlers
//
// These are called by the hardware. They simply call `pic_irq`, which does the
// hard work for them.
////////////////////////////////////////////////////////////////////////////////

extern "x86-interrupt" fn irq_0(esf: &mut ExceptionStackFrame) {
    pic_irq(0, esf);
}

extern "x86-interrupt" fn irq_1(esf: &mut ExceptionStackFrame) {
    pic_irq(1, esf);
}

extern "x86-interrupt" fn irq_2(esf: &mut ExceptionStackFrame) {
    pic_irq(2, esf);
}

extern "x86-interrupt" fn irq_3(esf: &mut ExceptionStackFrame) {
    pic_irq(3, esf);
}

extern "x86-interrupt" fn irq_4(esf: &mut ExceptionStackFrame) {
    pic_irq(4, esf);
}

extern "x86-interrupt" fn irq_5(esf: &mut ExceptionStackFrame) {
    pic_irq(5, esf);
}

extern "x86-interrupt" fn irq_6(esf: &mut ExceptionStackFrame) {
    pic_irq(6, esf);
}

extern "x86-interrupt" fn irq_7(esf: &mut ExceptionStackFrame) {
    pic_irq(7, esf);
}

extern "x86-interrupt" fn irq_8(esf: &mut ExceptionStackFrame) {
    pic_irq(8, esf);
}

extern "x86-interrupt" fn irq_9(esf: &mut ExceptionStackFrame) {
    pic_irq(9, esf);
}

extern "x86-interrupt" fn irq_a(esf: &mut ExceptionStackFrame) {
    pic_irq(0xa, esf);
}

extern "x86-interrupt" fn irq_b(esf: &mut ExceptionStackFrame) {
    pic_irq(0xb, esf);
}

extern "x86-interrupt" fn irq_c(esf: &mut ExceptionStackFrame) {
    pic_irq(0xc, esf);
}

extern "x86-interrupt" fn irq_d(esf: &mut ExceptionStackFrame) {
    pic_irq(0xd, esf);
}

extern "x86-interrupt" fn irq_e(esf: &mut ExceptionStackFrame) {
    pic_irq(0xe, esf);
}

extern "x86-interrupt" fn irq_f(esf: &mut ExceptionStackFrame) {
    pic_irq(0xf, esf);
}

/// Handle a breakpoint exception
extern "x86-interrupt" fn breakpoint_handler(stack_frame: &mut ExceptionStackFrame) {
    panic!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}
