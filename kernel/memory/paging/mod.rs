//! All things related to virtual memory.

mod e820;

use core::mem;

use buddy::BuddyAllocator;

use spin::Mutex;

use x86_64::{
    instructions::tlb,
    registers::model_specific::{Efer, EferFlags},
    structures::{
        idt::{ExceptionStackFrame, PageFaultErrorCode},
        paging::{
            FrameAllocator, Mapper, Page, PageSize, PageTable, PageTableFlags, PhysFrame,
            RecursivePageTable, Size4KiB,
        },
    },
    ux::u9,
    PhysAddr, VirtAddr,
};

use {KERNEL_HEAP_GUARD, KERNEL_HEAP_SIZE, KERNEL_HEAP_START};

use self::e820::E820Info;

/// The kernel's physical frame allocator. It returns frame numbers, not physical addresses.
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

// TODO: clean this up. Eventually, we will want this wrapper to be the only thing exposed for page
// frame allocation.
struct PhysBuddyAllocator<'a>(&'a mut BuddyAllocator<usize>);

impl<'a> FrameAllocator<Size4KiB> for PhysBuddyAllocator<'a> {
    fn alloc(&mut self) -> Option<PhysFrame<Size4KiB>> {
        self.0.alloc(1).map(|f| {
            PhysFrame::from_start_address(PhysAddr::new(f as u64 * Size4KiB::SIZE)).unwrap()
        })
    }
}

/// Initialize the physical and virtual memory allocators. Setup paging properly.
///
/// Currently, we have a single set of page tables that direct maps the first 2MiB of memory.
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

    // Add all available physical memory to the allocator based on info from the E820 BIOS call.
    // Don't add the first 2MiB since they are already in use.
    let mut total_mem = 0; // (in pages)
    for &(start, end) in e820.iter() {
        if end <= (KERNEL_HEAP_START + KERNEL_HEAP_SIZE) / (Size4KiB::SIZE as usize) {
            pmem_alloc.as_mut().unwrap().extend(start, end);
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
    let _ = PAGE_TABLES
        .lock()
        .as_mut()
        .unwrap()
        .map_to(
            new_pt_page,
            PhysFrame::from_start_address(PhysAddr::new(new_pt)).unwrap(),
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_CACHE,
            &mut PhysBuddyAllocator(pmem_alloc.as_mut().unwrap()),
        ).unwrap();

    // Update the PT with the new mappings for the first 2MiB.
    let page_table = new_pt_page.start_address().as_mut_ptr() as *mut PageTable;
    unsafe {
        (*page_table).zero();

        for i in 0u16..=(u9::MAX.into()) {
            if i == 0 {
                continue;
            } else if i as u64 * (1 << 12) == KERNEL_HEAP_GUARD {
                continue;
            }

            (*page_table)[i as usize].set_addr(
                PhysAddr::new(i as u64 * (1 << 12)),
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
    esf: &mut ExceptionStackFrame,
    // TODO: error code is not getting passed properly
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
    panic!(
        "Page fault at ip {:x}, addr {:x}",
        esf.instruction_pointer.as_u64(),
        cr2,
    );
}
