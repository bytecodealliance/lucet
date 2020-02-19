use libc::{c_void, ucontext_t, REG_RDI, REG_RIP};

#[derive(Clone, Copy, Debug)]
pub struct UContextPtr(*mut ucontext_t);

impl UContextPtr {
    #[inline]
    pub fn new(ptr: *mut c_void) -> Self {
        assert!(!ptr.is_null(), "non-null context");
        UContextPtr(ptr as *mut ucontext_t)
    }

    #[inline]
    pub fn get_ip(self) -> *const c_void {
        let mcontext = &unsafe { self.0.as_ref().unwrap() }.uc_mcontext;
        mcontext.gregs[REG_RIP as usize] as *const _
    }

    #[inline]
    pub fn set_ip(self, new_ip: *const c_void) {
        let mut mcontext = &mut unsafe { self.0.as_mut().unwrap() }.uc_mcontext;
        mcontext.gregs[REG_RIP as usize] = new_ip as i64;
    }

    #[inline]
    pub fn set_rdi(self, new_rdi: u64) {
        let mut mcontext = &mut unsafe { self.0.as_mut().unwrap() }.uc_mcontext;
        mcontext.gregs[REG_RDI as usize] = new_rdi as i64;
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct UContext {
    context: *mut ucontext_t,
}

impl UContext {
    #[inline]
    pub fn new(ptr: *mut c_void) -> Self {
        UContext {
            context: unsafe { (ptr as *mut ucontext_t).as_mut().expect("non-null context") },
        }
    }

    pub fn as_ptr(&mut self) -> UContextPtr {
        UContextPtr::new(self.context as *mut _ as *mut _)
    }
}

impl Into<UContext> for UContextPtr {
    #[inline]
    fn into(self) -> UContext {
        UContext { context: self.0 }
    }
}
