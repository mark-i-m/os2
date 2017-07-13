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
 */

	.extern kernel_main
	.global kStart

kStart:
	mov $kernelStack,%esp
	call kernel_main
loop:
	jmp loop

    .align 8
	.skip 4096
kernelStack:
	.word 0
