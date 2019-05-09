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
//!
//! # User space
//!
//! Capabilities _must never_ leave kernel mode because they are not fully thread-safe, and we
//! cannot control what users do with them. Instead, we only ever return `ResourceHandle`s and
//! resource metadata to user space.
//!
//! A `ResourceHandle` is guaranteed to be valid until it is destroy by the user.
//!
//! On the other hand, the metadata may become out of date with the actual kernel resource, so the
//! user should be prepared that. Each resource may also make its own guarantees about its
//! metadata, too, in addition to what is guaranteed for all resources.

use alloc::{boxed::Box, collections::BTreeMap, vec::Vec};

use core::marker::PhantomData;

use spin::Mutex;

/// A registry of cabilities.
static CAPABILITY_REGISTRY: Mutex<Option<BTreeMap<u128, Box<dyn Enable>>>> = Mutex::new(None);

/// Init the capability system.
pub fn init() {
    *CAPABILITY_REGISTRY.lock() = Some(BTreeMap::new());

    #[cfg(test)]
    {
        // Type testing: make sure that everything has the right trait bounds.
        CAPABILITY_REGISTRY
            .lock()
            .as_mut()
            .unwrap()
            .insert(0, Box::new(unsafe { VirtualMemoryRegion::new(0, 0) }));
        CAPABILITY_REGISTRY
            .lock()
            .as_mut()
            .unwrap()
            .insert(0, Box::new(CapabilityGroup::new()));
    }
}

/// All capabilities implement this trait.
///
/// It should be safe to send capabilities between (kernel) threads, even though in user mode,
/// resource handles are used instead.
pub trait Enable: Send {}

/// A handle to a resource in the capability registry.
pub struct ResourceHandle<R: Enable + 'static> {
    /// An index into the capability registry.
    key: u128,

    /// Conceptually, the resource handle owns a reference to the resource.
    _resource: PhantomData<&'static R>,
}

/// Capability on a memory region.
pub struct VirtualMemoryRegion {
    /// The first virtual address of the memory region.
    addr: usize,

    /// The length of the memory region.
    len: usize,
}

impl VirtualMemoryRegion {
    /// Create a capability for the given virtual address region. It is up to the caller to make
    /// sure that region is valid before constructing the capability.
    pub unsafe fn new(addr: usize, len: usize) -> Self {
        VirtualMemoryRegion { addr, len }
    }

    /// The first virtual address of the memory region.
    ///
    /// It is the user's job to make sure that the correct mappings exist before accessing the
    /// address.
    pub unsafe fn start(&self) -> usize {
        self.addr
    }
}

impl Enable for VirtualMemoryRegion {}

/// Capability on a group of capabilities.
pub struct CapabilityGroup {
    caps: Vec<Box<dyn Enable>>,
}

impl CapabilityGroup {
    pub fn new(caps: Vec<Box<dyn Enable>>) -> Self {
        CapabilityGroup { caps }
    }
}

impl Enable for CapabilityGroup {}
