pub mod mmap;

#[cfg(all(target_os = "linux", feature = "uffd"))]
pub mod uffd;

use crate::alloc::{Alloc, Limits, Slot};
use crate::embed_ctx::CtxMap;
use crate::error::Error;
use crate::instance::InstanceHandle;
use crate::module::Module;
use std::any::Any;
use std::sync::Arc;

/// A memory region in which Lucet instances are created and run.
///
/// These methods return an [`InstanceHandle`](struct.InstanceHandle.html) smart pointer rather than
/// the `Instance` itself. This allows the region implementation complete control of where the
/// instance metadata is stored.
pub trait Region: RegionInternal {
    /// Create a new instance within the region.
    ///
    /// Calling `region.new_instance(module)` is shorthand for
    /// `region.new_instance_builder(module).build()` for use when further customization is
    /// unnecessary.
    ///
    /// # Safety
    ///
    /// This function runs the guest code for the WebAssembly `start` section, and running any guest
    /// code is potentially unsafe; see [`Instance::run()`](struct.Instance.html#method.run).
    fn new_instance(&self, module: Arc<dyn Module>) -> Result<InstanceHandle, Error> {
        self.new_instance_builder(module).build()
    }

    /// Return an [`InstanceBuilder`](struct.InstanceBuilder.html) for the given module.
    fn new_instance_builder<'a>(&'a self, module: Arc<dyn Module>) -> InstanceBuilder<'a> {
        InstanceBuilder::new(self.as_dyn_internal(), module)
    }

    /// Return the number of instance slots that are currently free in the region.
    ///
    /// A value greater than zero does not guarantee that a subsequent call to
    /// `Region::new_instance()` will succeed, as other threads may instantiate from the region in
    /// the meantime.
    fn free_slots(&self) -> usize;

    /// Return the number of instance slots that are currently in use in the region.
    ///
    /// A value less than `self.capacity()` does not guarantee that a subsequent call to
    /// `Region::new_instance()` will succeed, as other threads may instantiate from the region in
    /// the meantime.
    fn used_slots(&self) -> usize;

    /// Return the total instance slot capacity of the region.
    fn capacity(&self) -> usize;
}

/// A `RegionInternal` is a collection of `Slot`s which are managed as a whole.
pub trait RegionInternal: Send + Sync {
    fn new_instance_with(
        &self,
        module: Arc<dyn Module>,
        embed_ctx: CtxMap,
        heap_memory_size_limit: usize,
    ) -> Result<InstanceHandle, Error>;

    /// Unmaps the heap, stack, and globals of an `Alloc`, while retaining the virtual address
    /// ranges in its `Slot`.
    fn drop_alloc(&self, alloc: &mut Alloc);

    /// Expand the heap for the given slot to include the given range.
    fn expand_heap(&self, slot: &Slot, start: u32, len: u32) -> Result<(), Error>;

    fn reset_heap(&self, alloc: &mut Alloc, module: &dyn Module) -> Result<(), Error>;

    /// Get the runtime memory size limits
    fn get_limits(&self) -> &Limits;

    fn as_dyn_internal(&self) -> &dyn RegionInternal;
}

/// A trait for regions that are created with a fixed capacity and limits.
///
/// This is not part of [`Region`](trait.Region.html) so that `Region` types can be made into trait
/// objects.
pub trait RegionCreate: Region {
    /// The type name of the region; useful for testing.
    const TYPE_NAME: &'static str;

    /// Create a new `Region` that can support a given number instances, each subject to the same
    /// runtime limits.
    fn create(instance_capacity: usize, limits: &Limits) -> Result<Arc<Self>, Error>;
}

/// A builder for instances; created by
/// [`Region::new_instance_builder()`](trait.Region.html#method.new_instance_builder).
pub struct InstanceBuilder<'a> {
    region: &'a dyn RegionInternal,
    module: Arc<dyn Module>,
    embed_ctx: CtxMap,
    heap_memory_size_limit: usize,
}

impl<'a> InstanceBuilder<'a> {
    fn new(region: &'a dyn RegionInternal, module: Arc<dyn Module>) -> Self {
        InstanceBuilder {
            region,
            module,
            embed_ctx: CtxMap::default(),
            heap_memory_size_limit: region.get_limits().heap_memory_size,
        }
    }

    /// Add a smaller, custom limit for the heap memory size to the built instance.
    ///
    /// This call is optional. Attempts to build a new instance fail if the
    /// limit supplied by with_heap_size_limit() exceeds that of the region.
    pub fn with_heap_size_limit(mut self, heap_memory_size_limit: usize) -> Self {
        self.heap_memory_size_limit = heap_memory_size_limit;
        self
    }

    /// Add an embedder context to the built instance.
    ///
    /// Up to one context value of any particular type may exist in the instance. If a context value
    /// of the same type already exists, it is replaced by the new value.
    pub fn with_embed_ctx<T: Any>(mut self, ctx: T) -> Self {
        self.embed_ctx.insert(ctx);
        self
    }

    /// Build the instance.
    ///
    /// # Safety
    ///
    /// This function runs the guest code for the WebAssembly `start` section, and running any guest
    /// code is potentially unsafe; see [`Instance::run()`](struct.Instance.html#method.run).
    pub fn build(self) -> Result<InstanceHandle, Error> {
        self.region
            .new_instance_with(self.module, self.embed_ctx, self.heap_memory_size_limit)
    }
}
