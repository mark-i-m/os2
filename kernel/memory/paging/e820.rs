//! Utilities for reading E820 info about physical memory.
//!
//! This module provides an idomatic, safe interface for getting memory regions from the info
//! output by the E820 BIOS call.

use alloc::{Vec, BTreeSet};
use core::ops::Deref;

use os_bootinfo::{MemoryRegion, MemoryRegionType};

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
        // e820 regions in the memory map can overlap. Worse, overlapping regions can have
        // different usability info. Here we will be conservative and say that a portion of memory
        // is usable only if all overlapping regions are marked usable.

        // Also, this function is optimized for readability. Since we only have 32 regions at most,
        // performance is not an issue.

        // First, get all the info from e820. Only the first `memory_map_count` entries are valid.
        let info: Vec<_> = unsafe { memory_map }
            .iter()
            .map(|region| {
                (
                    region.range.start_addr(),
                    region.range.end_addr(),
                    region.region_type,
                )
            })
            .collect();

            printk!("{:?}", info);

        // To make life easy, we will break up partially overlapping regions so that if two regions
        // overlap, they overlap exactly (i.e. same start and end addr).
        let mut endpoints: BTreeSet<u64> = BTreeSet::new();
        for &(start, end, _) in info.iter() {
            endpoints.insert(start);
            endpoints.insert(end);
        }

        let mut info: Vec<_> = info
            .into_iter()
            .flat_map(|(start, end, ty)| {
                let mid: Vec<u64> = endpoints
                    .iter()
                    .map(|&x| x)
                    .filter(|&point| point >= start && point <= end)
                    .collect();
                let mut pieces = Vec::new();

                for i in 0..mid.len() - 1 {
                    pieces.push((mid[i], mid[i + 1], ty));
                }

                pieces
            })
            .collect();

        // Sort by start of region
        info.sort_by_key(|&(start, _, _)| start);

        // Finally, find out if each region is useable.
        let mut regions = Vec::new();
        for start in endpoints.into_iter() {
            let same_start: Vec<_> = info.drain_filter(|&mut (s,_,_)| start == s).collect();
            let all_usable = same_start.iter().all(|&(s,e,ty)| s < e && ty == MemoryRegionType::Usable);

            if all_usable {
                regions.push(same_start.into_iter().next().map(|(s,e,_)| (s as usize,e as usize - 1)).unwrap());
            }
        }

        printk!("{:#?}", regions);

        E820Info { regions }
    }

    /// Compute the number of physical pages available.
    pub fn num_phys_pages(&self) -> usize {
        self.regions.iter().map(|(start, end)| end - start).sum()
    }
}

// Allows iterating over regions :)
impl Deref for E820Info {
    type Target = [(usize, usize)];

    fn deref(&self) -> &Self::Target {
        &self.regions
    }
}
