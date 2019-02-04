pub mod mmap;

use crate::alloc::{Alloc, Slot};
use crate::error::Error;
use crate::instance::InstanceHandle;
use crate::module::Module;
use libc::c_void;

/// A memory region in which Lucet instances are created and run.
///
/// These methods return an [`InstanceHandle`](struct.InstanceHandle.html) smart pointer rather than
/// the `Instance` itself. This allows the region implementation complete control of where the
/// instance metadata is stored.
pub trait Region: RegionInternal {
    /// Create a new instance within the region with an [embedding
    /// context](index.html#embedding-with-hostcalls).
    fn new_instance_with_ctx(
        &self,
        module: Box<dyn Module>,
        embed_ctx: *mut c_void,
    ) -> Result<InstanceHandle, Error>;

    /// Create a new instance within the region.
    fn new_instance(&self, module: Box<dyn Module>) -> Result<InstanceHandle, Error> {
        self.new_instance_with_ctx(module, std::ptr::null_mut())
    }
}

/// A `RegionInternal` is a collection of `Slot`s which are managed as a whole.
pub trait RegionInternal {
    /// Unmaps the heap, stack, and globals of an `Alloc`, while retaining the virtual address
    /// ranges in its `Slot`.
    fn drop_alloc(&self, alloc: &mut Alloc);

    /// Expand the heap for the given slot to include the given range.
    fn expand_heap(&self, slot: &Slot, start: u32, len: u32) -> Result<(), Error>;

    fn reset_heap(&self, alloc: &mut Alloc, module: &dyn Module) -> Result<(), Error>;
}
