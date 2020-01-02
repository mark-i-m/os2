//! All things related to virtual memory.
//!
//! Virtual memory is laid out as follows:
//! - Single address space (48-bits): everything lives in the same address space.
//! - Page 0 is null.
//! - Page 0xB8000 is the VGA buffer
//! - Pages [2MB, 32MB-1): reserved for kernel text (with kernel loaded at start of this region)
//! - Page 32MB-1: heap guard page (to defend against heap errors spilling into the kernel text)
//! - Page [32MB, 36MB): kernel heap
//!
//! Physical memory is arranged by the bootloader, which first runs E820 to get a memory map. The
//! `BootInfo` struct contains the current state of memory, including memory already allocated by
//! the bootload for page tables, kernel text, etc...

use alloc::collections::BTreeMap;

use bootloader::BootInfo;

use buddy::BuddyAllocator;

use spin::Mutex;

use x86_64::{
    registers::model_specific::{Efer, EferFlags},
    structures::{
        idt::{InterruptStackFrame, PageFaultErrorCode},
        paging::{
            FrameAllocator, Mapper, Page, PageSize, PageTable, PageTableFlags, PageTableIndex,
            PhysFrame, RecursivePageTable, Size2MiB, Size4KiB, UnusedPhysFrame,
        },
    },
    PhysAddr, VirtAddr,
};

use crate::cap::{Capability, ResourceHandle, UnregisteredResourceHandle};

/// The kernel's physical frame allocator. It returns frame numbers, not physical addresses.
static PHYS_MEM_ALLOC: Mutex<Option<phys::BuddyAllocator>> = Mutex::new(None);

/// The kernel's virtual memory allocator. It returns page numbers, not virtual addresses. This
/// allocator assigns parts of the 48-bit single address space when asked.
static VIRT_MEM_ALLOC: Mutex<Option<BuddyAllocator<usize>>> = Mutex::new(None);

/// The page tables for the system.
static PAGE_TABLES: Mutex<Option<RecursivePageTable>> = Mutex::new(None);
///
/// The set of allowed pages. These pages are allowed to take a page fault.
///
/// Current format: (start, (len, flags))
///
/// TODO: We should check permissions/capabilities for the fault first.
static ALLOWED: Mutex<Option<BTreeMap<u64, (u64, PageTableFlags)>>> = Mutex::new(None);

/// Address of guard page of the kernel heap (page before the first page of the heap).
pub const KERNEL_HEAP_GUARD: u64 = (32 << 20) - (1 << 12);

/// Address of first page of the kernel heap. Needs to be 2MiB-aligned because we map it using 2MiB
/// pages.
pub const KERNEL_HEAP_START: u64 = KERNEL_HEAP_GUARD + (1 << 12);

/// The size of the kernel heap (bytes). Needs to be a multiple of 2MiB because we map it using
/// 2MiB pages.
pub const KERNEL_HEAP_SIZE: u64 = 4 << 20; // 4MiB

/// The number of bits of virtual address space.
const ADDRESS_SPACE_WIDTH: u8 = 48;

/// The available virtual address ranges, excluding areas used by the kernel (`[start, end]`).
const VIRT_ADDR_AVAILABLE: &[(usize, usize)] = &[
    // Lower half - kernel
    (
        (KERNEL_HEAP_START + KERNEL_HEAP_SIZE) as usize,
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
    use core::mem;

    use bootloader::{bootinfo::MemoryRegionType, BootInfo};

    use x86_64::{
        structures::paging::{FrameAllocator, PageSize, PhysFrame, Size4KiB, UnusedPhysFrame},
        PhysAddr,
    };

    use super::{KERNEL_HEAP_SIZE, KERNEL_HEAP_START, PHYS_MEM_ALLOC};

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

    unsafe impl FrameAllocator<Size4KiB> for BuddyAllocator {
        fn allocate_frame(&mut self) -> Option<UnusedPhysFrame<Size4KiB>> {
            self.0.alloc(1).map(|f| {
                let frame = PhysFrame::from_start_address(PhysAddr::new(f as u64 * Size4KiB::SIZE))
                    .unwrap();
                unsafe { UnusedPhysFrame::new(frame) }
            })
        }
    }

    /// Initialize the physical memory allocator.
    pub fn init(boot_info: &'static BootInfo) {
        let last_page = boot_info
            .memory_map
            .iter()
            .map(|&region| region.range.end_frame_number)
            .max()
            .expect("No physical pages in memory map");

        // Decide how many tiers the allocator should have (rough estimate of log)
        let nbins = (8 * mem::size_of::<usize>()) as u8 - (last_page.leading_zeros() as u8);

        // Create the allocator.
        //
        // It's ok for us to just hold this lock because we don't expect to have any page faults during
        // this function's execution.
        let mut pmem_alloc = PHYS_MEM_ALLOC.lock();
        *pmem_alloc = Some(BuddyAllocator::new(nbins));

        // Add all available physical memory to the allocator based on info from the E820 BIOS call.
        let mut total_mem = 0; // (in pages)
        for &region in boot_info
            .memory_map
            .iter()
            .filter(|&region| match region.region_type {
                MemoryRegionType::Usable => true,

                // Includes unusable regions and regions already in use by the kernel text, initial
                // page tables, etc.
                _ => false,
            })
        {
            const RESERVED: usize =
                ((KERNEL_HEAP_START + KERNEL_HEAP_SIZE) / Size4KiB::SIZE) as usize;

            let start = region.range.start_frame_number as usize;
            let end = region.range.end_frame_number as usize;

            if end <= RESERVED {
                // inside kernel reserved region
                continue;
            } else if start > RESERVED {
                // beyond reserved region
                pmem_alloc.as_mut().unwrap().extend(start, end);
                printk!("\tadded frames {:#X} - {:#X}\n", start, end);
            } else if start <= RESERVED {
                // chop off the reserved part
                pmem_alloc.as_mut().unwrap().extend(RESERVED, end);
                printk!("\tadded frames {:#X} - {:#X}\n", RESERVED, end);
            }
            total_mem += end - start + 1;
        }

        printk!("\tphysical memory inited - {} frames\n", total_mem);
    }
}

pub fn early_init(boot_info: &'static BootInfo) {
    phys::init(boot_info);
    init_early_paging(boot_info);
}

/// Initialize just enough paging to bootstrap the remaining initialization.
fn init_early_paging(boot_info: &'static BootInfo) {
    // Enable the No-Execute bit on page tables.
    unsafe {
        Efer::update(|flags| *flags |= EferFlags::NO_EXECUTE_ENABLE);
    }

    // Recursive tables are setup by bootloader.
    let boot_pt = unsafe { &mut *(boot_info.recursive_page_table_addr as *mut PageTable) };
    let mut page_tables = PAGE_TABLES.lock();
    *page_tables = Some(unsafe {
        RecursivePageTable::new(boot_pt).expect("Recursive page table init failed.")
    });

    let mut pmem_alloc = PHYS_MEM_ALLOC.lock();

    // Map some space for the kernel heap, which will be inited shortly.
    let heap_start: Page<Size2MiB> = Page::containing_address(VirtAddr::new(KERNEL_HEAP_START));
    let heap_end: Page<Size2MiB> =
        Page::containing_address(VirtAddr::new(KERNEL_HEAP_START + KERNEL_HEAP_SIZE));
    for page in Page::range(heap_start, heap_end) {
        let frame = PhysFrame::from_start_address(PhysAddr::new(
            (pmem_alloc
                .as_mut()
                .unwrap()
                .alloc((Size2MiB::SIZE >> 12) as usize)
                .expect("Unable to allocate physical memory for early init")
                << 12) as u64,
        ))
        .expect("expected aligned page");

        unsafe {
            page_tables
                .as_mut()
                .unwrap()
                .map_to(
                    page,
                    unsafe { UnusedPhysFrame::new(frame) },
                    PageTableFlags::PRESENT
                        | PageTableFlags::WRITABLE
                        | PageTableFlags::GLOBAL
                        | PageTableFlags::NO_EXECUTE,
                    pmem_alloc.as_mut().unwrap(),
                )
                .expect("Unable to map");
        }
    }

    printk!("\tearly page tables inited\n");
}

/// Do late paging initialization. At this point we have a working physical memory allocator and
/// kernel heap.
pub fn init(boot_info: &'static BootInfo) {
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

    let mut allowed = ALLOWED.lock();
    *allowed = Some(BTreeMap::new());

    printk!("\tvirtual address allocator inited\n");

    ///////////////////////////////////////////////////////////////////////////
    // Initially all page table entries are black listed for userspace, but we want to disable at
    // finer granularity, so we will enable the user accessible bit for the top two levels page
    // tables.
    //
    // Any subsequent allocations will then set their own permissions.
    ///////////////////////////////////////////////////////////////////////////

    let pml4 = unsafe { &mut *(boot_info.recursive_page_table_addr as *mut PageTable) };

    let recursive_index =
        PageTableIndex::new(((boot_info.recursive_page_table_addr >> 12) & 0b111_111_111) as u16);

    for pml4_index in 0..512 {
        // Skip unused entries
        if pml4[pml4_index].is_unused() {
            continue;
        }

        // Set the pml4 entry's flag
        let flags = pml4[pml4_index].flags();
        pml4[pml4_index].set_flags(flags | PageTableFlags::USER_ACCESSIBLE);

        // Iterate through its pdpt and set appropriate flags there too...
        let pdpt_page: *const PageTable = Page::from_page_table_indices(
            recursive_index,
            recursive_index,
            recursive_index,
            PageTableIndex::new(pml4_index as u16),
        )
        .start_address()
        .as_ptr();
        let pdpt = unsafe { &mut *(pdpt_page as *mut PageTable) };

        for pdpt_index in 0..512 {
            if pdpt[pdpt_index].is_unused() {
                continue;
            }

            // Set the pdpt entry's flag
            let flags = pdpt[pdpt_index].flags();
            pdpt[pdpt_index].set_flags(flags | PageTableFlags::USER_ACCESSIBLE);
        }
    }
}

/// Capability on a memory region.
#[derive(Debug)]
pub struct VirtualMemoryRegion {
    /// The first virtual address of the memory region (bytes).
    addr: u64,

    /// The length of the memory region (bytes).
    len: u64,
}

impl VirtualMemoryRegion {
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
    pub fn alloc(npages: usize) -> UnregisteredResourceHandle {
        let mem = VIRT_MEM_ALLOC
            .lock()
            .as_mut()
            .unwrap()
            .alloc(npages)
            .expect("Out of virtual memory.");

        UnregisteredResourceHandle::new(Capability::VirtualMemoryRegion(VirtualMemoryRegion {
            addr: mem as u64 * Size4KiB::SIZE,
            len: npages as u64 * Size4KiB::SIZE,
        }))
    }

    /// Like `alloc`, but adds 2 to npages and calls `guard`.
    pub fn alloc_with_guard(npages: usize) -> UnregisteredResourceHandle {
        let mut mem = Self::alloc(npages + 2);
        if let Capability::VirtualMemoryRegion(mem) = mem.as_mut_ref() {
            mem.guard();
        } else {
            unreachable!();
        }
        mem
    }

    /// The first virtual address of the memory region.
    ///
    /// It is the user's job to make sure that the correct mappings exist before accessing the
    /// address.
    pub fn start(&self) -> *mut u8 {
        self.addr as *mut u8
    }

    /// The length of the region (in bytes).
    pub fn len(&self) -> u64 {
        self.len
    }

    /// Shrink the region by one page at the beginning and end to account for guard pages.
    pub fn guard(&mut self) {
        self.addr += Size4KiB::SIZE;
        self.len -= Size4KiB::SIZE * 2;
    }
}

/// Mark the `region` as usable with the given `flags`. This does not allocate any physical memory.
/// Pages will be allocated by demand paging.
pub fn map_region(region: ResourceHandle, flags: PageTableFlags) {
    let (start, len) = {
        region.with(|cap| {
            let region = cap_unwrap!(VirtualMemoryRegion(cap));
            (region.start(), region.len())
        })
    };
    ALLOWED
        .lock()
        .as_mut()
        .unwrap()
        .insert(start as u64, (len, flags));
}

/// Handle a page fault
pub extern "x86-interrupt" fn handle_page_fault(
    esf: &mut InterruptStackFrame,
    // TODO: Fault frame and interrupt frame are not the same, but the stack should contain the
    // correct error code.
    _error: PageFaultErrorCode,
) {
    // Read CR2 to get the page fault address
    let cr2: u64;
    unsafe {
        asm! {
            "movq %cr2, $0"
             : "=r"(cr2)
             : /* no input */
             : /* no clobbers */
             : "volatile"
        };
    }

    // Check if the page is allowed. We need to check if any range contains the fault address.
    if let Some((start, (len, flags))) = ALLOWED.lock().as_ref().unwrap().range(0..=cr2).next_back()
    {
        printk!(
            "Page fault at ip {:x}, addr {:x}. Found region start: {:x}, len: {}, flags: {:?}\n",
            esf.instruction_pointer.as_u64(),
            cr2,
            start,
            len,
            flags
        );

        // Map the correct region
        let page: Page<Size4KiB> =
            Page::from_start_address(VirtAddr::new(*start as u64)).expect("Region is unaligned");
        let frame = PHYS_MEM_ALLOC
            .lock()
            .as_mut()
            .unwrap()
            .allocate_frame()
            .expect("Unable to allocate physical memory");
        PAGE_TABLES
            .lock()
            .as_mut()
            .unwrap()
            .map_to(page, frame, *flags, PHYS_MEM_ALLOC.lock().as_mut().unwrap())
            .expect("Unable to map page");
    } else {
        panic!(
            "Segfault at ip {:x}, addr {:x}",
            esf.instruction_pointer.as_u64(),
            cr2,
        );
    }
}
