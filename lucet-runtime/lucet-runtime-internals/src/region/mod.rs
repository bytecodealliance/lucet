pub mod mmap;

use crate::alloc::{Alloc, Slot};
use crate::error::Error;
use crate::instance::InstanceHandle;
use crate::module::Module;
use std::sync::Arc;

/// A memory region in which Lucet instances are created and run.
///
/// These methods return an [`InstanceHandle`](struct.InstanceHandle.html) smart pointer rather than
/// the `Instance` itself. This allows the region implementation complete control of where the
/// instance metadata is stored.
pub trait Region: RegionInternal {
    /// Create a new instance within the region.
    ///
    /// # Safety
    ///
    /// This function runs the guest code for the WebAssembly `start` section, and running any guest
    /// code is potentially unsafe; see [`Instance::run()`](struct.Instance.html#method.run).
    fn new_instance(&self, module: Arc<dyn Module>) -> Result<InstanceHandle, Error>;
}

/// A `RegionInternal` is a collection of `Slot`s which are managed as a whole.
pub trait RegionInternal: Send + Sync {
    /// Unmaps the heap, stack, and globals of an `Alloc`, while retaining the virtual address
    /// ranges in its `Slot`.
    fn drop_alloc(&self, alloc: &mut Alloc);

    /// Expand the heap for the given slot to include the given range.
    fn expand_heap(&self, slot: &Slot, start: u32, len: u32) -> Result<(), Error>;

    fn reset_heap(&self, alloc: &mut Alloc, module: &dyn Module) -> Result<(), Error>;
}
