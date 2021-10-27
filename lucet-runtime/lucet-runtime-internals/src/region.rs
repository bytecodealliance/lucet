pub mod mmap;

#[cfg(all(target_os = "linux", feature = "uffd"))]
pub mod uffd;

use crate::alloc::{Alloc, AllocStrategy, Limits, Slot};
use crate::embed_ctx::CtxMap;
use crate::error::Error;
use crate::instance::{InstanceHandle, ResourceLimiter};
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
    fn new_instance_with(&self, args: NewInstanceArgs) -> Result<InstanceHandle, Error>;

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
    args: NewInstanceArgs,
}

/// Arguments that a region needs to create a new `Instance`.
///
/// This type is primarily created by `InstanceBuilder`, but its definition is public to support
/// out-of-crate implementations of `RegionInternal`.
pub struct NewInstanceArgs {
    pub module: Arc<dyn Module>,
    pub embed_ctx: CtxMap,
    pub heap_memory_size_limit: usize,
    pub alloc_strategy: AllocStrategy,
    pub terminate_on_heap_oom: bool,
    pub resource_limiter: Option<Box<dyn ResourceLimiter>>,
}

impl<'a> InstanceBuilder<'a> {
    fn new(region: &'a dyn RegionInternal, module: Arc<dyn Module>) -> Self {
        InstanceBuilder {
            region,
            args: NewInstanceArgs {
                module,
                embed_ctx: CtxMap::default(),
                heap_memory_size_limit: region.get_limits().heap_memory_size,
                alloc_strategy: AllocStrategy::Linear,
                terminate_on_heap_oom: false,
                resource_limiter: None,
            },
        }
    }

    /// Allocate the instance using the supplied `AllocStrategy`.
    ///
    /// This call is optional.  The default allocation strategy for
    /// Regions is Linear, which allocates the instance using next available
    /// alloc.  If a different strategy is desired, choose from those
    /// available in `AllocStrategy`.
    pub fn with_alloc_strategy(mut self, alloc_strategy: AllocStrategy) -> Self {
        self.args.alloc_strategy = alloc_strategy;
        self
    }

    /// Add a smaller, custom limit for the heap memory size to the built instance.
    ///
    /// This call is optional. Attempts to build a new instance fail if the
    /// limit supplied by with_heap_size_limit() exceeds that of the region.
    pub fn with_heap_size_limit(mut self, heap_memory_size_limit: usize) -> Self {
        self.args.heap_memory_size_limit = heap_memory_size_limit;
        self
    }

    /// Add an embedder context to the built instance.
    ///
    /// Up to one context value of any particular type may exist in the instance. If a context value
    /// of the same type already exists, it is replaced by the new value.
    pub fn with_embed_ctx<T: Any>(mut self, ctx: T) -> Self {
        self.args.embed_ctx.insert(ctx);
        self
    }

    /// Whether to terminate the guest with `TerminationDetails::HeapOutOfMemory` when `memory.grow`
    /// fails, rather than returning `-1`; disabled by default.
    ///
    /// This behavior deviates from the WebAssembly spec, but is useful in practice for determining
    /// when guest programs fail due to an exhausted heap.
    ///
    /// Most languages will compile to code that includes an `unreachable` instruction if allocation
    /// fails, but this same instruction might also appear when other types of assertions fail,
    /// `panic!()` is called, etc. Terminating allows the error to be more directly identifiable.
    pub fn with_terminate_on_heap_oom(mut self, terminate_on_heap_oom: bool) -> Self {
        self.args.terminate_on_heap_oom = terminate_on_heap_oom;
        self
    }

    /// Add a resource limiter to the built instance.
    ///
    /// This call is optional. It can be used to add additional checks when the instance requests
    /// additional resources, e.g. when growing memory. These checks are useful to take dynamic,
    /// non-WebAssembly-related concerns into account.
    pub fn with_resource_limiter(mut self, limiter: impl ResourceLimiter + 'static) -> Self {
        self.args.resource_limiter = Some(Box::new(limiter));
        self
    }

    /// Build the instance.
    pub fn build(self) -> Result<InstanceHandle, Error> {
        self.region.new_instance_with(self.args)
    }
}
