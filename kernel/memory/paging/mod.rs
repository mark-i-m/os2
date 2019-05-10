//! All things related to virtual memory.
//!
//! Virtual memory is laid out as follows:
//! - Single address space (48-bits): everything lives in the same address space.
//! - The first page is null.
//! - Addresses Page 1 to 1MB: kernel text
//! - Page 1MB: heap guard page (to defend against heap errors spilling into the kernel text)
//! - Page 1MB+1page to 3MB: kernel heap (initially 1MB but extended to a size of 2MB)

mod e820;

use core::mem;

use buddy::BuddyAllocator;

use spin::Mutex;

use x86_64::{
    instructions::tlb,
    registers::model_specific::{Efer, EferFlags},
    structures::{
        idt::{InterruptStackFrame, PageFaultErrorCode},
        paging::{
            Mapper, Page, PageSize, PageTable, PageTableFlags, PhysFrame, RecursivePageTable,
            Size4KiB,
        },
    },
    ux::u9,
    PhysAddr, VirtAddr,
};

use {KERNEL_HEAP_GUARD, KERNEL_HEAP_SIZE, KERNEL_HEAP_START};

use self::e820::E820Info;

use crate::cap::{ResourceHandle, VirtualMemoryRegion};

/// The kernel's physical frame allocator. It returns frame numbers, not physical addresses.
static PHYS_MEM_ALLOC: Mutex<Option<phys::BuddyAllocator>> = Mutex::new(None);

/// The kernel's virtual memory allocator. It returns page numbers, not virtual addresses. This
/// allocator assigns parts of the 48-bit single address space when asked.
static VIRT_MEM_ALLOC: Mutex<Option<BuddyAllocator<usize>>> = Mutex::new(None);

extern "C" {
    /// The root PML4 for the system.
    static mut page_map_l4: PageTable;

    /// The address of the initial kernel stack. We want to unmap the end of the stack to protect
    /// against overflows that would overwrite important things like the IDT.
    static mut kernelStackTop: usize;
}

/// The page tables for the system. The page tables are recursive in the 511-th entry.
static PAGE_TABLES: Mutex<Option<RecursivePageTable>> = Mutex::new(None);

/// Recursive page table index.
const RECURSIVE_IDX: u9 = u9::MAX; // 511

/// The amount to extend the kernel heap by during init.
const KERNEL_HEAP_EXTEND: u64 = 1 << 20; // 1MB

/// The number of bits of virtual address space.
const ADDRESS_SPACE_WIDTH: u8 = 48;

/// The available virtual address ranges, excluding areas used by the kernel (`[start, end]`).
const VIRT_ADDR_AVAILABLE: &[(usize, usize)] = &[
    // Lower half - kernel
    (
        KERNEL_HEAP_START + KERNEL_HEAP_SIZE + KERNEL_HEAP_EXTEND as usize,
        (1 << (ADDRESS_SPACE_WIDTH - 1)) - 1,
    ),
    // Higher half
    //
    // NOTE: unfortunately, `buddy` is buggy and doesn't handle overflows correctly, so the upper
    // half address cause it to overflow. Thus, we discard half of the address space as a quick
    // fix. This is probably ok (though disappointing) because we still have ~128TiB of address
    // space.
    //(!((1 << (ADDRESS_SPACE_WIDTH - 1)) - 1), core::usize::MAX),
];

/// Physical memory allocator.
mod phys {
    use x86_64::{
        structures::paging::{FrameAllocator, PageSize, PhysFrame, Size4KiB},
        PhysAddr,
    };

    /// A thin wrapper around `BuddyAllocator` that just implements `FrameAllocator`.
    pub struct BuddyAllocator(buddy::BuddyAllocator<usize>);

    impl BuddyAllocator {
        pub fn new(nbins: u8) -> Self {
            BuddyAllocator(buddy::BuddyAllocator::new(nbins))
        }

        pub fn extend(&mut self, start: usize, end: usize) {
            self.0.extend(start, end);
        }

        pub fn alloc(&mut self, n: usize) -> Option<usize> {
            self.0.alloc(n)
        }

        pub fn free(&mut self, val: usize, n: usize) {
            self.0.free(val, n)
        }
    }

    impl FrameAllocator<Size4KiB> for BuddyAllocator {
        fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
            self.0.alloc(1).map(|f| {
                PhysFrame::from_start_address(PhysAddr::new(f as u64 * Size4KiB::SIZE)).unwrap()
            })
        }
    }
}

/// Initialize the physical and virtual memory allocators. Set up paging properly.
///
/// Before this, we have a single set of page tables that direct maps the first 2MiB of memory.
///
/// Afterwards, we have set up the phyiscal memory allocator, set up page tables for the single
/// address space, set up the null page, extend the kernel heap, set up the virtual memory
/// allocator for the single address space, and reserve the kernel virtual memory area.
pub fn init() {
    ///////////////////////////////////////////////////////////////////////////
    // Setup the physical memory allocator with info from E820
    ///////////////////////////////////////////////////////////////////////////

    // Read E820 info
    let e820 = E820Info::read();

    // Decide how many tiers the allocator should have (rough estimate of log)
    let nbins = (8 * mem::size_of::<usize>()) as u8 - (e820.num_phys_pages().leading_zeros() as u8);

    // Create the allocator.
    //
    // It's ok for us to just hold this lock because we don't expect to have any page faults during
    // this function's execution.
    let mut pmem_alloc = PHYS_MEM_ALLOC.lock();
    *pmem_alloc = Some(phys::BuddyAllocator::new(nbins));

    // Add all available physical memory to the allocator based on info from the E820 BIOS call.
    // Don't add the first 2MiB since they are already in use.
    let mut total_mem = 0; // (in pages)
    for &(start, end) in e820.iter() {
        let reserved = (KERNEL_HEAP_START + KERNEL_HEAP_SIZE + KERNEL_HEAP_EXTEND as usize)
            / (Size4KiB::SIZE as usize);
        if end <= reserved {
            // inside kernel reserved region
            continue;
        } else if start > reserved {
            // beyond reserved region
            pmem_alloc.as_mut().unwrap().extend(start, end);
            printk!("\tadded frames {:#X} - {:#X}\n", start, end);
        } else if start <= reserved {
            // chop off the reserved part
            pmem_alloc.as_mut().unwrap().extend(reserved, end);
            printk!("\tadded frames {:#X} - {:#X}\n", reserved, end);
        }
        total_mem += end - start + 1;
    }

    printk!("\tphysical memory inited - {} frames\n", total_mem);

    ///////////////////////////////////////////////////////////////////////////
    // Setup recursive page tables.
    ///////////////////////////////////////////////////////////////////////////

    // Enable the No-Execute bit on page tables.
    unsafe {
        Efer::update(|flags| *flags |= EferFlags::NO_EXECUTE_ENABLE);
    }

    // Add recursive mapping
    unsafe {
        page_map_l4[RECURSIVE_IDX].set_addr(
            PhysAddr::new((&page_map_l4) as *const PageTable as u64),
            PageTableFlags::PRESENT
                | PageTableFlags::WRITABLE
                | PageTableFlags::NO_CACHE
                | PageTableFlags::GLOBAL
                | PageTableFlags::NO_EXECUTE,
        );
    }

    *PAGE_TABLES.lock() =
        Some(unsafe { RecursivePageTable::new_unchecked(&mut page_map_l4, RECURSIVE_IDX) });

    printk!("\tpage tables inited\n");

    ///////////////////////////////////////////////////////////////////////////
    // Redo paging from the beginning of memory
    //  - direct map the beginning memory (no change)
    //  - Page 0 is null, so no mapping
    //  - The page before the kernel heap is null, so no mapping
    //
    // NOTE: QEMU doesn't report signed extended mappings with `info mem`, but they are indeed
    // happening.
    ///////////////////////////////////////////////////////////////////////////

    // We cannot just unmap the first page of memory because currently, we are executing on that
    // first 2MiB huge page. Instead, we need to create a page table that direct maps the first
    // 2MiB except for the 0-th page (null) and the page at 1MiB (guards kernel heap). Then we can
    // just change the page table entry in the PD to point to the new page table.

    // Allocate a page for the new page table entry.
    let new_pt = pmem_alloc.as_mut().unwrap().alloc(1).unwrap() as u64 * Size4KiB::SIZE;

    // We need to map the new PT somewhere so that we can update it.
    let new_pt_page =
        Page::from_page_table_indices(u9::new(0), u9::new(0), u9::new(0xA), u9::new(0));
    unsafe {
        let _ = PAGE_TABLES
            .lock()
            .as_mut()
            .unwrap()
            .map_to(
                new_pt_page,
                PhysFrame::from_start_address(PhysAddr::new(new_pt)).unwrap(),
                PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_CACHE,
                pmem_alloc.as_mut().unwrap(),
            )
            .unwrap();
    }

    // Update the PT with the new mappings for the first 2MiB except for the stack guard page.
    let page_table = new_pt_page.start_address().as_mut_ptr() as *mut PageTable;
    unsafe {
        (*page_table).zero();

        for i in 0u16..=(u9::MAX.into()) {
            if i == 0
                || u64::from(i) * (1 << 12) == KERNEL_HEAP_GUARD
                || u64::from(i) * (1 << 12) == (&kernelStackTop as *const usize as u64)
            {
                continue;
            }

            (*page_table)[i as usize].set_addr(
                PhysAddr::new(u64::from(i) * (1 << 12)),
                PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::GLOBAL,
            );
        }
    }

    // Replace the current 2MiB huge page entry in the PD with a mapping to the new PT.
    let page_dir =
        Page::from_page_table_indices(RECURSIVE_IDX, RECURSIVE_IDX, u9::new(0), u9::new(0))
            .start_address()
            .as_mut_ptr() as *mut PageTable;
    unsafe {
        (*page_dir)[0].set_addr(
            PhysAddr::new(new_pt),
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::GLOBAL,
        );
    }

    tlb::flush(VirtAddr::zero());
    unsafe {
        tlb::flush(VirtAddr::from_ptr(&kernelStackTop as *const usize));
    }

    // Unmap the new PT from where we were updating it.
    PAGE_TABLES
        .lock()
        .as_mut()
        .unwrap()
        .unmap(new_pt_page)
        .unwrap()
        .1
        .flush();

    printk!("\tkernel page tables inited\n");

    ///////////////////////////////////////////////////////////////////////////
    // Extend the direct-mapped section after the kernel heap, and extend the kernel heap
    ///////////////////////////////////////////////////////////////////////////

    let current_heap_end = (KERNEL_HEAP_START + KERNEL_HEAP_SIZE) as u64;
    {
        let mut pt = PAGE_TABLES.lock();
        for i in 0..(KERNEL_HEAP_EXTEND >> 12) {
            unsafe {
                <_ as Mapper<Size4KiB>>::identity_map(
                    pt.as_mut().unwrap(),
                    PhysFrame::from_start_address(PhysAddr::new(current_heap_end + (i << 12)))
                        .unwrap(),
                    PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::GLOBAL,
                    pmem_alloc.as_mut().unwrap(),
                )
                .unwrap()
                .flush();
            }
        }
    } // lock drops

    // Make sure that there are no concurrent allocations
    x86_64::instructions::interrupts::without_interrupts(|| unsafe {
        crate::ALLOCATOR.extend(current_heap_end as *mut u8, KERNEL_HEAP_EXTEND as usize)
    });

    printk!("\theap extended\n");

    ///////////////////////////////////////////////////////////////////////////
    // Set up the virtual address space allocator with 48-bits of virtual memory. Reserve the
    // kernel's space at the beginning of memory.
    ///////////////////////////////////////////////////////////////////////////

    let mut vmem_alloc = VIRT_MEM_ALLOC.lock();
    *vmem_alloc = Some(BuddyAllocator::new(ADDRESS_SPACE_WIDTH));

    for (start, end) in VIRT_ADDR_AVAILABLE {
        printk!("\tadd virt addrs [{:16X}, {:16X}]\n", start, end);
        vmem_alloc.as_mut().unwrap().extend(*start, *end);
    }

    printk!("\tvirtual address allocator inited\n");
}

/// Allocate a region of virtual memory (but not backed by physical memory). Specifically, allocate
/// the given number of pages. These allocations are basically parmanent.
///
/// No page table mappings are created. It is the user's responsibility to make sure the memory is
/// mapped before it is used.
///
/// Return a capability for the allocated region.
///
/// # Panics
///
/// If we exhaust the virtual address space.
pub fn valloc(npages: usize) -> ResourceHandle<VirtualMemoryRegion> {
    let mem = VIRT_MEM_ALLOC
        .lock()
        .as_mut()
        .unwrap()
        .alloc(npages)
        .expect("Out of virtual memory.");

    unsafe { crate::cap::register(VirtualMemoryRegion::new(mem, npages)) }
}

/// Handle a page fault
pub extern "x86-interrupt" fn handle_page_fault(
    esf: &mut InterruptStackFrame,
    // TODO: Fault frame and interrupt frame are not the same, but the stack should contain the
    // correct error code.
    _error: PageFaultErrorCode,
) {
    // Read CR2 to get the page fault address
    let cr2: usize;
    unsafe {
        asm! {
            "movq %cr2, $0"
             : "=r"(cr2)
             : /* no input */
             : /* no clobbers */
             : "volatile"
        };
    }

    // TODO
    panic!(
        "Page fault at ip {:x}, addr {:x}",
        esf.instruction_pointer.as_u64(),
        cr2,
    );
}
