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

	// pd
	mov $page_dir, %eax
	orw $0x1, (%eax) // present
	orw $0x2, (%eax) // writable
	orw $0x80, (%eax) // 2MiB page
	// entry's address is already 0

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

	// set PAE
	mov %cr4, %eax
	or $0x20, %eax
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
change this.  We will use a single 2MiB page to identity map the beginning
memory. Until we turn on real paging, we need to be extra careful about
dereferencing null pointers...
*/
.align 4096
#page_table:  // 2MiB page, so we don't need this :)
#	.zero 4096
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
	.global gdt64
gdt64:
	.quad 0 // zero entry
	.quad (1<<43) | (1<<44) | (1<<47) | (1<<53) // code segment

    .global tssDescriptor
tssDescriptor:
    .quad 0         # gdt64[2] TSS part1
    .quad 0         # gdt64[3] TSS part2

gdt64Desc:
	.word (gdt64Desc - gdt64 - 1) // length of gdt
	.long gdt64

    .global kernelCodeSeg
kernelCodeSeg:
    .long 1 * 8

    .global tssDS
tssDS:
    .long 2 * 8

###############################################################################

/* This is the 64-bit kernel entry point! We made it to long mode! */

	.align 1024
	.code64

	.extern kernel_main

kStart64:

	// set segment to null
	mov $0, %eax
	mov %eax, %ss
	mov %eax, %ds
	mov %eax, %es
	mov %eax, %fs
	mov %eax, %gs

	mov $kernelStack,%esp
	call kernel_main
loop:
	jmp loop

    .align 8
	.skip 9182
kernelStack:
	.word 0

