pub mod mmap;

use crate::alloc::Alloc;
use crate::instance::InstanceHandle;
use crate::module::Module;
use failure::Error;
use libc::c_void;

/// A `Region` is a collection of `Slot`s which are managed as a whole.
pub trait Region {
    fn new_instance_with_ctx(
        &self,
        module: Box<dyn Module>,
        embed_ctx: *mut c_void,
    ) -> Result<InstanceHandle, Error>;

    fn new_instance(&self, module: Box<dyn Module>) -> Result<InstanceHandle, Error> {
        self.new_instance_with_ctx(module, std::ptr::null_mut())
    }

    /// Unmaps the heap, stack, and globals of an `Alloc`, while retaining the virtual address
    /// ranges in its `Slot`.
    fn drop_alloc(&self, alloc: &mut Alloc);

    /// Expand the heap by some number of bytes, returning the offset in the heap at which the new
    /// space begins.
    fn expand_heap(&self, alloc: &mut Alloc, expand_bytes: u32) -> Result<u32, Error>;

    fn reset_heap(&self, alloc: &mut Alloc, module: &dyn Module) -> Result<(), Error>;
}
