use crate::module::{Module, RuntimeSpec};
use crate::region::Region;
use failure::Error;
use libc::{c_void, SIGSTKSZ};
use nix::unistd::{sysconf, SysconfVar};
use std::sync::{Arc, Once, Weak};

const HOST_PAGE_SIZE_EXPECTED: usize = 4096;
static mut HOST_PAGE_SIZE: usize = 0;
static HOST_PAGE_SIZE_INIT: Once = Once::new();

/// Our host is Linux x86_64, which should always use a 4K page.
///
/// We double check the expected value using `sysconf` at runtime.
pub fn host_page_size() -> usize {
    unsafe {
        HOST_PAGE_SIZE_INIT.call_once(|| match sysconf(SysconfVar::PAGE_SIZE) {
            Ok(Some(sz)) => {
                if sz as usize == HOST_PAGE_SIZE_EXPECTED {
                    HOST_PAGE_SIZE = HOST_PAGE_SIZE_EXPECTED;
                } else {
                    panic!(
                        "host page size was {}; expected {}",
                        sz, HOST_PAGE_SIZE_EXPECTED
                    );
                }
            }
            _ => panic!("could not get host page size from sysconf"),
        });
        HOST_PAGE_SIZE
    }
}

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

    pub region: Weak<dyn Region>,
}

impl Slot {
    pub fn stack_top(&self) -> *mut c_void {
        (self.stack as usize + self.limits.stack_size) as *mut c_void
    }
}

/// The structure that manages the allocations backing an `Instance`.
///
/// `Alloc`s are not to be created directly, but rather are created by `Region`s during instance
/// creation.
pub struct Alloc {
    pub heap_accessible_size: usize,
    pub heap_inaccessible_size: usize,
    pub runtime_spec: RuntimeSpec,
    pub slot: Option<Slot>,
    pub region: Arc<dyn Region>,
}

impl Drop for Alloc {
    fn drop(&mut self) {
        // eprintln!("Alloc::drop()");
        self.region.clone().drop_alloc(self);
    }
}

impl Alloc {
    pub fn addr_in_heap_guard(&self, addr: *const c_void) -> bool {
        let heap = self.slot().heap as usize;
        let guard_start = heap + self.heap_accessible_size;
        let guard_end = heap + self.slot().limits.heap_address_space_size;
        // eprintln!(
        //     "addr = {:p}, guard_start = {:p}, guard_end = {:p}",
        //     addr, guard_start as *mut c_void, guard_end as *mut c_void
        // );
        (addr as usize >= guard_start) && ((addr as usize) < guard_end)
    }

    pub fn expand_heap(&mut self, expand_bytes: u32) -> Result<u32, Error> {
        self.region.clone().expand_heap(self, expand_bytes)
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
    pub unsafe fn heap_u32_mut(&self) -> &mut [u32] {
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

    /// Return the globals as a mutable slice.
    pub unsafe fn globals_mut(&mut self) -> &mut [i64] {
        std::slice::from_raw_parts_mut(
            self.slot().globals as *mut i64,
            self.slot().limits.globals_size / 8,
        )
    }

    /// Return the sigstack as a mutable byte slice.
    pub unsafe fn sigstack_mut(&mut self) -> &mut [u8] {
        std::slice::from_raw_parts_mut(self.slot().sigstack as *mut u8, libc::SIGSTKSZ)
    }

    pub fn mem_in_heap(&self, ptr: *const c_void, len: usize) -> bool {
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
#[derive(Clone, Debug)]
pub struct Limits {
    /// Max size of the heap, which can be backed by real memory. (default 1M)
    pub heap_memory_size: usize,
    /// Size of total virtual memory. (default 8M)
    pub heap_address_space_size: usize,
    /// Size of the guest stack. (default 128K)
    pub stack_size: usize,
    /// Size of the globals region in bytes; each global uses 8 bytes. (default 4K)
    pub globals_size: usize,
}

impl Default for Limits {
    fn default() -> Limits {
        Limits {
            heap_memory_size: 16 * 64 * 1024,
            heap_address_space_size: 8 * 1024 * 1024,
            stack_size: 128 * 1024,
            globals_size: 4096,
        }
    }
}

impl Limits {
    pub fn total_memory_size(&self) -> usize {
        // Memory is laid out as follows:
        // * the instance (up to instance_heap_offset)
        // * the heap, followed by guard pages
        // * the stack (grows towards heap guard pages)
        // * one guard page (for good luck?)
        // * globals
        // * one guard page (to catch signal stack overflow)
        // * the signal stack (size given by signal.h SIGSTKSZ macro)
        instance_heap_offset()
            + self.heap_address_space_size as usize
            + self.stack_size as usize
            + host_page_size()
            + self.globals_size as usize
            + host_page_size()
            + SIGSTKSZ
    }

    /// Validate that the limits are aligned to page sizes, and that the stack is not empty.
    pub fn validate(&self) -> bool {
        self.heap_memory_size % host_page_size() == 0
            && self.heap_address_space_size % host_page_size() == 0
            && self.stack_size % host_page_size() == 0
            && self.globals_size % host_page_size() == 0
            && self.stack_size > 0
    }
}

pub mod tests;
