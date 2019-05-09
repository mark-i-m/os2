//! Capability system.
//!
//! A capability is unique handle on a resource which gives the holder access to the resource.
//! Having the capability is sufficient to give access to the resource, and it contains enough
//! information to fully describe the resource.
//!
//! A capability can be passed around to give other holders access.
//!
//! All capabilities are stored in a global kernel-level capability registry.
//!
//! # Optimizations
//!
//! Capabilities can be large, so passing them around could incur a performance hit, especially as
//! passing capabilities to continuations is a frequent occurence. Thus, capabilities are uniquely
//! identified by a 128-bit `ResourceHandle`, which can be used to index into the capability
//! registry.
//!
//! However, passing around a lot of capabilities still means passing around a lot of 128-bit
//! handles. To mitigate this, handles can be grouped into a `ResourceGroup`, which is a capability
//! that contains other capabilities and gives access to all of them. To keep things simple,
//! capability groups may _not_ have other groups in them.

use alloc::{boxed::Box, collections::BTreeMap, sync::Arc, vec::Vec};

use core::pin::Pin;

use spin::Mutex;

/// A registry of cabilities.
static CAPABILITY_REGISTRY: Mutex<Option<BTreeMap<u128, Box<dyn Enable>>>> = Mutex::new(None);

/// Init the capability system.
pub fn init() {
    *CAPABILITY_REGISTRY.lock() = Some(BTreeMap::new());

    // TODO: type testing
    CAPABILITY_REGISTRY
        .lock()
        .as_mut()
        .unwrap()
        .insert(0, Box::new(Capability::new(VirtualMemoryRegion::new(0, 0))));
    CAPABILITY_REGISTRY
        .lock()
        .as_mut()
        .unwrap()
        .insert(0, Box::new(Capability::new(CapabilityGroup::new(0, 0))));
}

/// All capabilities implement this trait.
///
/// A capability should be thread-safe so that it can sent across threads.
pub trait Enable: Send {}

/// A handle to a resource in the capability registry.
pub struct ResourceHandle {
    /// An index into the capability registry.
    key: u128,
}

/// A reference-counted, immutable, pinned capability around some resource.
///
/// This is a convenience for turning an arbitrary struct into a thread-safe immutable capability.
/// If the capability needs to be mutable, then either you need to implement `Enable` for it
/// manually by making it thread-safe, or keep the mutable part somewhere else.
pub struct Capability<R> {
    cap: Arc<Mutex<R>>,
}

impl<R> Enable for Capability<R> {}

impl<R> Capability<R> {
    /// construct a new capability
    pub fn new(cap: R) -> Self {
        Capability { cap: Arc::pin(cap) }
    }

    pub fn cap(&self) -> &R {
        &self.cap
    }
}

/// Capability on a memory region.
pub struct VirtualMemoryRegion {
    /// The first linear address of the memory region.
    ///
    /// TODO: unsafe accessor: it is the user's job to make sure that the correct mappings exist
    /// before accessing the address.
    addr: usize,

    /// The length of the memory region.
    len: usize,
}

impl VirtualMemoryRegion {
    pub fn new(addr: usize, len: usize) -> Self {
        VirtualMemoryRegion { addr, len }
    }
}

/// Capability on a group of capabilities.
pub struct CapabilityGroup {
    caps: Box<dyn Enable>,
}

impl CapabilityGroup {
    pub fn new() -> Self {
        CapabilityGroup {
            caps: Box::new(Capability::new(VirtualMemoryRegion::new(0, 0))), // TODO
        }
    }
}
