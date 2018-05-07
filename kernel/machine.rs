//! A bunch of assembly-based utilities

use interrupts::pic_irq;

/// a wrapper around inb
pub unsafe fn inb(port: u16) -> u8 {
    /*
    push %edx
    mov 8(%esp),%dx
    inb %dx,%al
    pop %edx
    and $0xff,%eax
    ret
    */

    let read: u8;

    asm!{
        "pushq %rdx
         movw $1, %dx
         inb %dx, %al
         popq %rdx
         andq $$0xff,%rax "
         : "={rax}"(read)
         : "r"(port)
         : "rax", "rdx", "rsp"
         : "volatile"
    };

    read
}

/// a wrapper around outb
pub unsafe fn outb(port: u16, val: u8) {
    /*
    push %edx
    mov 8(%esp),%dx
    mov 12(%esp),%al
    outb %al,%dx
    pop %edx
    ret
    */

    asm!{
        "pushq %rdx
        movw $0,%dx
        movb $1,%al
        outb %al,%dx
        popq %rdx "
        : /* No outputs */
        : "r"(port), "r"(val)
        : "rax", "rdx", "rsp"
        : "volatile"
    };
}

pub unsafe fn ltr(tr: usize) {
    /*
    mov 4(%esp),%eax
    ltr %ax
    ret
    */

    asm!{
        "movq $0, %rax
        ltr %ax "
        : /* No outputs */
        : "r"(tr)
        : "rax"
        : "volatile"
    };
}

pub unsafe fn cli() {
    /*
    cli
    */

    asm!{
        "cli"
        : /* No outputs */
        : /* No inputs*/
        : /* No clobbers */
        : /* No options */
    };
}

pub unsafe fn sti() {
    /*
    sti
    */

    asm!{
        "sti"
        : /* No outputs */
        : /* No inputs*/
        : /* No clobbers */
        : /* No options */
    };
}

/// Initialize the PIT with the given divide
pub unsafe fn pit_do_init(divide: usize) {
    /*
	pushf			        # push IF
	cli			            # disable interrupts
	movb $0b00110100,%al	# 00 (channel 0)
				            # 110 (lobyte/hibyte)
				            # 100 (rate generator)
	outb %al,$0x43		    # write command
	movb 8(%esp),%al	    # divide
	outb %al,$0x40
	movb 9(%esp),%al
	outb %al,$0x40
	popf			        # pop IF
    */

    let first_byte = (divide & 0xFF) as u8;
    let second_byte = ((divide & 0xFF00) >> 8) as u8;

    asm!{
        "pushf
        cli
        movb $$0b00110100,%al
        outb %al,$$0x43
        movb $0,%al
        outb %al,$$0x40
        movb $1,%al
        outb %al,$$0x40
        popf"
    : /* No output */
    : "r"(first_byte), "r"(second_byte)
    : "rax"
    : "volatile"
    };
}

#[naked]
pub unsafe extern "C" fn irq0() {
    asm!{
        "push %rax
        movq $$0, %rax "
        : /* No outputs */
        : /* No inputs */
        : "rax", "rsp"
        : "volatile"
    };

    irq_common()
}

#[naked]
pub unsafe extern "C" fn irq1() {
    asm!{
        "push %rax
        movq $$1, %rax "
        : /* No outputs */
        : /* No inputs */
        : "rax", "rsp"
        : "volatile"
    };

    irq_common()
}

#[naked]
pub unsafe extern "C" fn irq2() {
    asm!{
        "push %rax
        movq $$2, %rax "
        : /* No outputs */
        : /* No inputs */
        : "rax", "rsp"
        : "volatile"
    };

    irq_common()
}

#[naked]
pub unsafe extern "C" fn irq3() {
    asm!{
        "push %rax
        movq $$3, %rax "
        : /* No outputs */
        : /* No inputs */
        : "rax", "rsp"
        : "volatile"
    };

    irq_common()
}

#[naked]
pub unsafe extern "C" fn irq4() {
    asm!{
        "push %rax
        movq $$4, %rax "
        : /* No outputs */
        : /* No inputs */
        : "rax", "rsp"
        : "volatile"
    };

    irq_common()
}

#[naked]
pub unsafe extern "C" fn irq5() {
    asm!{
        "push %rax
        movq $$5, %rax "
        : /* No outputs */
        : /* No inputs */
        : "rax", "rsp"
        : "volatile"
    };

    irq_common()
}

#[naked]
pub unsafe extern "C" fn irq6() {
    asm!{
        "push %rax
        movq $$6, %rax "
        : /* No outputs */
        : /* No inputs */
        : "rax", "rsp"
        : "volatile"
    };

    irq_common()
}

#[naked]
pub unsafe extern "C" fn irq7() {
    asm!{
        "push %rax
        movq $$7, %rax "
        : /* No outputs */
        : /* No inputs */
        : "rax", "rsp"
        : "volatile"
    };

    irq_common()
}

#[naked]
pub unsafe extern "C" fn irq8() {
    asm!{
        "push %rax
        movq $$8, %rax "
        : /* No outputs */
        : /* No inputs */
        : "rax", "rsp"
        : "volatile"
    };

    irq_common()
}

#[naked]
pub unsafe extern "C" fn irq9() {
    asm!{
        "push %rax
        movq $$9, %rax "
        : /* No outputs */
        : /* No inputs */
        : "rax", "rsp"
        : "volatile"
    };

    irq_common()
}

#[naked]
pub unsafe extern "C" fn irq10() {
    asm!{
        "push %rax
        movq $$10, %rax "
        : /* No outputs */
        : /* No inputs */
        : "rax", "rsp"
        : "volatile"
    };

    irq_common()
}

#[naked]
pub unsafe extern "C" fn irq11() {
    asm!{
        "push %rax
        movq $$11, %rax "
        : /* No outputs */
        : /* No inputs */
        : "rax", "rsp"
        : "volatile"
    };

    irq_common()
}

#[naked]
pub unsafe extern "C" fn irq12() {
    asm!{
        "push %rax
        movq $$12, %rax "
        : /* No outputs */
        : /* No inputs */
        : "rax", "rsp"
        : "volatile"
    };

    irq_common()
}

#[naked]
pub unsafe extern "C" fn irq13() {
    asm!{
        "push %rax
        movq $$13, %rax "
        : /* No outputs */
        : /* No inputs */
        : "rax", "rsp"
        : "volatile"
    };

    irq_common()
}

#[naked]
pub unsafe extern "C" fn irq14() {
    asm!{
        "push %rax
        movq $$14, %rax "
        : /* No outputs */
        : /* No inputs */
        : "rax", "rsp"
        : "volatile"
    };

    irq_common()
}

#[naked]
pub unsafe extern "C" fn irq15() {
    asm!{
        "push %rax
        movq $$15, %rax "
        : /* No outputs */
        : /* No inputs */
        : "rax", "rsp"
        : "volatile"
    };

    irq_common()
}

#[repr(C, packed)]
pub struct IrqContext {
    cr2: u64,
    r15: u64,
    r14: u64,
    r13: u64,
    r12: u64,
    r11: u64,
    r10: u64,
    r9: u64,
    r8: u64,
    rbp: u64,
    rdi: u64,
    rsi: u64,
    rdx: u64,
    rcx: u64,
    rbx: u64,
    rax: u64,
}

#[naked]
#[inline]
unsafe fn irq_common() -> ! {
    let irq: usize;
    let context_ptr: *mut IrqContext;

    // Save all registers
    asm!{
        "
        push %rbx
        push %rcx
        push %rdx
        push %rsi
        push %rdi
        push %rbp
        push %r8
        push %r9
        push %r10
        push %r11
        push %r12
        push %r13
        push %r14
        push %r15
        mov %cr2, %rbp
        push %rbp
        mov %rsp, $0
        mov %rax, $1
        "
        : "=r"(context_ptr), "=r"(irq)
        : /* No inputs */
        : "rbp"
        : "volatile"
    };

    // Handle interrupt
    pic_irq(irq, &mut *context_ptr);

    // Pop arguments and iretq
    asm!{
        "
        pop %rbp
        mov %rbp, %cr2
        pop %r15
        pop %r14
        pop %r13
        pop %r12
        pop %r11
        pop %r10
        pop %r9
        pop %r8
        pop %rbp
        pop %rdi
        pop %rsi
        pop %rdx
        pop %rcx
        pop %rbx
        pop %rax
        iretq
        "
        : /* No outputs */
        : /* No inputs */
        : "rax", "rbx", "rcx", "rdx", "rsi", "rdi", "rbp", "rsp", "r8",
          "r9", "r10", "r11", "r12", "r13", "r14", "r15"
        : "volatile"
    };

    panic!("Should never get here!");
}

#[naked]
pub unsafe extern "C" fn page_fault_handler() {
    use memory::handle_page_fault;

    let fault_addr: usize;

    asm!{
        "
        push %rax
        push %rbx
        push %rcx
        push %rdx
        push %rsi
        push %rdi
        push %rbp
        push %r8
        push %r9
        push %r10
        push %r11
        push %r12
        push %r13
        push %r14
        push %r15

        mov %cr2, $0   /* address */
        "
        : "=r"(fault_addr)
        : /* No inputs */
        : "rsp"
        : "volatile"
    };

    handle_page_fault(fault_addr);

    asm!{
        "
        pop %r15
        pop %r14
        pop %r13
        pop %r12
        pop %r11
        pop %r10
        pop %r9
        pop %r8
        pop %rbp
        pop %rdi
        pop %rsi
        pop %rdx
        pop %rcx
        pop %rbx
        pop %rax
        add $$8,%rsp   /* pop error */
        iret
        "
    };
}
