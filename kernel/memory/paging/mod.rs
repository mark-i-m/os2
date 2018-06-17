//! All things related to virtual memory.

mod e820;

use core::mem;

use buddy::BuddyAllocator;

use spin::Mutex;

use x86_64::{
    structures::{
        idt::{ExceptionStackFrame, PageFaultErrorCode},
        paging::{PageSize, PageTable, PageTableFlags, RecursivePageTable, Size4KiB},
    },
    ux::u9,
    PhysAddr,
};

use self::e820::E820Info;

/// The kernel's physical frame allocator
static PHYS_MEM_ALLOC: Mutex<Option<BuddyAllocator<usize>>> = Mutex::new(None);

extern "C" {
    /// The root PML4 for the system.
    static mut page_map_l4: PageTable;
}

/// The page tables for the system. The page tables are recursive in the 511-th entry.
static PAGE_TABLES: Mutex<Option<RecursivePageTable>> = Mutex::new(None);

/// Recursive page table index.
const RECURSIVE_IDX: u9 = u9::MAX; // 511

// TODO: virtual address space allocator

/// Initialize the physical and virtual memory allocators. Setup paging properly.
///
/// Currently, we have a single set of page tables that direct maps the first 2MiB of memory with a
/// single huge page.
pub fn init() {
    ///////////////////////////////////////////////////////////////////////////
    // Setup the physical memory allocator with info from E820
    ///////////////////////////////////////////////////////////////////////////

    // Read E820 info
    let e820 = E820Info::read();

    // Decide how many tiers the allocator should have (rough estimate of log)
    let nbins = (8 * mem::size_of::<usize>()) as u8 - (e820.num_phys_pages().leading_zeros() as u8);

    // Create the allocator
    let mut pmem_alloc = PHYS_MEM_ALLOC.lock();
    *pmem_alloc = Some(BuddyAllocator::new(nbins));

    // Add all available physical memory to the allocator based on info from the E820 BIOS call
    let mut total_mem = 0; // (in pages)
    for &(start, end) in e820.iter() {
        pmem_alloc.as_mut().unwrap().extend(start, end);
        total_mem += end - start + 1;
    }

    printk!("\tphysical memory inited - {} frame\n", total_mem);

    ///////////////////////////////////////////////////////////////////////////
    // Setup recursive page tables.
    ///////////////////////////////////////////////////////////////////////////

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
    // TODO: Redo paging from the beginning of memory
    //  - direct map the beginning memory
    //  - Page 0 is null, so no mapping
    //  - The page before the kernel heap is null, so no mapping
    ///////////////////////////////////////////////////////////////////////////

    printk!("\tkernel page tables inited\n");

    ///////////////////////////////////////////////////////////////////////////
    // TODO: Extend the direct-mapped section after the kernel heap, and extend the kernel heap
    ///////////////////////////////////////////////////////////////////////////

    printk!("\theap extended\n");

    ///////////////////////////////////////////////////////////////////////////
    // TODO: set up the virtual address space allocator with 48-bits of virtual memory. Reserve the
    // kernel's space at the beginning of memory.
    ///////////////////////////////////////////////////////////////////////////

    printk!("\tvirtual address allocator inited\n");
}

/// Handle a page fault
pub extern "x86-interrupt" fn handle_page_fault(
    _esf: &mut ExceptionStackFrame,
    _error: PageFaultErrorCode,
) {
    // Read CR2 to get the page fault address
    let cr2: usize;
    unsafe {
        asm!{
            "movq %cr2, $0"
             : "=r"(cr2)
             : /* no input */
             : /* no clobbers */
             : "volatile"
        };
    }

    // TODO
    panic!("Page fault at {:x}", cr2);
}
