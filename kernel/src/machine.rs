//! A bunch of assembly-based utilities

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
        "pushq %rdx \n\t
         movw $1, %dx \n\t
         inb %dx, %al \n\t
         popq %rdx \n\t
         andq $$0xff,%rax \n\t"
         : "={rax}"(read)
         : "r"(port)
         : "rax"
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
        "pushq %rdx \n\t
        movw $0,%dx \n\t
        movb $1,%al \n\t
        outb %al,%dx \n\t
        popq %rdx \n\t"
        : /* No outputs */
        : "r"(port), "r"(val)
        : "rax"
        : "volatile"
    };
}
