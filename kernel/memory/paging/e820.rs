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

/// Safe wrapper around the info from E820.
pub struct E820Info {
    regions: Vec<(usize, usize)>,
}

impl E820Info {
    /// Read the information from the E820 `memory_map` and parse into a safe wrapper.
    pub fn read() -> Self {
        let mut regions = Vec::new();

        // TODO

        E820Info { regions }
    }

    /// Compute the number of physical pages available.
    pub fn num_phys_pages(&self) -> usize {
        0 // TODO
    }
}

// Allows iterating over regions :)
impl Deref for E820Info {
    type Target = [(usize, usize)];

    fn deref(&self) -> &Self::Target {
        &self.regions
    }
}
