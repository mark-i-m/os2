/*
 * This is the 32 bit kernel entry point
 *
 * In protected mode
 * At address 0x9000
 * Paging is disabled
 * ESP is not valid
 * Temporary GDT
 * BIOS IDT, TR
 *
 * We need to set up paging before we can go to long mode.
 */

	.code32

	.align 512

	.global kStart32
kStart32:
	// Set up initial page tables so we can jump to 64-bit long mode

    // pt -- we set all page table entries
    mov $0, %ecx // pte idx
    mov $page_table, %eax // page table

    pteloop:
        orw $0x1, (%eax, %ecx, 8) // present
        orw $0x2, (%eax, %ecx, 8) // writable
        mov %ecx, %edx
        shl $12, %edx // compute the page frame addr
        or %edx, (%eax, %ecx, 8) // set address
        add $1, %ecx
        cmp $512, %ecx
    jl pteloop

	// pd
	mov $page_dir, %eax
	orw $0x1, (%eax) // present
	orw $0x2, (%eax) // writable
	orw $page_table, (%eax) // address of page_table

	// pdp
	mov $page_dir_pointer, %eax
	orw $0x1, (%eax) // present
	orw $0x2, (%eax) // writeable
	orw $page_dir, (%eax) // address of page_dir

	// pml4
	mov $page_map_l4, %eax
	orw $0x1, (%eax) // present
	orw $0x2, (%eax) // writeable
	orw $page_dir_pointer, (%eax) // address of pdp

	// set cr3 to point to pml4
	mov $page_map_l4, %eax
	mov %eax, %cr3

	// set PAE and PGE
	mov %cr4, %eax
	or $0x20, %eax
	or $0x80, %eax
	mov %eax, %cr4

	// set the long mode bit in EFER MSR
	mov $0xC0000080, %ecx
	rdmsr
	or $0x100, %eax
	wrmsr

	// turn on paging
	mov %cr0, %eax
	or $0x80000000, %eax
	mov %eax, %cr0

	// set up a long mode IDT and GDT :/
    lidt idtDesc
	lgdt gdt64Desc

	// long jump to kernel entry point in 64-bit mode
	ljmp $8, $kStart64

hang32:
	jmp hang32 // Should never get here!

/*
Initial page tables so we can jump to 64-bit mode.

For now, we just set up an identity mapping for the kernel... we will later
change this. For now, we will map the first 2MiB. Until we turn on real paging,
we need to be extra careful about dereferencing null pointers...
*/
.align 4096
page_table:
	.zero 4096
page_dir:
	.zero 4096
page_dir_pointer:
	.zero 4096

    .global page_map_l4
page_map_l4:
	.zero 4096

/*
Long-mode IDT... all zeros for now, but will add some interrupts later.
*/
    .global idt64
idt64:
    .zero (256 * 16)

idtDesc:
    .word (256 * 16) - 1 // length of idt
    .long idt64

/*
Long-mode GDT: a single code segment. No data segment is need in 64-bit mode.
*/
    .align 16
	.global gdt64
gdt64:
	.quad 0 // zero entry
	.quad (1<<43) | (1<<44) | (1<<47) | (1<<53) // code segment

    .zero 7 * 8

gdt64Desc:
	.word (gdt64Desc - gdt64 - 1) // length of gdt
	.long gdt64

    .global kernelCodeSeg
kernelCodeSeg:
    .long 1 * 8

/*
Long-mode TSS.
*/
    .align 16
    .global tss64
tss64:
    .zero 8 * 12

###############################################################################

/* This is the 64-bit kernel entry point! We made it to long mode! */

	.align 1024
	.code64

	.extern kernel_main

kStart64:

	// set segment to null
	mov $0, %rax
	mov %rax, %ss
	mov %rax, %ds
	mov %rax, %es
	mov %rax, %fs
	mov %rax, %gs

	mov $kernelStack,%rsp
	call kernel_main
loop:
	jmp loop

/* This is the kernel stack (8KB) that we initially run on. To protect against
 * buffer overflows, we unmap a buffer page at the top of the stack as soon as
 * memory management is setup.
 */

    .align 4096 // page aligned
    .global kernelStackTop
kernelStackTop:
    .skip 4096  // buffer page
	.skip 9182  // the actual stack
kernelStack:
	.word 0

