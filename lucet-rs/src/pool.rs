use crate::errors::LucetError;
use crate::instance::Instance;
use crate::module::Module;
use libc::{c_int, INT_MAX};
use lucet_sys::*;
use std::ptr;
use xfailure::xbail;

pub struct Pool {
    pub(crate) lucet_pool: *mut lucet_pool,
}

unsafe impl Send for Pool {}
unsafe impl Sync for Pool {}

pub struct PoolBuilder {
    entries_: usize,
    limits: lucet_alloc_limits,
}

impl Pool {
    pub fn builder() -> PoolBuilder {
        let entries_ = 1000;
        let limits = lucet_alloc_limits {
            heap_memory_size: 16 * 64 * 1024,
            heap_address_space_size: 8 * 1024 * 1024,
            stack_size: 128 * 1024,
            globals_size: 4 * 1024,
        };
        PoolBuilder { entries_, limits }
    }

    pub fn instantiate(&self, module: &Module) -> Result<Instance, LucetError> {
        Instance::new(self, module)
    }

    pub fn page_size(&self) -> usize {
        LUCET_WASM_PAGE_SIZE as _
    }
}

impl Default for Pool {
    fn default() -> Self {
        Self::builder()
            .build()
            .expect("default PoolBuilder parameters are valid")
    }
}

impl Clone for Pool {
    fn clone(&self) -> Pool {
        unsafe { lucet_pool_incref(self.lucet_pool) };
        Pool {
            lucet_pool: self.lucet_pool,
        }
    }
}

impl Drop for Pool {
    fn drop(&mut self) {
        unsafe {
            lucet_pool_decref(self.lucet_pool);
            self.lucet_pool = ptr::null_mut();
        };
    }
}

impl PoolBuilder {
    pub fn entries(mut self, entries: usize) -> Self {
        self.entries_ = entries;
        self
    }

    pub fn heap_memory_size(mut self, heap_memory_size: u64) -> Self {
        self.limits.heap_memory_size = heap_memory_size;
        self
    }

    pub fn heap_address_space_size(mut self, heap_address_space_size: u64) -> Self {
        self.limits.heap_address_space_size = heap_address_space_size;
        self
    }

    pub fn stack_size(mut self, stack_size: u32) -> Self {
        self.limits.stack_size = stack_size;
        self
    }

    pub fn globals_size(mut self, globals_size: u32) -> Self {
        self.limits.globals_size = globals_size;
        self
    }

    pub fn build(self) -> Result<Pool, LucetError> {
        if self.entries_ == 0 || self.entries_ > INT_MAX as usize {
            xbail!(LucetError::UsageError("Unsupported number of entries"));
        }
        if self.limits.heap_memory_size % 4096 != 0 {
            xbail!(LucetError::UsageError(
                "Unsupported heap memory size: must be divisible by 4096"
            ));
        }
        if self.limits.heap_address_space_size % 4096 != 0 {
            xbail!(LucetError::UsageError(
                "Unsupported heap address space size: must be divisible by 4096"
            ));
        }
        if self.limits.stack_size % 4096 != 0 {
            xbail!(LucetError::UsageError(
                "Unsupported stack size: must be divisible by 4096"
            ));
        }
        if self.limits.globals_size % 4096 != 0 {
            xbail!(LucetError::UsageError(
                "Unsupported globals size: must be divisible by 4096"
            ));
        }
        let lucet_pool = unsafe { lucet_pool_create(self.entries_ as c_int, &self.limits) };
        if lucet_pool.is_null() {
            xbail!(LucetError::RuntimeError(
                "Unable to create a pool with the given parameters"
            ));
        }
        let pool = Pool { lucet_pool };
        Ok(pool)
    }
}
