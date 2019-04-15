use libc::{c_void, ucontext_t, REG_RIP};

#[derive(Clone, Copy, Debug)]
pub struct UContext {
    inner_ptr: *const ucontext_t,
}

impl UContext {
    #[inline]
    pub fn new(ptr: *const c_void) -> UContext {
        UContext {
            inner_ptr: ptr as *const ucontext_t,
        }
    }

    #[inline]
    pub fn get_ip(&self) -> *const c_void {
        let mcontext = unsafe { *self.inner_ptr }.uc_mcontext;
        mcontext.gregs[REG_RIP as usize] as *const _
    }
}
