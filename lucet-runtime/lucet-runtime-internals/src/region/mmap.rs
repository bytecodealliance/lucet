use crate::alloc::{host_page_size, instance_heap_offset, Alloc, Limits, Slot};
use crate::embed_ctx::CtxMap;
use crate::error::Error;
use crate::instance::{new_instance_handle, Instance, InstanceHandle};
use crate::module::Module;
use crate::region::{Region, RegionCreate, RegionInternal};
#[cfg(not(target_os = "linux"))]
use libc::memset;
use libc::{c_void, SIGSTKSZ};
use nix::sys::mman::{madvise, mmap, munmap, MapFlags, MmapAdvise, ProtFlags};
use std::ptr;
use std::sync::{Arc, Mutex, Weak};

/// A [`Region`](../trait.Region.html) backed by `mmap`.
///
/// `MmapRegion` lays out memory for instances in a contiguous block,
/// with an Instance's space reserved, followed by heap, stack, globals, and sigstack.
///
/// This results in an actual layout of an instance on an `MmapRegion`-produced `Slot` being:
/// ```text
/// 0x0000: +-----------------------+ <-- Instance
/// 0x0000: |  .magic               |
/// 0x0008: |  ...                  |
/// 0x000X: |  ...                  |
/// 0x0XXX: |  .alloc -> Alloc {    |
/// 0x0XXX: |    .start    = 0x0000 |
/// 0x0XXX: |    .heap     = 0x1000 |
/// 0x0XXX: |    .stack    = 0xN000 |
/// 0x0XXX: |    .globals  = 0xM000 |
/// 0x0XXX: |    .sigstack = 0xS000 |
/// 0x0XXX: |  }                    |
/// 0x0XXX: |  ...                  |
/// 0x0XXX: ~      ~padding~        ~
/// 0x0XXX: |  ...                  |
/// 0x0XXX: |  .globals    = 0xM000 | <-- InstanceRuntimeData
/// 0x0XXX: |  .inst_count = 0x0000 |
/// 0x1000: +-----------------------+ <-- Heap, and `lucet_vmctx`. One page into the allocation.
/// 0x1XXX: |                       |
/// 0xXXXX: ~  .......heap.......   ~ // heap size is governed by limits.heap_address_space_size
/// 0xXXXX: |                       |
/// 0xN000: +-----------------------| <-- Stack (at heap_start + limits.heap_address_space_size)
/// 0xNXXX: |                       |
/// 0xXXXX: ~  .......stack......   ~ // stack size is governed by limits.stack_size
/// 0xXXXX: |                       |
/// 0xXXXx: --- stack guard page ----
/// 0xM000: +-----------------------| <-- Globals (at stack_start + limits.stack_size + PAGE_SIZE)
/// 0xMXXX: |                       |
/// 0xXXXX: ~  ......globals.....   ~
/// 0xXXXX: |                       |
/// 0xXXXX  --- global guard page ---
/// 0xS000: +-----------------------| <-- Sigstack (at globals_start + globals_size + PAGE_SIZE)
/// 0xSXXX: |  ......sigstack....   | // sigstack is SIGSTKSZ bytes
/// 0xSXXX: +-----------------------|
/// ```
pub struct MmapRegion {
    capacity: usize,
    freelist: Mutex<Vec<Slot>>,
    limits: Limits,
}

impl Region for MmapRegion {}

impl RegionInternal for MmapRegion {
    fn new_instance_with(
        &self,
        module: Arc<dyn Module>,
        embed_ctx: CtxMap,
    ) -> Result<InstanceHandle, Error> {
        let slot = self
            .freelist
            .lock()
            .unwrap()
            .pop()
            .ok_or(Error::RegionFull(self.capacity))?;

        if slot.heap as usize % host_page_size() != 0 {
            lucet_bail!("heap is not page-aligned; this is a bug");
        }

        let limits = &slot.limits;
        module.validate_runtime_spec(limits)?;

        for (ptr, len) in [
            // make the stack read/writable
            (slot.stack, limits.stack_size),
            // make the globals read/writable
            (slot.globals, limits.globals_size),
            // make the sigstack read/writable
            (slot.sigstack, SIGSTKSZ),
        ]
        .into_iter()
        {
            // eprintln!("setting r/w {:p}[{:x}]", *ptr, len);
            unsafe { mprotect(*ptr, *len, ProtFlags::PROT_READ | ProtFlags::PROT_WRITE)? };
        }

        // note: the initial heap will be made read/writable when `new_instance_handle` calls `reset`

        let inst_ptr = slot.start as *mut Instance;

        // upgrade the slot's weak region pointer so the region can't get dropped while the instance
        // exists
        let region = slot
            .region
            .upgrade()
            // if this precondition isn't met, something is deeply wrong as some other region's slot
            // ended up in our freelist
            .expect("backing region of slot (`self`) exists");

        let alloc = Alloc {
            heap_accessible_size: 0, // the `reset` call in `new_instance_handle` will set this
            heap_inaccessible_size: slot.limits.heap_address_space_size,
            slot: Some(slot),
            region,
        };

        let inst = new_instance_handle(inst_ptr, module, alloc, embed_ctx)?;

        Ok(inst)
    }

    fn drop_alloc(&self, alloc: &mut Alloc) {
        let slot = alloc
            .slot
            .take()
            .expect("alloc didn't have a slot during drop; dropped twice?");

        if slot.heap as usize % host_page_size() != 0 {
            panic!("heap is not page-aligned");
        }

        // clear and disable access to the heap, stack, globals, and sigstack
        for (ptr, len) in [
            // We don't ever shrink the heap, so we only need to zero up until the accessible size
            (slot.heap, alloc.heap_accessible_size),
            (slot.stack, slot.limits.stack_size),
            (slot.globals, slot.limits.globals_size),
            (slot.sigstack, SIGSTKSZ),
        ]
        .into_iter()
        {
            // eprintln!("setting none {:p}[{:x}]", *ptr, len);
            unsafe {
                // MADV_DONTNEED is not guaranteed to clear pages on non-Linux systems
                #[cfg(not(target_os = "linux"))]
                {
                    mprotect(*ptr, *len, ProtFlags::PROT_READ | ProtFlags::PROT_WRITE)
                        .expect("mprotect succeeds during drop");
                    memset(*ptr, 0, *len);
                }
                mprotect(*ptr, *len, ProtFlags::PROT_NONE).expect("mprotect succeeds during drop");
                madvise(*ptr, *len, MmapAdvise::MADV_DONTNEED)
                    .expect("madvise succeeds during drop");
            }
        }

        self.freelist.lock().unwrap().push(slot);
    }

    fn expand_heap(&self, slot: &Slot, start: u32, len: u32) -> Result<(), Error> {
        unsafe {
            mprotect(
                (slot.heap as usize + start as usize) as *mut c_void,
                len as usize,
                ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
            )?;
        }
        Ok(())
    }

    fn reset_heap(&self, alloc: &mut Alloc, module: &dyn Module) -> Result<(), Error> {
        let heap = alloc.slot().heap;

        if alloc.heap_accessible_size > 0 {
            // zero the whole heap, if any of it is currently accessible
            let heap_size = alloc.slot().limits.heap_address_space_size;

            unsafe {
                // `mprotect()` and `madvise()` are sufficient to zero a page on Linux,
                // but not necessarily on all POSIX operating systems, and on macOS in particular.
                #[cfg(not(target_os = "linux"))]
                {
                    mprotect(
                        heap,
                        alloc.heap_accessible_size,
                        ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
                    )?;
                    memset(heap, 0, alloc.heap_accessible_size);
                }
                mprotect(heap, heap_size, ProtFlags::PROT_NONE)?;
                madvise(heap, heap_size, MmapAdvise::MADV_DONTNEED)?;
            }
        }

        let initial_size = module
            .heap_spec()
            .map(|h| h.initial_size as usize)
            .unwrap_or(0);

        // reset the heap to the initial size, and mprotect those pages appropriately
        if initial_size > 0 {
            unsafe {
                mprotect(
                    heap,
                    initial_size,
                    ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
                )?
            };
        }
        alloc.heap_accessible_size = initial_size;
        alloc.heap_inaccessible_size = alloc.slot().limits.heap_address_space_size - initial_size;

        // Initialize the heap using the module sparse page data. There cannot be more pages in the
        // sparse page data than will fit in the initial heap size.
        //
        // Pages with a corresponding Some entry in the sparse page data are initialized with
        // the contents of that data.
        //
        // Any pages which don't have an entry in the sparse page data, either because their entry
        // is None, or because the sparse data has fewer pages than the initial heap, are zeroed.
        let heap = unsafe { alloc.heap_mut() };
        let initial_pages =
            initial_size
                .checked_div(host_page_size())
                .ok_or(lucet_incorrect_module!(
                    "initial heap size {} is not divisible by host page size ({})",
                    initial_size,
                    host_page_size()
                ))?;
        for page_num in 0..initial_pages {
            let page_base = page_num * host_page_size();
            if heap.len() < page_base {
                return Err(lucet_incorrect_module!(
                    "sparse page data length exceeded initial heap size"
                ));
            }
            if let Some(contents) = module.get_sparse_page_data(page_num) {
                // otherwise copy in the page data
                heap[page_base..page_base + host_page_size()].copy_from_slice(contents);
            }
        }

        Ok(())
    }

    fn as_dyn_internal(&self) -> &dyn RegionInternal {
        self
    }
}

impl Drop for MmapRegion {
    fn drop(&mut self) {
        for slot in self.freelist.get_mut().unwrap().drain(0..) {
            Self::free_slot(slot);
        }
    }
}

impl RegionCreate for MmapRegion {
    const TYPE_NAME: &'static str = "MmapRegion";

    fn create(instance_capacity: usize, limits: &Limits) -> Result<Arc<Self>, Error> {
        MmapRegion::create(instance_capacity, limits)
    }
}

impl MmapRegion {
    /// Create a new `MmapRegion` that can support a given number instances, each subject to the
    /// same runtime limits.
    ///
    /// The region is returned in an `Arc`, because any instances created from it carry a reference
    /// back to the region.
    pub fn create(instance_capacity: usize, limits: &Limits) -> Result<Arc<Self>, Error> {
        assert!(
            SIGSTKSZ % host_page_size() == 0,
            "signal stack size is a multiple of host page size"
        );
        limits.validate()?;

        let region = Arc::new(MmapRegion {
            capacity: instance_capacity,
            freelist: Mutex::new(Vec::with_capacity(instance_capacity)),
            limits: limits.clone(),
        });
        {
            let mut freelist = region.freelist.lock().unwrap();
            for _ in 0..instance_capacity {
                freelist.push(MmapRegion::create_slot(&region)?);
            }
        }

        Ok(region)
    }

    fn create_slot(region: &Arc<MmapRegion>) -> Result<Slot, Error> {
        // get the chunk of virtual memory that the `Slot` will manage
        let mem = unsafe {
            mmap(
                ptr::null_mut(),
                region.limits.total_memory_size(),
                ProtFlags::PROT_NONE,
                MapFlags::MAP_ANON | MapFlags::MAP_PRIVATE,
                0,
                0,
            )?
        };

        // set the first part of the memory to read/write so that the `Instance` can be stored there
        // TODO: post slot refactor, is this necessary/desirable?
        unsafe {
            mprotect(
                mem,
                instance_heap_offset(),
                ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
            )?
        };

        // lay out the other sections in memory
        let heap = mem as usize + instance_heap_offset();
        let stack = heap + region.limits.heap_address_space_size;
        let globals = stack + region.limits.stack_size + host_page_size();
        let sigstack = globals + host_page_size();

        Ok(Slot {
            start: mem,
            heap: heap as *mut c_void,
            stack: stack as *mut c_void,
            globals: globals as *mut c_void,
            sigstack: sigstack as *mut c_void,
            limits: region.limits.clone(),
            region: Arc::downgrade(region) as Weak<dyn RegionInternal>,
        })
    }

    fn free_slot(slot: Slot) {
        // eprintln!(
        //     "unmapping {:p}[{:x}]",
        //     slot.start,
        //     slot.limits.total_memory_size()
        // );
        let res = unsafe { munmap(slot.start, slot.limits.total_memory_size()) };
        res.expect("munmap succeeded");
    }
}

// TODO: remove this once `nix` PR https://github.com/nix-rust/nix/pull/991 is merged
unsafe fn mprotect(addr: *mut c_void, length: libc::size_t, prot: ProtFlags) -> nix::Result<()> {
    nix::errno::Errno::result(libc::mprotect(addr, length, prot.bits())).map(drop)
}
