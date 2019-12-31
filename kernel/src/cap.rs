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

use rand::{Rng, SeedableRng};

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
pub trait Enable: Send + core::fmt::Debug {}

/// A handle to a resource in the capability registry.
#[derive(Debug)]
pub struct ResourceHandle<R: Enable + 'static> {
    /// An index into the capability registry.
    key: u128,

    /// Conceptually, the resource handle owns a reference to the resource.
    _resource: PhantomData<&'static R>,
}

impl<R: Enable + 'static> Clone for ResourceHandle<R> {
    fn clone(&self) -> Self {
        ResourceHandle {
            key: self.key.clone(),
            _resource: PhantomData,
        }
    }
}

impl<R: Enable + 'static> Copy for ResourceHandle<R> {}

/// A capability that has not been registered yet.  An unregistered capability can be modified
/// until it is registered.
#[derive(Debug)]
pub struct UnregisteredResourceHandle<R: Enable + 'static> {
    resource: R,
}

impl<R: Enable + 'static> UnregisteredResourceHandle<R> {
    /// Create a new unregistered resource handle.
    pub fn new(resource: R) -> Self {
        UnregisteredResourceHandle { resource }
    }

    /// Register this unregistered resource handle. After this is done, the resource handle cannot
    /// be updated.
    pub fn register(self) -> ResourceHandle<R> {
        let mut locked = CAPABILITY_REGISTRY.lock();

        // Generate a new random key. We are generating 128-bit random value, so the odds of a
        // collision by chance or by malicious users are extremely low.
        //
        // NOTE: I am not actually using a random sequence because I am seeding the RNG.
        let mut rand = rand::rngs::StdRng::from_seed([0; 32]).gen();

        while locked.as_mut().unwrap().contains_key(&rand) {
            // extremely unlikely...
            rand = rand;
        }

        locked
            .as_mut()
            .unwrap()
            .insert(rand, Box::new(self.resource));

        ResourceHandle {
            key: rand,
            _resource: PhantomData,
        }

        // unlock
    }

    /// Return an immutable reference to the resource.
    pub fn as_ref(&self) -> &R {
        &self.resource
    }

    /// Return a mutable reference to the resource.
    pub fn as_mut_ref(&mut self) -> &mut R {
        &mut self.resource
    }
}

////////////////////////////////////////////////////////////////////////////////
// Implementations of different capabilities.
////////////////////////////////////////////////////////////////////////////////

/// Capability on a group of capabilities.
#[derive(Debug)]
pub struct CapabilityGroup {
    caps: Vec<Box<dyn Enable>>,
}

impl CapabilityGroup {
    pub fn new(caps: Vec<Box<dyn Enable>>) -> Self {
        // TODO: make sure there are no groups within...
        CapabilityGroup { caps }
    }
}

impl Enable for CapabilityGroup {}
