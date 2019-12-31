//! This file contains the memory allocator used by the kernel. It is a thin wrapper around
//! smallheap.

use core::alloc::{GlobalAlloc, Layout};
use core::cell::RefCell;

use smallheap::Allocator;

/// A wrapper around the heap allocator for use as the `global_allocator`.
pub struct KernelAllocator {
    heap: RefCell<Option<Allocator>>,
}

impl KernelAllocator {
    pub const fn new() -> Self {
        KernelAllocator {
            heap: RefCell::new(None),
        }
    }

    pub fn set_heap(&mut self, heap: Allocator) {
        *self.heap.borrow_mut() = Some(heap);
    }

    pub unsafe fn extend(&mut self, start: *mut u8, size: usize) {
        self.heap.borrow_mut().as_mut().unwrap().extend(start, size)
    }

    pub fn size(&self) -> usize {
        self.heap.borrow().as_ref().unwrap().size()
    }
}

unsafe impl GlobalAlloc for KernelAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.heap
            .borrow_mut()
            .as_mut()
            .unwrap()
            .malloc(layout.size(), layout.align())
            .map(|p| p.as_ptr() as *mut u8)
            .unwrap_or(core::ptr::null_mut())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.heap
            .borrow_mut()
            .as_mut()
            .unwrap()
            .free(ptr as *mut u8, layout.size())
    }
}

/// Initialize the kernel heap
pub fn init(allocator: &mut KernelAllocator, start: usize, size: usize) {
    unsafe {
        allocator.extend(start as *mut u8, size);
    }

    let free_size = allocator.size();

    printk!(
        "\theap inited - start addr: 0x{:x}, end addr: 0x{:x}, {} bytes\n",
        start,
        start + size,
        free_size,
    );
}

#[alloc_error_handler]
fn oom(_: Layout) -> ! {
    panic!("OOM!");
}

pub mod early {
    use smallheap::Allocator;

    use super::KernelAllocator;

    /// Reserve some space in the kernel text section for a small initial kernel heap.
    ///
    /// It is a pain to try to parse the memory map and manually allocate memory for the kernel heap
    /// before the memory allocator can be initialized. Instead, we use this reserved space in the
    /// kernel text section to bootstrap the heap.
    static mut INITIAL_KHEAP_SPACE: InitialHeapSpace = InitialHeapSpace::empty();

    /// The size of the initial heap space in bytes.
    const INITIAL_KHEAP_SPACE_SIZE: usize = 4 << 12;

    /// An aligned region for the initial heap.
    #[repr(C, align(4096))]
    struct InitialHeapSpace([u8; INITIAL_KHEAP_SPACE_SIZE]);

    impl InitialHeapSpace {
        const fn empty() -> Self {
            InitialHeapSpace([0; INITIAL_KHEAP_SPACE_SIZE])
        }
    }

    pub fn init(allocator: &mut KernelAllocator) {
        let init_heap_start = unsafe { (&mut INITIAL_KHEAP_SPACE) as *mut InitialHeapSpace }.cast();
        let heap = unsafe { Allocator::new(init_heap_start, INITIAL_KHEAP_SPACE_SIZE) };
        let free_size = heap.size();

        allocator.set_heap(heap);

        printk!(
            "\tearly heap inited - start addr: 0x{:x}, end addr: 0x{:x}, {} bytes\n",
            init_heap_start as usize,
            init_heap_start as usize + INITIAL_KHEAP_SPACE_SIZE,
            free_size,
        );
    }
}
