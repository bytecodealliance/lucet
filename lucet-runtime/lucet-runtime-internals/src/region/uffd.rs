use super::NewInstanceArgs;
use crate::alloc::{instance_heap_offset, AddrLocation, Alloc, Limits, Slot};
use crate::error::Error;
use crate::instance::{new_instance_handle, Instance, InstanceHandle, InstanceInternal};
use crate::module::Module;
use crate::region::{Region, RegionCreate, RegionInternal};
use crate::sysdeps::host_page_size;
use crate::WASM_PAGE_SIZE;
use crate::{lucet_bail, lucet_ensure, lucet_format_err};
use libc::c_void;
use nix::poll;
use nix::sys::mman::{madvise, mmap, mprotect, munmap, MapFlags, MmapAdvise, ProtFlags};
use std::os::unix::io::{AsRawFd, RawFd};
use std::ptr;
use std::sync::{Arc, Mutex, Weak};
use std::thread::{self, JoinHandle};
use userfaultfd::{Event, IoctlFlags, Uffd, UffdBuilder};

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
    config: UffdConfig,
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

fn wake_invalid_access(
    inst: &mut Instance,
    uffd: &Uffd,
    page_addr: usize,
    page_size: usize,
) -> Result<(), Error> {
    // Set the protection to NONE to induce a SIGBUS for the access on the next retry
    unsafe {
        mprotect(page_addr as _, page_size, ProtFlags::PROT_NONE)
            .map_err(|e| Error::InternalError(e.into()))?;
    }

    inst.alloc_mut().invalid_pages.push((page_addr, page_size));

    uffd.wake(page_addr as _, page_size)
        .map_err(|e| Error::InternalError(e.into()))?;

    Ok(())
}

fn reset_invalid_pages(alloc: &mut Alloc) -> Result<(), Error> {
    // Reset the protection level for invalid page accesses
    for (addr, len) in alloc.invalid_pages.drain(..) {
        unsafe {
            mprotect(addr as _, len, ProtFlags::PROT_READ | ProtFlags::PROT_WRITE)
                .map_err(|e| Error::InternalError(e.into()))?;
        }
    }

    Ok(())
}

fn page_fault_handler(
    fault_addr: usize,
    config: &UffdConfig,
    uffd: &Uffd,
    limits: &Limits,
    region_start: usize,
    instance_capacity: usize,
) -> Result<(), Error> {
    let fault_page = fault_addr - (fault_addr % host_page_size());
    let instance_size = limits.total_memory_size();

    let in_region =
        fault_addr >= region_start && fault_addr < region_start + instance_size * instance_capacity;
    if !in_region {
        lucet_bail!(
            "fault addr {} outside uffd region {}[{}]",
            fault_addr,
            region_start,
            instance_size * instance_capacity,
        );
    }

    let fault_offs = fault_addr - region_start;
    let fault_base = fault_offs - (fault_offs % instance_size);
    let inst_base = region_start + fault_base;

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
        lucet_bail!(
            "instance magic incorrect: fault address {:p}, region {:p}[{}]",
            fault_addr as *mut c_void,
            region_start as *mut c_void,
            instance_size * instance_capacity
        );
    }

    let loc = inst.alloc().addr_location(fault_addr as *const c_void);
    match loc {
        AddrLocation::InaccessibleHeap | AddrLocation::StackGuard => {
            // page fault occurred out of bounds; trigger a fault by waking the faulting
            // thread without copying or zeroing
            wake_invalid_access(inst, &uffd, fault_page as _, host_page_size())?;
        }
        AddrLocation::SigStackGuard | AddrLocation::Unknown => {
            tracing::error!("UFFD pagefault at fatal location: {:?}", loc);
            wake_invalid_access(inst, &uffd, fault_page as _, host_page_size())?;
        }
        AddrLocation::Globals | AddrLocation::SigStack => {
            tracing::error!("UFFD pagefault at unexpected location: {:?}", loc);
            wake_invalid_access(inst, &uffd, fault_page as _, host_page_size())?;
        }
        AddrLocation::Stack => match config.stack_init {
            Disposition::Lazy => unsafe {
                zeropage(
                    uffd,
                    fault_page as *mut c_void,
                    host_page_size(),
                    true,
                    config.enoent_retry_limit,
                )
                .map_err(|e| Error::InternalError(e.into()))?;
            },
            Disposition::Eager => {
                lucet_bail!("fault in eagerly initialized stack should be unreachable")
            }
        },

        AddrLocation::Heap => match config.heap_page_size {
            HeapPageSize::Host => HostPageSizedUffdStrategy.heap_fault(
                config,
                &uffd,
                inst.module(),
                inst.alloc(),
                fault_page as *mut c_void,
            )?,
            HeapPageSize::Wasm => WasmPageSizedUffdStrategy.heap_fault(
                config,
                &uffd,
                inst.module(),
                inst.alloc(),
                fault_page as *mut c_void,
            )?,
        },
    }
    Ok(())
}

fn uffd_worker_thread(
    config: UffdConfig,
    uffd: Arc<Uffd>,
    region_start: *mut c_void,
    instance_capacity: usize,
    handler_pipe: RawFd,
    limits: Limits,
) -> Result<(), Error> {
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

        match uffd.read_event() {
            Err(e) => lucet_bail!("error reading event from uffd: {}", e),
            Ok(None) => lucet_bail!("uffd had POLLIN set, but could not be read"),
            Ok(Some(Event::Pagefault { addr, .. })) => page_fault_handler(
                addr as usize,
                &config,
                &uffd,
                &limits,
                region_start as usize,
                instance_capacity,
            )?,
            Ok(Some(e)) => lucet_bail!("unexpected uffd event: {:?}", e),
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

unsafe fn zeropage(
    uffd: &Uffd,
    start: *mut c_void,
    len: usize,
    wake: bool,
    retry_limit: usize,
) -> userfaultfd::Result<usize> {
    let mut retries = 0;

    loop {
        let res = uffd.zeropage(start, len, wake);
        if let Err(userfaultfd::Error::ZeropageFailed(nix::errno::Errno::ENOENT)) = res {
            retries += 1;
            if retries >= retry_limit {
                return res;
            }
        } else {
            return res;
        }
    }
}

unsafe fn copy(
    uffd: &Uffd,
    src: *const c_void,
    dst: *mut c_void,
    len: usize,
    wake: bool,
    retry_limit: usize,
) -> userfaultfd::Result<usize> {
    let mut retries = 0;

    loop {
        let res = uffd.copy(src, dst, len, wake);
        if let Err(userfaultfd::Error::CopyFailed(nix::errno::Errno::ENOENT)) = res {
            retries += 1;
            if retries >= retry_limit {
                return res;
            }
        } else {
            return res;
        }
    }
}

impl RegionInternal for UffdRegion {
    fn new_instance_with(
        &self,
        NewInstanceArgs {
            module,
            embed_ctx,
            heap_memory_size_limit,
            mut alloc_strategy,
            terminate_on_heap_oom,
            ..
        }: NewInstanceArgs,
    ) -> Result<InstanceHandle, Error> {
        let limits = self.get_limits();
        module.validate_runtime_spec(&limits, heap_memory_size_limit)?;

        // Use the supplied alloc_strategy to get the next available slot
        // for this new instance.
        let slot;
        {
            let mut free_slot_vector = self.freelist.lock().unwrap();
            let slot_index = alloc_strategy.next(free_slot_vector.len(), self.capacity())?;
            slot = free_slot_vector.swap_remove(slot_index);
        }

        assert_eq!(
            slot.heap as usize % host_page_size(),
            0,
            "heap must be page-aligned"
        );

        let mut init_list = vec![
            (slot.globals, limits.globals_size),
            (slot.sigstack, limits.signal_stack_size),
        ];
        if let Disposition::Eager = self.config.stack_init {
            init_list.push((slot.stack, limits.stack_size));
        }

        for (ptr, len) in init_list.into_iter() {
            // globals_size = 0 is valid, but the ioctl fails if you pass it 0
            if len > 0 {
                unsafe {
                    zeropage(&self.uffd, ptr, len, true, self.config.enoent_retry_limit)
                        .expect("zeropage succeeds");
                }
            }
        }

        // if heap_init is eager and module has a linear memory, initialize it:
        if let Disposition::Eager = self.config.heap_init {
            if let Some(heap_spec) = module.heap_spec() {
                let initial_size = heap_spec.initial_size as usize;
                assert_eq!(
                    initial_size % host_page_size(),
                    0,
                    "initial heap is page divisible"
                );
                // All associated host pages gets zeroed
                unsafe {
                    zeropage(
                        &self.uffd,
                        slot.heap,
                        initial_size,
                        false,
                        self.config.enoent_retry_limit,
                    )
                    .map_err(|e| Error::InternalError(e.into()))?;
                }

                let heap_pages = initial_size / host_page_size();
                for pages_into_heap in 0..heap_pages {
                    if let Some(init_contents) = module.get_sparse_page_data(pages_into_heap) {
                        let page_addr = slot.heap as usize + pages_into_heap * host_page_size();
                        let page: &mut [u8] = unsafe {
                            std::slice::from_raw_parts_mut(
                                page_addr as *mut c_void as *mut u8,
                                host_page_size(),
                            )
                        };
                        page.copy_from_slice(init_contents)
                    }
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
            invalid_pages: Vec::new(),
        };

        let mut inst = new_instance_handle(inst_ptr, module, alloc, embed_ctx)?;
        inst.set_terminate_on_heap_oom(terminate_on_heap_oom);

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

        // Reset the protection level for invalid page accesses
        reset_invalid_pages(alloc).expect("invalid pages are reset during drop");

        // set dontneed for everything past the `Instance` page
        let ptr = (slot.start as usize + instance_heap_offset()) as *mut c_void;
        let len = slot.limits.total_memory_size() - instance_heap_offset();
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
        // Reset the protection level for invalid page accesses
        reset_invalid_pages(alloc)?;

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
        UffdRegion::create(instance_capacity, limits, UffdConfig::default())
    }
}

impl Drop for UffdRegion {
    fn drop(&mut self) {
        // write to the pipe to notify the handler to exit
        if let Err(e) = nix::unistd::write(self.handler_pipe, b"macht nichts") {
            // this probably means the handler errored out; note it but don't panic
            tracing::error!("couldn't write to handler shutdown pipe: {}", e);
        };

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
    pub fn create(
        instance_capacity: usize,
        limits: &Limits,
        config: UffdConfig,
    ) -> Result<Arc<Self>, Error> {
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
        let handler_config = config.clone();
        let abort_on_handler_error = config.abort_on_handler_error;
        let handler_start = start as usize;
        let handler_limits = limits.clone();
        let handler = thread::Builder::new()
            .name("uffd region handler".into())
            .spawn(move || {
                let res = uffd_worker_thread(
                    handler_config,
                    handler_uffd.clone(),
                    handler_start as *mut c_void,
                    instance_capacity,
                    handler_pipe_recv,
                    handler_limits,
                );
                // clean up the shutdown pipe before terminating
                if let Err(e) = nix::unistd::close(handler_pipe_recv) {
                    // note but don't return an error just for the pipe
                    tracing::error!("error closing handler_pipe_recv: {}", e);
                }
                match res {
                    Err(e) => {
                        if abort_on_handler_error {
                            tracing::error!("aborting due to error on uffd handler thread: {}", e);
                            std::process::abort();
                        }
                        // We can't currently recover from something going wrong in the handler thread,
                        // so we unregister the region and wake all faulting threads so that they crash
                        // rather than hanging.
                        handler_uffd
                            .unregister(handler_start as *mut c_void, total_region_size)
                            .unwrap_or_else(|e| {
                                tracing::error!("error while unregistering in error case: {}", e)
                            });
                        handler_uffd
                            .wake(handler_start as *mut c_void, total_region_size)
                            .unwrap_or_else(|e| {
                                tracing::error!("error while waking in error case: {}", e)
                            });
                        Err(e)
                    }
                    Ok(()) => Ok(()),
                }
            })
            .expect("error spawning uffd region handler");

        let region = Arc::new(UffdRegion {
            config,
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
        unsafe {
            zeropage(
                &region.uffd,
                start,
                host_page_size(),
                true,
                region.config.enoent_retry_limit,
            )
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Disposition {
    Eager,
    Lazy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeapPageSize {
    Host,
    Wasm,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UffdConfig {
    pub heap_page_size: HeapPageSize,
    pub heap_init: Disposition,
    pub stack_init: Disposition,
    pub abort_on_handler_error: bool,
    pub enoent_retry_limit: usize,
}

impl Default for UffdConfig {
    fn default() -> UffdConfig {
        UffdConfig {
            heap_page_size: HeapPageSize::Wasm,
            heap_init: Disposition::Lazy,
            stack_init: Disposition::Lazy,
            abort_on_handler_error: false,
            enoent_retry_limit: 4,
        }
    }
}

pub trait UffdStrategy: Send + Sync + 'static {
    fn stack_fault(
        &self,
        config: &UffdConfig,
        uffd: &Uffd,
        fault_page: *mut c_void,
    ) -> Result<(), Error>;
    fn heap_fault(
        &self,
        config: &UffdConfig,
        uffd: &Uffd,
        module: &dyn Module,
        alloc: &Alloc,
        fault_page: *mut c_void,
    ) -> Result<(), Error>;
}

pub struct HostPageSizedUffdStrategy;

impl UffdStrategy for HostPageSizedUffdStrategy {
    fn stack_fault(
        &self,
        config: &UffdConfig,
        uffd: &Uffd,
        fault_page: *mut c_void,
    ) -> Result<(), Error> {
        unsafe {
            zeropage(
                uffd,
                fault_page as *mut c_void,
                host_page_size(),
                true,
                config.enoent_retry_limit,
            )
            .map_err(|e| Error::InternalError(e.into()))?;
        }
        Ok(())
    }

    fn heap_fault(
        &self,
        config: &UffdConfig,
        uffd: &Uffd,
        module: &dyn Module,
        alloc: &Alloc,
        fault_page: *mut c_void,
    ) -> Result<(), Error> {
        let pages_into_heap = (fault_page as usize - alloc.slot().heap as usize) / host_page_size();

        // page fault occurred in the heap; copy or zero
        if let Some(page) = module.get_sparse_page_data(pages_into_heap) {
            // we are in the sparse data area, with a non-empty page; copy it in
            unsafe {
                copy(
                    uffd,
                    page.as_ptr() as *const c_void,
                    fault_page,
                    host_page_size(),
                    true,
                    config.enoent_retry_limit,
                )
                .map_err(|e| Error::InternalError(e.into()))?;
            }
        } else {
            // else if outside the sparse data area, or with an empty page
            unsafe {
                zeropage(
                    uffd,
                    fault_page,
                    host_page_size(),
                    true,
                    config.enoent_retry_limit,
                )
                .map_err(|e| Error::InternalError(e.into()))?;
            }
        }
        Ok(())
    }
}

pub struct WasmPageSizedUffdStrategy;

impl UffdStrategy for WasmPageSizedUffdStrategy {
    fn stack_fault(
        &self,
        config: &UffdConfig,
        uffd: &Uffd,
        fault_page: *mut c_void,
    ) -> Result<(), Error> {
        unsafe {
            zeropage(
                uffd,
                fault_page,
                host_page_size(),
                true,
                config.enoent_retry_limit,
            )
            .map_err(|e| Error::InternalError(e.into()))?;
        }
        Ok(())
    }

    fn heap_fault(
        &self,
        config: &UffdConfig,
        uffd: &Uffd,
        module: &dyn Module,
        alloc: &Alloc,
        fault_page: *mut c_void,
    ) -> Result<(), Error> {
        let slot = alloc.slot.as_ref().unwrap();
        // Find the address of the fault relative to the heap base
        let rel_fault_addr = fault_page as usize - slot.heap as usize;
        // Find the base of the wasm page, relative to the heap start
        let rel_wasm_page_base_addr = rel_fault_addr - (rel_fault_addr % WASM_PAGE_SIZE as usize);
        // Find the absolute address of the base of the wasm page
        let wasm_page_base_addr = slot.heap as usize + rel_wasm_page_base_addr;
        // Find the number of host pages into the heap the wasm page base begins at
        let base_pages_into_heap = rel_wasm_page_base_addr / host_page_size();

        assert!(WASM_PAGE_SIZE as usize > host_page_size());
        assert_eq!(WASM_PAGE_SIZE as usize % host_page_size(), 0);

        let host_pages_per_wasm_page = WASM_PAGE_SIZE as usize / host_page_size();

        for page_num in 0..host_pages_per_wasm_page {
            let pages_into_heap = base_pages_into_heap + page_num;
            let host_page_addr = wasm_page_base_addr + (page_num * host_page_size());

            if alloc.addr_location(host_page_addr as *const c_void) != AddrLocation::Heap {
                tracing::error!("Heap ended earlier than expected.");
                break;
            }

            // page fault occurred in the heap; copy or zero
            if let Some(page) = module.get_sparse_page_data(pages_into_heap) {
                // we are in the sparse data area, with a non-empty page; copy it in
                unsafe {
                    copy(
                        uffd,
                        page.as_ptr() as *const c_void,
                        host_page_addr as *mut c_void,
                        host_page_size(),
                        false,
                        config.enoent_retry_limit,
                    )
                    .map_err(|e| Error::InternalError(e.into()))?;
                }
            } else {
                // else if outside the sparse data area, or with an empty page
                unsafe {
                    zeropage(
                        uffd,
                        host_page_addr as *mut c_void,
                        host_page_size(),
                        false,
                        config.enoent_retry_limit,
                    )
                    .map_err(|e| Error::InternalError(e.into()))?;
                }
            }
        }

        uffd.wake(wasm_page_base_addr as *mut c_void, WASM_PAGE_SIZE as usize)
            .map_err(|e| Error::InternalError(e.into()))?;

        Ok(())
    }
}
