use crate::error::Error;
use crate::module::Module;
use crate::region::RegionInternal;
use crate::sysdeps::host_page_size;
use libc::c_void;
use lucet_module::GlobalValue;
use rand::{thread_rng, Rng, RngCore};
use std::fmt;
use std::sync::{Arc, Mutex, Weak};

pub fn instance_heap_offset() -> usize {
    1 * host_page_size()
}

/// A set of pointers into virtual memory that can be allocated into an `Alloc`.
///
/// The `'r` lifetime parameter represents the lifetime of the region that backs this virtual
/// address space.
///
/// The memory layout in a `Slot` is meant to be reused in order to reduce overhead on region
/// implementations. To back the layout with real memory, use `Region::allocate_runtime`.
///
/// To ensure a `Slot` can only be backed by one allocation at a time, it contains a mutex, but
/// otherwise can be freely copied.
#[repr(C)]
pub struct Slot {
    /// The beginning of the contiguous virtual memory chunk managed by this `Alloc`.
    ///
    /// The first part of this memory, pointed to by `start`, is always backed by real memory, and
    /// is used to store the lucet_instance structure.
    pub start: *mut c_void,

    /// The next part of memory contains the heap and its guard pages.
    ///
    /// The heap is backed by real memory according to the `HeapSpec`. Guard pages trigger a sigsegv
    /// when accessed.
    pub heap: *mut c_void,

    /// The stack comes after the heap.
    ///
    /// Because the stack grows downwards, we get the added safety of ensuring that stack overflows
    /// go into the guard pages, if the `Limits` specify guard pages. The stack is always the size
    /// given by `Limits.stack_pages`.
    pub stack: *mut c_void,

    /// The WebAssembly Globals follow the stack and a single guard page.
    pub globals: *mut c_void,

    /// The signal handler stack follows the globals.
    ///
    /// Having a separate signal handler stack allows the signal handler to run in situations where
    /// the normal stack has grown into the guard page.
    pub sigstack: *mut c_void,

    /// Limits of the memory.
    ///
    /// Should not change through the lifetime of the `Alloc`.
    pub limits: Limits,

    pub region: Weak<dyn RegionInternal>,
}

// raw pointers require unsafe impl
unsafe impl Send for Slot {}
unsafe impl Sync for Slot {}

impl Slot {
    pub fn stack_top(&self) -> *mut c_void {
        (self.stack as usize + self.limits.stack_size) as *mut c_void
    }
}

/// The strategy by which a `Region` selects an allocation to back an `Instance`.
#[derive(Clone)]
pub enum AllocStrategy {
    /// Allocate from the next slot available.
    Linear,
    ///  Allocate randomly from the set of available slots.
    Random,
    /// Allocate randomly from the set of available slots using the
    /// supplied random number generator.
    ///
    /// This strategy is used to create reproducible behavior for testing.
    CustomRandom(Arc<Mutex<dyn RngCore + Send>>),
}

impl fmt::Debug for AllocStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            AllocStrategy::Linear => write!(f, "AllocStrategy::Linear"),
            AllocStrategy::Random => write!(f, "AllocStrategy::Random"),
            AllocStrategy::CustomRandom(_) => write!(f, "AllocStrategy::CustomRandom(...)"),
        }
    }
}

impl AllocStrategy {
    /// For a given `AllocStrategy`, use the number of free_slots and
    /// capacity to determine the next slot to allocate for an
    /// `Instance`.
    pub fn next(&mut self, free_slots: usize, capacity: usize) -> Result<usize, Error> {
        if free_slots == 0 {
            return Err(Error::RegionFull(capacity));
        }
        match self {
            AllocStrategy::Linear => Ok(free_slots - 1),
            AllocStrategy::Random => {
                // Instantiate a random number generator and get a
                // random slot index.
                let mut rng = thread_rng();
                Ok(rng.gen_range(0, free_slots))
            }
            AllocStrategy::CustomRandom(custom_rng) => {
                // Get a random slot index using the supplied random
                // number generator.
                let mut rng = custom_rng.lock().unwrap();
                Ok(rng.gen_range(0, free_slots))
            }
        }
    }
}

/// The structure that manages the allocations backing an `Instance`.
///
/// `Alloc`s are not to be created directly, but rather are created by `Region`s during instance
/// creation.
pub struct Alloc {
    pub heap_accessible_size: usize,
    pub heap_inaccessible_size: usize,
    pub heap_memory_size_limit: usize,
    pub slot: Option<Slot>,
    pub region: Arc<dyn RegionInternal>,
}

impl Drop for Alloc {
    fn drop(&mut self) {
        // eprintln!("Alloc::drop()");
        self.region.clone().drop_alloc(self);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AddrLocation {
    Heap,
    InaccessibleHeap,
    StackGuard,
    Stack,
    Globals,
    SigStackGuard,
    SigStack,
    Unknown,
}

impl AddrLocation {
    /// If a fault occurs in this location, is it fatal to the entire process?
    ///
    /// This is currently a permissive baseline that only returns true for unknown locations and the
    /// signal stack guard, in case a `Region` implementation uses faults to populate the accessible
    /// locations like the heap and the globals.
    pub fn is_fault_fatal(self) -> bool {
        use AddrLocation::*;
        match self {
            SigStackGuard | Unknown => true,
            _ => false,
        }
    }
}

impl Alloc {
    /// Where in an `Alloc` does a particular address fall?
    pub fn addr_location(&self, addr: *const c_void) -> AddrLocation {
        let addr = addr as usize;

        let heap_start = self.slot().heap as usize;
        let heap_inaccessible_start = heap_start + self.heap_accessible_size;
        let heap_inaccessible_end = heap_start + self.slot().limits.heap_address_space_size;

        if (addr >= heap_start) && (addr < heap_inaccessible_start) {
            return AddrLocation::Heap;
        }
        if (addr >= heap_inaccessible_start) && (addr < heap_inaccessible_end) {
            return AddrLocation::InaccessibleHeap;
        }

        let stack_start = self.slot().stack as usize;
        let stack_end = stack_start + self.slot().limits.stack_size;
        let stack_guard_start = stack_start - host_page_size();

        if (addr >= stack_guard_start) && (addr < stack_start) {
            return AddrLocation::StackGuard;
        }
        if (addr >= stack_start) && (addr < stack_end) {
            return AddrLocation::Stack;
        }

        let globals_start = self.slot().globals as usize;
        let globals_end = globals_start + self.slot().limits.globals_size;

        if (addr >= globals_start) && (addr < globals_end) {
            return AddrLocation::Globals;
        }

        let sigstack_start = self.slot().sigstack as usize;
        let sigstack_end = sigstack_start + self.slot().limits.signal_stack_size;
        let sigstack_guard_start = sigstack_start - host_page_size();

        if (addr >= sigstack_guard_start) && (addr < sigstack_start) {
            return AddrLocation::SigStackGuard;
        }
        if (addr >= sigstack_start) && (addr < sigstack_end) {
            return AddrLocation::SigStack;
        }

        AddrLocation::Unknown
    }

    pub fn expand_heap(&mut self, expand_bytes: u32, module: &dyn Module) -> Result<u32, Error> {
        let slot = self.slot();

        if expand_bytes == 0 {
            // no expansion takes place, which is not an error
            return Ok(self.heap_accessible_size as u32);
        }

        let host_page_size = host_page_size() as u32;

        if self.heap_accessible_size as u32 % host_page_size != 0 {
            lucet_bail!("heap is not page-aligned; this is a bug");
        }

        if expand_bytes > std::u32::MAX - host_page_size - 1 {
            bail_limits_exceeded!("expanded heap would overflow address space");
        }

        // round the expansion up to a page boundary
        let expand_pagealigned =
            ((expand_bytes + host_page_size - 1) / host_page_size) * host_page_size;

        // `heap_inaccessible_size` tracks the size of the allocation that is addressible but not
        // accessible. We cannot perform an expansion larger than this size.
        if expand_pagealigned as usize > self.heap_inaccessible_size {
            bail_limits_exceeded!("expanded heap would overflow addressable memory");
        }

        // the above makes sure this expression does not underflow
        let guard_remaining = self.heap_inaccessible_size - expand_pagealigned as usize;

        if let Some(heap_spec) = module.heap_spec() {
            // The compiler specifies how much guard (memory which traps on access) must be beyond the
            // end of the accessible memory. We cannot perform an expansion that would make this region
            // smaller than the compiler expected it to be.
            if guard_remaining < heap_spec.guard_size as usize {
                bail_limits_exceeded!("expansion would leave guard memory too small");
            }

            // The compiler indicates that the module has specified a maximum memory size. Don't let
            // the heap expand beyond that:
            if let Some(max_size) = heap_spec.max_size {
                if self.heap_accessible_size + expand_pagealigned as usize > max_size as usize {
                    bail_limits_exceeded!(
                        "expansion would exceed module-specified heap limit: {:?}",
                        max_size
                    );
                }
            }
        } else {
            return Err(Error::NoLinearMemory("cannot expand heap".to_owned()));
        }
        // The runtime sets a limit on how much of the heap can be backed by real memory. Don't let
        // the heap expand beyond that:
        if self.heap_accessible_size + expand_pagealigned as usize > self.heap_memory_size_limit {
            bail_limits_exceeded!(
                "expansion would exceed runtime-specified heap limit: {:?}",
                slot.limits
            );
        }

        let newly_accessible = self.heap_accessible_size;

        self.region
            .clone()
            .expand_heap(slot, newly_accessible as u32, expand_pagealigned)?;

        self.heap_accessible_size += expand_pagealigned as usize;
        self.heap_inaccessible_size -= expand_pagealigned as usize;

        Ok(newly_accessible as u32)
    }

    pub fn reset_heap(&mut self, module: &dyn Module) -> Result<(), Error> {
        self.region.clone().reset_heap(self, module)
    }

    pub fn heap_len(&self) -> usize {
        self.heap_accessible_size
    }

    pub fn slot(&self) -> &Slot {
        self.slot
            .as_ref()
            .expect("alloc missing its slot before drop")
    }

    /// Return the heap as a byte slice.
    pub unsafe fn heap(&self) -> &[u8] {
        std::slice::from_raw_parts(self.slot().heap as *mut u8, self.heap_accessible_size)
    }

    /// Return the heap as a mutable byte slice.
    pub unsafe fn heap_mut(&mut self) -> &mut [u8] {
        std::slice::from_raw_parts_mut(self.slot().heap as *mut u8, self.heap_accessible_size)
    }

    /// Return the heap as a slice of 32-bit words.
    pub unsafe fn heap_u32(&self) -> &[u32] {
        assert!(self.slot().heap as usize % 4 == 0, "heap is 4-byte aligned");
        assert!(
            self.heap_accessible_size % 4 == 0,
            "heap size is multiple of 4-bytes"
        );
        std::slice::from_raw_parts(self.slot().heap as *mut u32, self.heap_accessible_size / 4)
    }

    /// Return the heap as a mutable slice of 32-bit words.
    pub unsafe fn heap_u32_mut(&mut self) -> &mut [u32] {
        assert!(self.slot().heap as usize % 4 == 0, "heap is 4-byte aligned");
        assert!(
            self.heap_accessible_size % 4 == 0,
            "heap size is multiple of 4-bytes"
        );
        std::slice::from_raw_parts_mut(self.slot().heap as *mut u32, self.heap_accessible_size / 4)
    }

    /// Return the heap as a slice of 64-bit words.
    pub unsafe fn heap_u64(&self) -> &[u64] {
        assert!(self.slot().heap as usize % 8 == 0, "heap is 8-byte aligned");
        assert!(
            self.heap_accessible_size % 8 == 0,
            "heap size is multiple of 8-bytes"
        );
        std::slice::from_raw_parts(self.slot().heap as *mut u64, self.heap_accessible_size / 8)
    }

    /// Return the heap as a mutable slice of 64-bit words.
    pub unsafe fn heap_u64_mut(&mut self) -> &mut [u64] {
        assert!(self.slot().heap as usize % 8 == 0, "heap is 8-byte aligned");
        assert!(
            self.heap_accessible_size % 8 == 0,
            "heap size is multiple of 8-bytes"
        );
        std::slice::from_raw_parts_mut(self.slot().heap as *mut u64, self.heap_accessible_size / 8)
    }

    /// Return the stack as a mutable byte slice.
    ///
    /// Since the stack grows down, `alloc.stack_mut()[0]` is the top of the stack, and
    /// `alloc.stack_mut()[alloc.limits.stack_size - 1]` is the last byte at the bottom of the
    /// stack.
    pub unsafe fn stack_mut(&mut self) -> &mut [u8] {
        std::slice::from_raw_parts_mut(self.slot().stack as *mut u8, self.slot().limits.stack_size)
    }

    /// Return the stack as a mutable slice of 64-bit words.
    ///
    /// Since the stack grows down, `alloc.stack_mut()[0]` is the top of the stack, and
    /// `alloc.stack_mut()[alloc.limits.stack_size - 1]` is the last word at the bottom of the
    /// stack.
    pub unsafe fn stack_u64_mut(&mut self) -> &mut [u64] {
        assert!(
            self.slot().stack as usize % 8 == 0,
            "stack is 8-byte aligned"
        );
        assert!(
            self.slot().limits.stack_size % 8 == 0,
            "stack size is multiple of 8-bytes"
        );
        std::slice::from_raw_parts_mut(
            self.slot().stack as *mut u64,
            self.slot().limits.stack_size / 8,
        )
    }

    /// Return the globals as a slice.
    pub unsafe fn globals(&self) -> &[GlobalValue] {
        std::slice::from_raw_parts(
            self.slot().globals as *const GlobalValue,
            self.slot().limits.globals_size / std::mem::size_of::<GlobalValue>(),
        )
    }

    /// Return the globals as a mutable slice.
    pub unsafe fn globals_mut(&mut self) -> &mut [GlobalValue] {
        std::slice::from_raw_parts_mut(
            self.slot().globals as *mut GlobalValue,
            self.slot().limits.globals_size / std::mem::size_of::<GlobalValue>(),
        )
    }

    /// Return the sigstack as a mutable byte slice.
    pub unsafe fn sigstack_mut(&mut self) -> &mut [u8] {
        std::slice::from_raw_parts_mut(
            self.slot().sigstack as *mut u8,
            self.slot().limits.signal_stack_size,
        )
    }

    pub fn mem_in_heap<T>(&self, ptr: *const T, len: usize) -> bool {
        let start = ptr as usize;
        let end = start + len;

        let heap_start = self.slot().heap as usize;
        let heap_end = heap_start + self.heap_accessible_size;

        // TODO: check for off-by-ones
        start <= end
            && start >= heap_start
            && start < heap_end
            && end >= heap_start
            && end <= heap_end
    }
}

/// Runtime limits for the various memories that back a Lucet instance.
///
/// Each value is specified in bytes, and must be evenly divisible by the host page size (4K).
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Limits {
    /// Max size of the heap, which can be backed by real memory. (default 1M)
    pub heap_memory_size: usize,
    /// Size of total virtual memory. (default 8G)
    pub heap_address_space_size: usize,
    /// Size of the guest stack. (default 128K)
    pub stack_size: usize,
    /// Amount of the guest stack that must be available for hostcalls. (default 32K)
    pub hostcall_reservation: usize,
    /// Size of the globals region in bytes; each global uses 8 bytes. (default 4K)
    pub globals_size: usize,
    /// Size of the signal stack in bytes. (default SIGSTKSZ for release builds, at least 12K for
    /// debug builds; minimum MINSIGSTKSZ)
    ///
    /// This difference is to account for the greatly increased stack size usage in the signal
    /// handler when running without optimizations.
    ///
    /// Note that debug vs. release mode is determined by `cfg(debug_assertions)`, so if you are
    /// specifically enabling debug assertions in your release builds, the default signal stack may
    /// be larger.
    pub signal_stack_size: usize,
}

// this constant isn't exported by `libc` on Mac
#[cfg(target_os = "macos")]
pub const MINSIGSTKSZ: usize = 32 * 1024;

#[cfg(not(target_os = "macos"))]
pub const MINSIGSTKSZ: usize = libc::MINSIGSTKSZ;

/// The recommended size of a signal handler stack for a Lucet instance.
///
/// This value is used as the `signal_stack_size` in `Limits::default()`.
///
/// The value of this constant depends on the platform, and on whether Rust optimizations are
/// enabled. In release mode, it is equal to [`SIGSTKSIZE`][sigstksz]. In debug mode, it is equal to
/// `SIGSTKSZ` or 12KiB, whichever is greater.
///
/// [sigstksz]: https://pubs.opengroup.org/onlinepubs/009695399/basedefs/signal.h.html
pub const DEFAULT_SIGNAL_STACK_SIZE: usize = {
    // on Linux, `SIGSTKSZ` is too small for the signal handler when compiled in debug mode
    #[cfg(all(debug_assertions, not(target_os = "macos")))]
    const SIZE: usize = 12 * 1024;

    // on Mac, `SIGSTKSZ` is way larger than we need; it would be nice to combine these debug cases once
    // `std::cmp::max` is a const fn
    #[cfg(all(debug_assertions, target_os = "macos"))]
    const SIZE: usize = libc::SIGSTKSZ;

    #[cfg(not(debug_assertions))]
    const SIZE: usize = libc::SIGSTKSZ;

    SIZE
};

impl Limits {
    pub const fn default() -> Limits {
        Limits {
            heap_memory_size: 16 * 64 * 1024,
            heap_address_space_size: 0x0002_0000_0000,
            stack_size: 128 * 1024,
            hostcall_reservation: 32 * 1024,
            globals_size: 4096,
            signal_stack_size: DEFAULT_SIGNAL_STACK_SIZE,
        }
    }

    pub const fn with_heap_memory_size(mut self, heap_memory_size: usize) -> Self {
        self.heap_memory_size = heap_memory_size;
        self
    }

    pub const fn with_heap_address_space_size(mut self, heap_address_space_size: usize) -> Self {
        self.heap_address_space_size = heap_address_space_size;
        self
    }

    pub const fn with_stack_size(mut self, stack_size: usize) -> Self {
        self.stack_size = stack_size;
        self
    }

    pub const fn with_hostcall_reservation(mut self, hostcall_reservation: usize) -> Self {
        self.hostcall_reservation = hostcall_reservation;
        self
    }

    pub const fn with_globals_size(mut self, globals_size: usize) -> Self {
        self.globals_size = globals_size;
        self
    }

    pub const fn with_signal_stack_size(mut self, signal_stack_size: usize) -> Self {
        self.signal_stack_size = signal_stack_size;
        self
    }

    pub fn total_memory_size(&self) -> usize {
        // Memory is laid out as follows:
        // * the instance (up to instance_heap_offset)
        // * the heap, followed by guard pages
        // * the stack (grows towards heap guard pages)
        // * globals
        // * one guard page (to catch signal stack overflow)
        // * the signal stack

        [
            instance_heap_offset(),
            self.heap_address_space_size,
            host_page_size(),
            self.stack_size,
            self.globals_size,
            host_page_size(),
            self.signal_stack_size,
        ]
        .iter()
        .try_fold(0usize, |acc, &x| acc.checked_add(x))
        .expect("total_memory_size doesn't overflow")
    }

    /// Validate that the limits are aligned to page sizes, and that the stack is not empty.
    pub fn validate(&self) -> Result<(), Error> {
        if self.heap_memory_size % host_page_size() != 0 {
            return Err(Error::InvalidArgument(
                "memory size must be a multiple of host page size",
            ));
        }
        if self.heap_address_space_size % host_page_size() != 0 {
            return Err(Error::InvalidArgument(
                "address space size must be a multiple of host page size",
            ));
        }
        if self.heap_memory_size > self.heap_address_space_size {
            return Err(Error::InvalidArgument(
                "address space size must be at least as large as memory size",
            ));
        }
        if self.stack_size % host_page_size() != 0 {
            return Err(Error::InvalidArgument(
                "stack size must be a multiple of host page size",
            ));
        }
        if self.globals_size % host_page_size() != 0 {
            return Err(Error::InvalidArgument(
                "globals size must be a multiple of host page size",
            ));
        }
        if self.stack_size <= 0 {
            return Err(Error::InvalidArgument("stack size must be greater than 0"));
        }
        // We allow `hostcall_reservation == self.stack_size`, a circumstance that guarantees
        // any hostcalls will fail with a StackOverflow.
        if self.hostcall_reservation > self.stack_size {
            return Err(Error::InvalidArgument(
                "hostcall reserved space must not be greater than stack size",
            ));
        }
        if self.signal_stack_size < MINSIGSTKSZ {
            tracing::info!(
                "signal stack size of {} requires manual configuration of signal stacks",
                self.signal_stack_size
            );
            tracing::debug!(
                "signal stack size must be at least MINSIGSTKSZ \
                 (defined in <signal.h>; {} on this system)",
                MINSIGSTKSZ,
            );
        }
        if cfg!(debug_assertions) && self.signal_stack_size < 12 * 1024 {
            tracing::info!(
                "signal stack size of {} requires manual configuration of signal stacks",
                self.signal_stack_size
            );
            tracing::debug!(
                "in debug mode, signal stack size must be at least MINSIGSTKSZ \
                 (defined in <signal.h>; {} on this system) or 12KiB, whichever is larger",
                MINSIGSTKSZ,
            );
        }
        if self.signal_stack_size % host_page_size() != 0 {
            return Err(Error::InvalidArgument(
                "signal stack size must be a multiple of host page size",
            ));
        }
        Ok(())
    }
}

pub fn validate_sigstack_size(signal_stack_size: usize) -> Result<(), Error> {
    if signal_stack_size < MINSIGSTKSZ {
        return Err(Error::InvalidArgument(
            "signal stack size must be at least MINSIGSTKSZ (defined in <signal.h>)",
        ));
    }
    if cfg!(debug_assertions) && signal_stack_size < 12 * 1024 {
        return Err(Error::InvalidArgument(
            "signal stack size must be at least 12KiB for debug builds",
        ));
    }
    Ok(())
}

pub mod tests;
