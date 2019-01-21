use crate::vmctx::Vmctx;
use lucet_sys::internal::{
    lucet_alloc, lucet_alloc_heap_spec, lucet_alloc_limits, lucet_alloc_runtime_spec,
    lucet_instance, lucet_instance_unsafe_ignore_current_lucet,
};
use lucet_sys::lucet_vmctx;
use std::os::raw::c_void;
use std::ptr;
use std::slice;

use nix::sys::mman::{mmap, munmap, MapFlags, ProtFlags};

fn make_limits(heap_memory_limit: usize) -> Box<lucet_alloc_limits> {
    Box::new(lucet_alloc_limits {
        heap_memory_size: heap_memory_limit as u64,
        heap_address_space_size: heap_memory_limit as u64,
        stack_size: 0,
        globals_size: 0,
    })
}

fn make_spec(heap_memory_limit: usize, initial_limit: usize) -> Box<lucet_alloc_runtime_spec> {
    let heap_spec = Box::new(lucet_alloc_heap_spec {
        reserved_size: heap_memory_limit as u64,
        guard_size: 0,
        initial_size: initial_limit as u64,
        max_size_valid: 0,
        max_size: 0,
    });

    Box::new(lucet_alloc_runtime_spec {
        heap: Box::leak(heap_spec),
        globals: ptr::null(),
    })
}

pub struct MockInstance {
    slab_ptr: *mut u8,
    slab_size: usize,
    alloc: *mut lucet_alloc,
}

impl MockInstance {
    pub fn new(
        initial_heap: &[u8],
        heap_memory_limit: usize,
        delegate: *const c_void,
    ) -> MockInstance {
        assert!(
            heap_memory_limit % 4096 == 0,
            "heap size must be a multiple of page size (4096)"
        );
        assert!(
            heap_memory_limit < ::std::u32::MAX as usize,
            "heap must fit in 32 bits"
        );

        // Round up to a wasm page size
        let initial_limit = {
            let ilen = initial_heap.len();
            const WASM_PAGE: usize = 64 * 1024;
            ((ilen + WASM_PAGE - 1) / WASM_PAGE) * WASM_PAGE // Next highest wasm page boundary
        };

        assert!(
            heap_memory_limit >= initial_limit,
            "initializer cannot be bigger than heap memory limit"
        );
        let slab_size = heap_memory_limit + 4096;

        // mmap operations will happen on the heap, so we need to create it with mmap.
        let slab_ptr = unsafe {
            mmap(
                ptr::null_mut(),
                slab_size,
                ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
                MapFlags::MAP_PRIVATE | MapFlags::MAP_ANONYMOUS,
                -1,
                0,
            )
            .expect("mmap slab with guard") as *mut u8
        };

        {
            let slab: &mut [u8] = unsafe { slice::from_raw_parts_mut(slab_ptr, slab_size) };
            let heap: &mut [u8] = &mut slab[4096..(4096 + initial_heap.len())];
            heap.copy_from_slice(initial_heap);
        }

        let instance: *mut lucet_instance = slab_ptr as *mut c_void as *mut _;

        let alloc = Box::new(lucet_alloc {
            start: slab_ptr as *mut i8,
            heap: unsafe { slab_ptr.offset(4096) as *mut i8 },
            heap_accessible_size: initial_limit,
            heap_inaccessible_size: heap_memory_limit - initial_limit,
            stack: ptr::null_mut(),
            globals: ptr::null_mut(),
            sigstack: ptr::null_mut(),
            limits: Box::leak(make_limits(heap_memory_limit)),
            spec: Box::leak(make_spec(heap_memory_limit, initial_limit)),
            region: ptr::null_mut(),
        });

        let alloc = Box::leak(alloc);

        unsafe {
            (*instance).delegate_obj = delegate as *mut c_void;
            (*instance).alloc = alloc;
        };

        unsafe {
            lucet_instance_unsafe_ignore_current_lucet(true);
        };

        MockInstance {
            slab_ptr,
            slab_size,
            alloc,
        }
    }

    pub fn vmctx(&self) -> Vmctx {
        let vmctx: *const lucet_vmctx = unsafe { self.slab_ptr.offset(4096) as *const _ };
        vmctx.into()
    }
}

impl Drop for MockInstance {
    fn drop(&mut self) {
        unsafe {
            Box::from_raw((*(*self.alloc).spec).heap as *mut lucet_alloc_heap_spec);
            Box::from_raw((*self.alloc).spec as *mut lucet_alloc_runtime_spec);
            Box::from_raw((*self.alloc).limits as *mut lucet_alloc_limits);
            Box::from_raw(self.alloc);

            munmap(self.slab_ptr as *mut ::libc::c_void, self.slab_size).expect("unmap slab");
        };
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn init_values() {
        let initializer = vec![1, 2, 3, 4, 0, 0, 1, 164];
        let inst = MockInstance::new(&initializer, 64 * 1024, ptr::null());
        let vmctx = inst.vmctx();
        let heap = vmctx.get_heap();
        for (offs, val) in initializer.iter().enumerate() {
            assert_eq!(val, heap.get(offs).unwrap());
        }
    }

    #[test]
    fn heap_size() {
        let initializer = vec![1, 2, 3, 4, 0, 0, 1, 164];
        let inst = MockInstance::new(&initializer, 64 * 1024, ptr::null());
        let vmctx = inst.vmctx();
        assert_eq!(vmctx.current_memory(), 1);
    }

    #[test]
    fn expand_memory() {
        let initializer = vec![1, 2, 3, 4, 0, 0, 1, 164];
        let inst = MockInstance::new(&initializer, 2 * 64 * 1024, ptr::null());
        let vmctx = inst.vmctx();
        assert_eq!(vmctx.current_memory(), 1);
        assert_eq!(vmctx.grow_memory(1), 1);
        assert_eq!(vmctx.current_memory(), 2);

        let heap = vmctx.get_heap();
        for (offs, val) in initializer.iter().enumerate() {
            assert_eq!(val, heap.get(offs).unwrap());
        }

        for offs in 64 * 1024..2 * 64 * 1024 {
            assert_eq!(0, *heap.get(offs).unwrap());
        }
    }
}
