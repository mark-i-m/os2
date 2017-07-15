/*
    This is the bootstrap code.

    It will be stored in sector#0 of the boot drive but the BIOS will load
    it at address 0x7C00

    We will compile the MBR at 0x7C00 then extract the appropriate 512 byte
    block and copy it to the boot drive

    The bootloader all fits within 0x7C00 to 0x8000 -- the first 2 sectors
*/

        .code16
    .section mbr,"xa"

    .global start
start:
    cli         # disable interrupts

    mov $0,%ax      # initialize data and stack segments
    mov %ax,%ds
    mov %ax,%ss

    movb %dl,0x7000     # save boot drive id

    # print an 'x' to let the world know we're here
    movb $'x',%al
    movb $1,%ah
    mov $0,%dx
    int $0x14

    # read the drive parameters
    movw $30,0x6000
    movb $0x48,%ah      # function code (read drive parameters)
    movb 0x7000,%dl     # drive index
    movw $0x6000,%si    # result buffer
    int $0x13       # call the BIOS

    movw 0x6010,%cx     # low 16 bits of sector count
                # this limits hdd size to 64k * 512 = 32MB

    # load the bootloader
    # read hdd into memory starting at bootloader_contd
    movw $0x6000,%si    # DAP pointer
    movb $16,(%si)      # size of buffer
    movb $0,1(%si)      # unused
    movw $1,2(%si)      # number of sectors
    movw $bootload_contd,4(%si) # buffer offset
    movw $0,6(%si)      # buffer segment
    movw $1,8(%si)      # starting sector number
    movw $0,10(%si)
    movw $0,12(%si)
    movw $0,14(%si)

    mov $0x42,%ah       # function code
    movb 0x7000,%dl     # drive index
    int $0x13       # read the sectors

    jnc bootload_contd # if there is an error print 'E' and hang
    movb $'E',%al
    movb $1,%ah
    mov $0,%dx
    int $0x14

hang:
    jmp hang

magic: # place magic MBR bytes 0x55AA
    .skip 510 - (magic - start)
    .byte 0x55
    .byte 0xaa

bootload_contd:
    # when the kernel is larger than 64KB, we need to load it in chunks of 64KB;
    # loop over the chunks loading them. 64KB => 64KB/512B = 128 sectors

    # read hdd into memory starting at loadKernelHere
    movw $0x6000,%si    # DAP pointer
    movb $16,(%si)      # size of buffer
    movb $0,1(%si)      # unused

    # number of sectors we need to copy
    movw %cx, %bx
    add $-2, %bx        # boot sectors

    # number of sectors copied
    xor %edi, %edi

load_loop:
    # copy up to 127 sectors at a time
    mov $0, %eax
    movw %bx, %ax
    sub  %di, %ax
    je cont
    cmp $127, %ax
    jle set_num_sec
    movw $127, %ax
set_num_sec:
    movw %ax,2(%si)     # number of sectors

    # where in memory to load to
    # this is a 4B field. The upper 2B are the segment, the lower 2B are offset.
    xor %ecx, %ecx
    imul $512, %edi, %ecx
    add $loadKernelHere, %ecx
    mov %cx,4(%si)      # buffer offset
    shr $16, %ecx
    shl $12, %ecx
    mov %cx,6(%si)      # buffer segment

    # which sector to start copying
    # most likely we won't need to copy more than 2^16 sectors
    # so the first 2B is sufficient
    mov $0, %ecx
    movw %di, %cx
    addw $2, %cx         # skip the first two sectors (since we already loaded them)
    movw %cx,8(%si)     # starting sector number
    movw $0,10(%si)
    movw $0,12(%si)
    movw $0,14(%si)

    # update counter
    add %ax, %di

    mov $0x42,%ah       # function code
    movb 0x7000,%dl     # drive index
    int $0x13       # read the sectors

    # on error print 'e' and hang
    jnc loop_jmp
    movb $'e',%al
    movb $1,%ah
    mov $0,%dx
    int $0x14
    jmp hang

loop_jmp:
    # print 'l'
    movb $'l',%al
    movb $1,%ah
    mov $0,%dx
    int $0x14

    jmp load_loop

cont:
    # detect available memory: ripped from OSDev
    # http://wiki.osdev.org/Detecting_Memory_(x86)#Getting_an_E820_Memory_Map
    mov $0, %di
    mov %di, %es
    mov $memory_map, %di
    xor %ebx, %ebx              # ebx must be 0 to start
    xor %bp, %bp                # bp is entry count
    mov $0x0534D4150, %edx      # "SMAP" into edx
    mov $0xE820, %eax
    movl $1, 20(%di)            # force a valid ACPI 3.X entry
    mov $24, %ecx               # ask for 24B
    int $0x15                   # BIOS call
    jc failed                   # carry set on first call means unsupported function
    mov $0x0534D4150, %edx      # Some BIOSes apparently trash this reg?
    cmp %eax, %edx              # on success, eax has "SMAP"
    jne failed
    test %ebx, %ebx             # ebx == 0 => list is 1 entry long (worthless)
    je failed
    jmp jmpin
e820lp:
    mov $0xE820, %eax           # eax, ecx were trashed
    mov $24, %ecx               # ask for 24B
    movl $1, 20(%di)            # force a valid ACPI 3.X entry
    int $0x15
    jc e820f                    # carry set => end of list already reached
    mov $0x0534D4150, %edx      # Some BIOSes apparently trash this reg?
jmpin:
    jcxz skipent                # skip 0 length entries
    cmp $20, %cl                # check for 24B ACPI 3.X response
    jbe notext
    testb $1, 20(%di)           # if so: is the "ignore this data" bit clear?
    je skipent
notext:
    mov 8(%di), %ecx            # get lower 32-bit of length of region
    or 12(%di), %ecx            # or with upper 32-bits to check for 0
    jz skipent                  # if length is 0, skip the region
    inc %bp                     # increment good entry count
    add $24, %di                # move to next entry storage spot
    cmp $20, %bp                # don't allow more than 20 regions
    je done
skipent:
    test %ebx, %ebx             # if ebx == 0, list is complete
    jne e820lp
e820f:
    mov %bp, memory_map_count   # store entry count
    clc                         # there is jc on end of list, so carry should be cleared
    jmp done
failed:
    movb $'m',%al               # if there is an error print 'm' and hang
    movb $1,%ah
    mov $0,%dx
    int $0x14
done:

    # load the gdt
    lgdt gdtDesc

    # load the idt
    lidt idtDesc

    # enable protected mode, still running in 16 bit segment with
        # original segments
    mov %cr0,%eax
    or $1,%eax
    mov %eax,%cr0


    # set the stack and data segments, can't touch memory until
    # we get into 32 bit mode
    #mov $16,%ax
    #mov %ax,%ds
    #mov %ax,%ss

    # do a far jump to the kernel entry point in 32 bit mode
    ljmp $8,$loadKernelHere


/* 32-bit "Protected" mode */

.align 512

	.extern kStart32

    .code32
loadKernelHere:
    mov $16,%ax
    mov %ax,%ss
    mov $16,%ax
    mov %ax,%ds
    jmp kStart32

#define GDT_COUNT 5

gdt:
    .long 0         # gdt[0] must be empty
    .long 0

    .long 0x0000ffff    # gdt[1] KERNEL CODE
    .long 0x00cf9800

    .long 0x0000ffff    # gdt[2] KERNEL DATA
    .long 0x00cf9200

    .long 0x0000ffff    # gdt[3] USER CODE
    .long 0x00cff800

    .long 0x0000ffff    # gdt[4] USER DATA
    .long 0x00cff200

gdtDesc:
    .word (GDT_COUNT * 8) - 1
    .long gdt


#define IDT_COUNT 256

    .global idt
idt:
    .skip IDT_COUNT * 8
idtDesc:
    .word (IDT_COUNT * 8) - 1
    .long idt


    .global memory_map
memory_map: # save some space for a memory map of no more than 20 entries
    # each entry is 24B, so 20 * 24B = 480B
    .skip 480

    .global memory_map_count
memory_map_count:
    .skip 4

###############################################################################
