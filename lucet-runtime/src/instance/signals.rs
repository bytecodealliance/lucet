//! A collection of wrappers that will be upstreamed to the `nix` crate eventually.

use bitflags::bitflags;

#[derive(Copy, Clone)]
#[allow(missing_debug_implementations)]
pub struct SigStack {
    stack: libc::stack_t,
}

impl SigStack {
    pub fn new(sp: *mut libc::c_void, flags: SigStackFlags, size: libc::size_t) -> SigStack {
        let mut stack = unsafe { std::mem::uninitialized::<libc::stack_t>() };
        stack.ss_sp = sp;
        stack.ss_flags = flags.bits();
        stack.ss_size = size;
        SigStack { stack }
    }

    pub fn flags(&self) -> SigStackFlags {
        SigStackFlags::from_bits_truncate(self.stack.ss_flags)
    }
}

impl AsRef<libc::stack_t> for SigStack {
    fn as_ref(&self) -> &libc::stack_t {
        &self.stack
    }
}

impl AsMut<libc::stack_t> for SigStack {
    fn as_mut(&mut self) -> &mut libc::stack_t {
        &mut self.stack
    }
}

bitflags! {
    pub struct SigStackFlags: libc::c_int {
        const SS_ONSTACK = libc::SS_ONSTACK;
        const SS_DISABLE = libc::SS_DISABLE;
    }
}

pub unsafe fn sigaltstack(ss: &SigStack) -> nix::Result<SigStack> {
    let mut oldstack = std::mem::uninitialized::<libc::stack_t>();

    let res = libc::sigaltstack(
        &ss.stack as *const libc::stack_t,
        &mut oldstack as *mut libc::stack_t,
    );

    nix::errno::Errno::result(res).map(|_| SigStack { stack: oldstack })
}
