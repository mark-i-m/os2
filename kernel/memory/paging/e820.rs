//! Utilities for reading E820 info about physical memory.

use alloc::Vec;
use core::ops::Deref;

use os_bootinfo::MemoryRegion;

extern "C" {
    /// The number of entries in `memory_map`.
    static memory_map_count: u32;

    /// The E820 table in memory.
    static memory_map: [MemoryRegion; 32];
}

pub struct E820Info {
    regions: Vec<(usize, usize)>,
}

impl E820Info {
    pub fn read() -> Self {
        // TODO
        E820Info {
            regions: Vec::new(),
        }
    }

    pub fn num_phys_pages(&self) -> usize {
        0 // TODO
    }
}

impl Deref for E820Info {
    type Target = [(usize, usize)];

    fn deref(&self) -> &Self::Target {
        &self.regions
    }
}
