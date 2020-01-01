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

use bootloader::BootInfo;

use buddy::BuddyAllocator;

use spin::Mutex;

use x86_64::{
    registers::model_specific::{Efer, EferFlags},
    structures::{
        idt::{InterruptStackFrame, PageFaultErrorCode},
        paging::{
            Mapper, Page, PageSize, PageTable, PageTableFlags, PhysFrame, RecursivePageTable,
            Size2MiB, Size4KiB, UnusedPhysFrame,
        },
    },
    PhysAddr, VirtAddr,
};

use crate::cap::{Enable, ResourceHandle, UnregisteredResourceHandle};

/// The kernel's physical frame allocator. It returns frame numbers, not physical addresses.
static PHYS_MEM_ALLOC: Mutex<Option<phys::BuddyAllocator>> = Mutex::new(None);

/// The kernel's virtual memory allocator. It returns page numbers, not virtual addresses. This
/// allocator assigns parts of the 48-bit single address space when asked.
static VIRT_MEM_ALLOC: Mutex<Option<BuddyAllocator<usize>>> = Mutex::new(None);

/// The page tables for the system.
static PAGE_TABLES: Mutex<Option<RecursivePageTable>> = Mutex::new(None);

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

/// Initialize the physical and virtual memory allocators. Set up paging properly.
///
/// Before this, we have a single set of page tables that direct maps the first 2MiB of memory.
///
/// Afterwards, we have set up the phyiscal memory allocator, set up page tables for the single
/// address space, set up the null page, extend the kernel heap, set up the virtual memory
/// allocator for the single address space, and reserve the kernel virtual memory area.
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

    printk!("\tvirtual address allocator inited\n");
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
    pub fn alloc(npages: usize) -> UnregisteredResourceHandle<Self> {
        let mem = VIRT_MEM_ALLOC
            .lock()
            .as_mut()
            .unwrap()
            .alloc(npages)
            .expect("Out of virtual memory.");

        UnregisteredResourceHandle::new(VirtualMemoryRegion {
            addr: mem as u64 * Size4KiB::SIZE,
            len: npages as u64 * Size4KiB::SIZE,
        })
    }

    /// Like `alloc`, but adds 2 to npages and calls `guard`.
    pub fn alloc_with_guard(npages: usize) -> UnregisteredResourceHandle<Self> {
        let mut mem = Self::alloc(npages + 2);
        mem.as_mut_ref().guard();
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
        self.addr -= Size4KiB::SIZE;
        self.len -= Size4KiB::SIZE;
    }
}

impl Enable for VirtualMemoryRegion {}

/// Add page table entries for the given virtual memory region, but don't mark them present or
/// allocate pages. Demand paging will populate them as needed.
pub fn map_region(region: ResourceHandle<VirtualMemoryRegion>, flags: PageTableFlags) {
    todo!();
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

    todo!(
        "Page fault at ip {:x}, addr {:x}",
        esf.instruction_pointer.as_u64(),
        cr2,
    );
}