use crate::alloc::{host_page_size, instance_heap_offset, AddrLocation, Alloc, Limits, Slot};
use crate::embed_ctx::CtxMap;
use crate::error::Error;
use crate::instance::{new_instance_handle, Instance, InstanceHandle, InstanceInternal};
use crate::module::Module;
use crate::region::{Region, RegionCreate, RegionInternal};
use crate::{lucet_bail, lucet_ensure, lucet_format_err};
use libc::c_void;
use nix::poll;
use nix::sys::mman::{madvise, mmap, munmap, MapFlags, MmapAdvise, ProtFlags};
use std::os::unix::io::{AsRawFd, RawFd};
use std::ptr;
use std::sync::{Arc, Mutex, Weak};
use std::thread::{self, JoinHandle};
use userfaultfd::{IoctlFlags, Uffd, UffdBuilder};

/// A [`Region`](trait.Region.html) backed by `mmap` and managed by `userfaultfd`.
///
/// Much like [`MmapRegion`](struct.MmapRegion.html) `UffdRegion` lays out virtual memory in a
/// contiguous block. See [`MmapRegion`](struct.MmapRegion.html) for details of the memory layout.
///
/// The difference is that `UffdRegion` is lazy. Only the minimum required physical memory is set up
/// to back that virtual memory before an `Instance` begins running. The stack and the heap are both
/// lazily allocated at runtime.
///
/// That lazy allocation is handled by the [`userfaultfd`][userfaultfd] system, using extensions
/// available in Linux version 4.11 or newer. The entire `Region` is registered with `userfaultfd`
/// handle.  When page faults occur due to attempts by the guest to access the lazy memory, the
/// guest thread is paused and a message is sent over the `userfaultfd` handle.
///
/// That message is picked up a separate thread which has the job of handling page faults. How it is
/// handled is dependent on where the page fault occurred. In the case where it occurs in the stack,
/// we just zero out the page. In the case it occurs in the heap, it is handled differently
/// depending on whether the page should contain data defined in the WebAssembly module. In the case
/// it should be blank we again just zero it out. In the case that it should contain data, we copy
/// the data into the page. In any case we finish by reawakening the guest thread.
///
/// If the fault occurs in a guard page, we do nothing, and reawaken the thread without allocating
/// the backing physical memory. This ends up causing the guest thread to raise a SIGBUS, which is
/// treated as a fatal error by the Lucet signal handler.
///
/// [userfaultfd]: http://man7.org/linux/man-pages/man2/userfaultfd.2.html
pub struct UffdRegion {
    uffd: Arc<Uffd>,
    start: *mut c_void,
    limits: Limits,
    freelist: Mutex<Vec<Slot>>,
    instance_capacity: usize,
    handler: Option<JoinHandle<Result<(), Error>>>,
    handler_pipe: RawFd,
}

// the start pointer prevents these from auto-deriving
unsafe impl Send for UffdRegion {}
unsafe impl Sync for UffdRegion {}

fn uffd_handler(
    uffd: Arc<Uffd>,
    start: *mut c_void,
    instance_capacity: usize,
    handler_pipe: RawFd,
    limits: Limits,
) -> Result<(), Error> {
    use userfaultfd::Event;

    let mut pollfds = [
        poll::PollFd::new(uffd.as_raw_fd(), poll::PollFlags::POLLIN),
        poll::PollFd::new(handler_pipe, poll::PollFlags::POLLIN),
    ];

    loop {
        let poll_res = poll::poll(&mut pollfds, 500)?;
        let uffd_pfd = pollfds[0];
        let pipe_pfd = pollfds[1];

        if poll_res == 0 {
            // we set a timeout on the poll in case the main thread panics, so the handler doesn't
            // run forever; just run the loop again
            continue;
        }

        // reading anything from the handler pipe kills this thread
        if let Some(ev) = pipe_pfd.revents() {
            lucet_ensure!(!ev.contains(poll::PollFlags::POLLERR), "pipe event error");
            if ev.contains(poll::PollFlags::POLLIN) {
                break;
            }
        }

        if let Some(ev) = uffd_pfd.revents() {
            lucet_ensure!(
                !ev.contains(poll::PollFlags::POLLERR) && ev.contains(poll::PollFlags::POLLIN),
                "unexpected uffd event flags: {:?}",
                ev
            );
        }

        // eprintln!("handling a fault on fd {}", uffd.as_raw_fd());

        match uffd.read_event() {
            Err(e) => lucet_bail!("error reading event from uffd: {}", e),
            Ok(None) => lucet_bail!("uffd had POLLIN set, but could not be read"),
            Ok(Some(Event::Pagefault {
                addr: fault_addr, ..
            })) => {
                // eprintln!("fd {} fault address: {:p}", uffd.as_raw_fd(), fault_addr);
                let fault_addr = fault_addr as usize;
                let fault_page = fault_addr - (fault_addr % host_page_size());
                let instance_size = limits.total_memory_size();

                let in_region = fault_addr >= start as usize
                    && fault_addr < start as usize + instance_size * instance_capacity;
                lucet_ensure!(in_region, "fault is within the uffd region");

                let fault_offs = fault_addr - start as usize;
                let fault_base = fault_offs - (fault_offs % instance_size);
                let inst_base = start as usize + fault_base;

                // NB: we are blatantly lying to the compiler here! the lifetime is *not* actually
                // static, but for the purposes of reaching in to read the sparse page data and the
                // heap layout, we can treat it as such. The important property to maintain is that
                // the *real* region lifetime (`'r`) lives at least as long as this handler thread,
                // which can be shown by examining the `drop` method of `UffdRegion`.

                let inst: &mut Instance = unsafe {
                    (inst_base as *mut Instance)
                        .as_mut()
                        .ok_or(lucet_format_err!("instance pointer is non-null"))?
                };
                if !inst.valid_magic() {
                    eprintln!(
                        "instance magic incorrect, fault address {:p}",
                        fault_addr as *mut c_void
                    );
                    lucet_bail!("instance magic incorrect");
                }

                let loc = inst.alloc().addr_location(fault_addr as *const c_void);
                match loc {
                    AddrLocation::InaccessibleHeap | AddrLocation::StackGuard => {
                        // eprintln!("fault in heap guard!");
                        // page fault occurred out of bounds; trigger a fault by waking the faulting
                        // thread without copying or zeroing
                        uffd.wake(fault_page as *mut c_void, host_page_size())
                            .map_err(|e| Error::InternalError(e.into()))?;
                    }
                    AddrLocation::SigStackGuard | AddrLocation::Unknown => {
                        tracing::error!("UFFD pagefault at fatal location: {:?}", loc);
                        uffd.wake(fault_page as *mut c_void, host_page_size())
                            .map_err(|e| Error::InternalError(e.into()))?;
                    }
                    AddrLocation::Globals | AddrLocation::SigStack => {
                        tracing::error!("UFFD pagefault at unexpected location: {:?}", loc);
                        uffd.wake(fault_page as *mut c_void, host_page_size())
                            .map_err(|e| Error::InternalError(e.into()))?;
                    }
                    AddrLocation::Stack => unsafe {
                        uffd.zeropage(fault_page as *mut c_void, host_page_size(), true)
                            .map_err(|e| Error::InternalError(e.into()))?;
                    },
                    AddrLocation::Heap => {
                        // page fault occurred in the heap; copy or zero
                        let pages_into_heap =
                            (fault_page - inst.alloc().slot().heap as usize) / host_page_size();
                        if let Some(page) = inst.module().get_sparse_page_data(pages_into_heap) {
                            // we are in the sparse data area, with a non-empty page; copy it in
                            unsafe {
                                uffd.copy(
                                    page.as_ptr() as *const c_void,
                                    fault_page as *mut c_void,
                                    host_page_size(),
                                    true,
                                )
                                .map_err(|e| Error::InternalError(e.into()))?;
                            }
                        } else {
                            // else if outside the sparse data area, or with an empty page
                            // eprintln!("zeroing a page at {:p}", fault_page as *mut c_void);
                            unsafe {
                                uffd.zeropage(fault_page as *mut c_void, host_page_size(), true)
                                    .map_err(|e| Error::InternalError(e.into()))?;
                            }
                        }
                    }
                }
            }
            Ok(Some(ev)) => panic!("unexpected uffd event: {:?}", ev),
        }
    }

    Ok(())
}

impl Region for UffdRegion {
    fn free_slots(&self) -> usize {
        self.freelist.lock().unwrap().len()
    }

    fn used_slots(&self) -> usize {
        self.capacity() - self.free_slots()
    }

    fn capacity(&self) -> usize {
        self.instance_capacity
    }
}

impl RegionInternal for UffdRegion {
    fn new_instance_with(
        &self,
        module: Arc<dyn Module>,
        embed_ctx: CtxMap,
        heap_memory_size_limit: usize,
    ) -> Result<InstanceHandle, Error> {
        let limits = self.get_limits();
        module.validate_runtime_spec(&limits, heap_memory_size_limit)?;

        let slot = self
            .freelist
            .lock()
            .unwrap()
            .pop()
            .ok_or(Error::RegionFull(self.instance_capacity))?;

        assert_eq!(
            slot.heap as usize % host_page_size(),
            0,
            "heap must be page-aligned"
        );

        for (ptr, len) in [
            // zero the globals
            (slot.globals, limits.globals_size),
            // zero the sigstack
            (slot.sigstack, limits.signal_stack_size),
        ]
        .into_iter()
        {
            // globals_size = 0 is valid, but the ioctl fails if you pass it 0
            if *len > 0 {
                // eprintln!("zeroing {:p}[{:x}]", *ptr, len);
                unsafe {
                    self.uffd
                        .zeropage(*ptr, *len, true)
                        .expect("uffd.zeropage succeeds");
                }
            }
        }

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
            heap_accessible_size: module
                .heap_spec()
                .map(|h| h.initial_size as usize)
                .unwrap_or(0),
            heap_inaccessible_size: slot.limits.heap_address_space_size,
            heap_memory_size_limit,
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

        // set dontneed for everything past the `Instance` page
        let ptr = (slot.start as usize + instance_heap_offset()) as *mut c_void;
        let len = slot.limits.total_memory_size() - instance_heap_offset();
        // eprintln!("setting none {:p}[{:x}]", ptr, len);
        unsafe {
            madvise(ptr, len, MmapAdvise::MADV_DONTNEED).expect("madvise succeeds during drop");
        }

        self.freelist.lock().unwrap().push(slot);
    }

    fn expand_heap(&self, _slot: &Slot, _start: u32, _len: u32) -> Result<(), Error> {
        // the actual work of heap expansion for UFFD is done in the worker thread; we just need the
        // `Alloc` to validate the new limits and update the metadata
        Ok(())
    }

    fn reset_heap(&self, alloc: &mut Alloc, module: &dyn Module) -> Result<(), Error> {
        // zero the heap, if any of it is currently accessible
        if alloc.heap_accessible_size > 0 {
            unsafe {
                madvise(
                    alloc.slot().heap,
                    alloc.heap_accessible_size,
                    MmapAdvise::MADV_DONTNEED,
                )?;
            }
        }

        // reset the heap to the initial size
        let initial_size = module
            .heap_spec()
            .map(|h| h.initial_size as usize)
            .unwrap_or(0);
        alloc.heap_accessible_size = initial_size;
        alloc.heap_inaccessible_size = alloc.slot().limits.heap_address_space_size - initial_size;
        Ok(())
    }

    fn get_limits(&self) -> &Limits {
        &self.limits
    }

    fn as_dyn_internal(&self) -> &dyn RegionInternal {
        self
    }
}

impl RegionCreate for UffdRegion {
    const TYPE_NAME: &'static str = "UffdRegion";

    fn create(instance_capacity: usize, limits: &Limits) -> Result<Arc<Self>, Error> {
        UffdRegion::create(instance_capacity, limits)
    }
}

impl Drop for UffdRegion {
    fn drop(&mut self) {
        // eprintln!("UffdRegion::drop()");
        // write to the pipe to notify the handler to exit
        if let Err(e) = nix::unistd::write(self.handler_pipe, b"macht nichts") {
            // this probably means the handler errored out; note it but don't panic
            eprintln!("couldn't write to handler shutdown pipe: {}", e);
        };

        // eprintln!("joining");
        // wait for the handler to exit
        let res = self
            .handler
            .take()
            .expect("region has a join handle")
            .join()
            .expect("join on uffd handler");

        // close the send end of the pipe; the handler closes the other end
        nix::unistd::close(self.handler_pipe).expect("close handler exit pipe");

        let total_region_size = self.instance_capacity * self.limits.total_memory_size();
        unsafe {
            munmap(self.start, total_region_size).expect("unmapping region");
        }

        if let Err(e) = res {
            panic!("uffd handler thread failed: {}", e);
        }
    }
}

impl UffdRegion {
    /// Create a new `UffdRegion` that can support a given number of instances, each subject to the
    /// same runtime limits.
    ///
    /// The region is returned in an `Arc`, because any instances created from it carry a reference
    /// back to the region.
    ///
    /// This also creates and starts a separate thread that is responsible for handling page faults
    /// that occur within the memory region.
    pub fn create(instance_capacity: usize, limits: &Limits) -> Result<Arc<Self>, Error> {
        if instance_capacity == 0 {
            return Err(Error::InvalidArgument(
                "region must be able to hold at least one instance",
            ));
        }
        limits.validate()?;

        let uffd = Arc::new(
            UffdBuilder::new()
                .close_on_exec(true)
                .non_blocking(true)
                .create()
                .map_err(|e| Error::InternalError(e.into()))?,
        );

        // map the chunk of virtual memory for all of the slots
        let total_region_size =
            if let Some(sz) = instance_capacity.checked_mul(limits.total_memory_size()) {
                sz
            } else {
                return Err(Error::InvalidArgument("requested region size too large"));
            };
        let start = unsafe {
            mmap(
                ptr::null_mut(),
                total_region_size,
                ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
                MapFlags::MAP_ANONYMOUS | MapFlags::MAP_PRIVATE | MapFlags::MAP_NORESERVE,
                0,
                0,
            )?
        };

        // register the memory region with uffd and verify the required ioctls are supported
        let ioctls = uffd
            .register(start, total_region_size)
            .map_err(|e| Error::InternalError(e.into()))?;
        if !ioctls.contains(IoctlFlags::WAKE | IoctlFlags::COPY | IoctlFlags::ZEROPAGE) {
            panic!("required uffd ioctls not supported; found: {:?}", ioctls);
        }

        let (handler_pipe_recv, handler_pipe) = nix::unistd::pipe()?;

        let handler_uffd = uffd.clone();
        // morally equivalent to `unsafe impl Send`
        let handler_start = start as usize;
        let handler_limits = limits.clone();
        let handler = thread::Builder::new()
            .name("uffd region handler".into())
            .spawn(move || {
                let res = uffd_handler(
                    handler_uffd.clone(),
                    handler_start as *mut c_void,
                    instance_capacity,
                    handler_pipe_recv,
                    handler_limits,
                );
                // clean up the shutdown pipe before terminating
                if let Err(e) = nix::unistd::close(handler_pipe_recv) {
                    // note but don't return an error just for the pipe
                    eprintln!("error closing handler_pipe_recv: {}", e);
                }
                if res.is_err() {
                    // We can't currently recover from something going wrong in the handler thread,
                    // so we unregister the region and wake all faulting threads so that they crash
                    // rather than hanging. This is in lieu of bringing down the other threads with
                    // `panic!`
                    handler_uffd
                        .unregister(handler_start as *mut c_void, total_region_size)
                        .unwrap_or_else(|e| {
                            eprintln!("error while unregistering in error case: {}", e)
                        });
                    handler_uffd
                        .wake(handler_start as *mut c_void, total_region_size)
                        .unwrap_or_else(|e| eprintln!("error while waking in error case: {}", e));
                }
                res
            })
            .expect("error spawning uffd region handler");

        let region = Arc::new(UffdRegion {
            uffd,
            start,
            limits: limits.clone(),
            freelist: Mutex::new(Vec::with_capacity(instance_capacity)),
            instance_capacity,
            handler: Some(handler),
            handler_pipe,
        });

        {
            let mut freelist = region.freelist.lock().unwrap();
            for i in 0..instance_capacity {
                freelist.push(UffdRegion::create_slot(&region, i)?);
            }
        }

        Ok(region)
    }

    fn create_slot(region: &Arc<UffdRegion>, index: usize) -> Result<Slot, Error> {
        // get the memory from the offset into the overall region
        let start =
            (region.start as usize + (index * region.limits.total_memory_size())) as *mut c_void;
        // lay out the other sections in memory
        let heap = start as usize + instance_heap_offset();
        let stack = heap + region.limits.heap_address_space_size + host_page_size();
        let globals = stack + region.limits.stack_size;
        let sigstack = globals + region.limits.globals_size + host_page_size();

        // turn on the `Instance` page
        // eprintln!("zeroing {:p}[{:x}]", start, host_page_size());
        unsafe {
            region
                .uffd
                .zeropage(start, host_page_size(), true)
                .map_err(|e| Error::InternalError(e.into()))?;
        }

        Ok(Slot {
            start,
            heap: heap as *mut c_void,
            stack: stack as *mut c_void,
            globals: globals as *mut c_void,
            sigstack: sigstack as *mut c_void,
            limits: region.limits.clone(),
            region: Arc::downgrade(region) as Weak<dyn RegionInternal>,
        })
    }
}
