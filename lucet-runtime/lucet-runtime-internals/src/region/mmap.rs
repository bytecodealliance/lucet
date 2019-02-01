use crate::alloc::{host_page_size, instance_heap_offset, Alloc, Limits, Slot};
use crate::instance::{new_instance_handle, Instance, InstanceHandle};
use crate::module::Module;
use crate::region::{Region, RegionInternal};
use failure::{bail, format_err, Error};
use libc::{c_void, SIGSTKSZ};
use nix::sys::mman::{madvise, mmap, munmap, MapFlags, MmapAdvise, ProtFlags};
use std::mem;
use std::ptr;
use std::sync::{Arc, Mutex, Weak};

pub struct MmapRegion {
    freelist: Mutex<Vec<Slot>>,
    limits: Limits,
}

impl Region for MmapRegion {
    fn new_instance_with_ctx(
        &self,
        module: Box<dyn Module>,
        embed_ctx: *mut c_void,
    ) -> Result<InstanceHandle, Error> {
        let slot = self
            .freelist
            .lock()
            .unwrap()
            .pop()
            .ok_or(format_err!("no available slots on region"))?;

        if slot.heap as usize % host_page_size() != 0 {
            panic!("heap is not page-aligned");
        }

        let runtime_spec = module.runtime_spec();

        // Assure that the total reserved + guard regions fit in the address space.
        // First check makes sure they fit our 32-bit model, and ensures the second
        // check doesn't overflow.
        if runtime_spec.heap.reserved_size > std::u32::MAX as u64 + 1
            || runtime_spec.heap.guard_size > std::u32::MAX as u64 + 1
        {
            bail!("spec over limits");
        }

        let limits = &slot.limits;

        if runtime_spec.heap.reserved_size as usize + runtime_spec.heap.guard_size as usize
            > limits.heap_address_space_size
        {
            bail!("spec over limits");
        }

        if runtime_spec.heap.initial_size as usize > limits.heap_memory_size {
            bail!("spec over limits");
        }

        if runtime_spec.globals.len() * mem::size_of::<u64>() > limits.globals_size {
            bail!("globals exceed limits");
        }

        for (ptr, len) in [
            // make the heap read/writable and record its initial size
            (slot.heap, runtime_spec.heap.initial_size as usize),
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

        let inst_ptr = slot.start as *mut Instance;

        // upgrade the slot's weak region pointer so the region can't get dropped while the instance
        // exists
        let region = slot
            .region
            .upgrade()
            .ok_or(format_err!("backing region of slot has been dropped"))?;

        let alloc = Alloc {
            heap_accessible_size: runtime_spec.heap.initial_size as usize,
            heap_inaccessible_size: slot.limits.heap_address_space_size,
            runtime_spec: runtime_spec.clone(),
            slot: Some(slot),
            region,
        };

        let inst = new_instance_handle(inst_ptr, module, alloc, embed_ctx)?;

        Ok(inst)
    }
}

impl RegionInternal for MmapRegion {
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
            (slot.heap, slot.limits.heap_address_space_size),
            (slot.stack, slot.limits.stack_size),
            (slot.globals, slot.limits.globals_size),
            (slot.sigstack, SIGSTKSZ),
        ]
        .into_iter()
        {
            // eprintln!("setting none {:p}[{:x}]", *ptr, len);
            unsafe {
                mprotect(*ptr, *len, ProtFlags::PROT_NONE).expect("mprotect succeeds during drop");
                madvise(*ptr, *len, MmapAdvise::MADV_DONTNEED)
                    .expect("madvise succeeds during drop");
            }
        }

        self.freelist.lock().unwrap().push(slot);
    }

    fn expand_heap(&self, alloc: &mut Alloc, expand_bytes: u32) -> Result<u32, Error> {
        let slot = alloc.slot();

        if expand_bytes == 0 {
            // no expansion takes place, which is not an error
            return Ok(alloc.heap_accessible_size as u32);
        }

        let host_page_size = host_page_size() as u32;

        if alloc.heap_accessible_size as u32 % host_page_size != 0 {
            panic!("heap is not page-aligned");
        }

        if expand_bytes > std::u32::MAX - host_page_size - 1 {
            bail!("expanded heap would overflow address space");
        }

        // round the expansion up to a page boundary
        let expand_pagealigned =
            ((expand_bytes + host_page_size - 1) / host_page_size) * host_page_size;

        // `heap_inaccessible_size` tracks the size of the allocation that is addressible but not
        // accessible. We cannot perform an expansion larger than this size.
        if expand_pagealigned as usize > alloc.heap_inaccessible_size {
            bail!("expanded heap would overflow addressable memory");
        }

        // the above makes sure this expression does not underflow
        let guard_remaining = alloc.heap_inaccessible_size - expand_pagealigned as usize;

        let rt_spec = &alloc.runtime_spec;

        // The compiler specifies how much guard (memory which traps on access) must be beyond the
        // end of the accessible memory. We cannot perform an expansion that would make this region
        // smaller than the compiler expected it to be.
        if guard_remaining < rt_spec.heap.guard_size as usize {
            bail!("expansion would leave guard memory too small");
        }

        // The compiler indicates that the module has specified a maximum memory size. Don't let
        // the heap expand beyond that:
        if rt_spec.heap.max_size_valid == 1
            && alloc.heap_accessible_size + expand_pagealigned as usize
                > rt_spec.heap.max_size as usize
        {
            bail!("expansion would exceed compiler-specified heap limit");
        }

        // The runtime sets a limit on how much of the heap can be backed by real memory. Don't let
        // the heap expand beyond that:
        if alloc.heap_accessible_size + expand_pagealigned as usize > slot.limits.heap_memory_size {
            bail!("expansion would exceed runtime-specified heap limit");
        }

        let newly_accessible = alloc.heap_accessible_size;
        unsafe {
            mprotect(
                (slot.heap as usize + newly_accessible) as *mut c_void,
                expand_pagealigned as usize,
                ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
            )?
        };
        alloc.heap_accessible_size += expand_pagealigned as usize;
        alloc.heap_inaccessible_size -= expand_pagealigned as usize;

        Ok(newly_accessible as u32)
    }

    fn reset_heap(&self, alloc: &mut Alloc, module: &dyn Module) -> Result<(), Error> {
        let heap_spec = &alloc.runtime_spec.heap;
        let initial_size = heap_spec.initial_size as usize;

        // reset the heap to the initial size
        if alloc.heap_accessible_size != initial_size {
            alloc.heap_accessible_size = initial_size;
            alloc.heap_inaccessible_size =
                alloc.slot().limits.heap_address_space_size - initial_size;

            // turn off any extra pages
            let acc_heap_end =
                (alloc.slot().heap as usize + alloc.heap_accessible_size) as *mut c_void;
            unsafe {
                mprotect(
                    acc_heap_end,
                    alloc.heap_inaccessible_size,
                    ProtFlags::PROT_NONE,
                )?;
                madvise(
                    acc_heap_end,
                    alloc.heap_inaccessible_size,
                    MmapAdvise::MADV_DONTNEED,
                )?;
            }
        }

        // Initialize the heap using the module sparse page data. There cannot be more pages in the
        // sparse page data than will fit in the initial heap size.
        //
        // Pages with a corresponding non-null entry in the sparse page data are initialized with
        // the contents of that data.
        //
        // Any pages which don't have an entry in the sparse page data, either because their entry
        // is NULL, or because the sparse data has fewer pages than the initial heap, are zeroed.
        let sparse_page_data = module.sparse_page_data()?;
        let heap = unsafe { alloc.heap_mut() };
        let initial_pages = initial_size
            .checked_div(host_page_size())
            .ok_or(format_err!(
                "initial heap size must be divisible by host page size"
            ))?;
        for page_num in 0..initial_pages {
            let page_base = page_num * host_page_size();
            if heap.len() < page_base {
                bail!("sparse page data exceeded initial heap size");
            }
            let contents_ptr = sparse_page_data.get(page_num).unwrap_or(&ptr::null());
            if contents_ptr.is_null() {
                // zero this page
                for b in heap[page_base..page_base + host_page_size()].iter_mut() {
                    *b = 0x00;
                }
            } else {
                // otherwise copy in the page data
                let contents = unsafe {
                    std::slice::from_raw_parts(*contents_ptr as *const u8, host_page_size())
                };
                heap[page_base..page_base + host_page_size()].copy_from_slice(contents);
            }
        }

        Ok(())
    }
}

impl Drop for MmapRegion {
    fn drop(&mut self) {
        for slot in self.freelist.get_mut().unwrap().drain(0..) {
            Self::free_slot(slot);
        }
    }
}

impl MmapRegion {
    pub fn create(num_slots: usize, limits: &Limits) -> Result<Arc<Self>, Error> {
        assert!(
            SIGSTKSZ % host_page_size() == 0,
            "signal stack size is a multiple of host page size"
        );
        if !limits.validate() {
            bail!("invalid limits");
        }

        let region = Arc::new(MmapRegion {
            freelist: Mutex::new(Vec::with_capacity(num_slots)),
            limits: limits.clone(),
        });
        {
            let mut freelist = region.freelist.lock().unwrap();
            for _ in 0..num_slots {
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
                MapFlags::MAP_ANONYMOUS | MapFlags::MAP_PRIVATE,
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
pub unsafe fn mprotect(
    addr: *mut c_void,
    length: libc::size_t,
    prot: ProtFlags,
) -> nix::Result<()> {
    nix::errno::Errno::result(libc::mprotect(addr, length, prot.bits())).map(drop)
}
