//! Functions to go back and forth between WASI types in host and wasm32 representations.
//!
//! This module is an adaptation of the `wasmtime-wasi` module
//! [`translate.rs`](https://github.com/CraneStation/wasmtime-wasi/blob/1a6ecf3a0378d71f3fc1ba25ce76a2b43e4166b8/lib/wasi/src/translate.rs);
//! its license file `LICENSE.wasmtime-wasi` is included in this project.
//!
//! Any of these functions that take a `Vmctx` argument are only meant to be called from within a
//! hostcall.
//!
//! This sort of manual encoding will hopefully be obsolete once the IDL is developed.

use crate::{host, wasm32};
use cast;
use cast::From as _0;
use lucet_runtime::vmctx::Vmctx;
use std::mem::{align_of, size_of};
use std::slice;

macro_rules! bail_errno {
    ( $errno:ident ) => {
        return Err(host::$errno as host::__wasi_errno_t);
    };
}

pub unsafe fn dec_ptr(
    vmctx: &mut Vmctx,
    ptr: wasm32::uintptr_t,
    len: usize,
) -> Result<*mut u8, host::__wasi_errno_t> {
    let heap = vmctx.heap_mut();

    // check that `len` fits in the wasm32 address space
    if len > wasm32::UINTPTR_MAX as usize {
        bail_errno!(__WASI_EINVAL);
    }

    // check that `ptr` and `ptr + len` are both within the guest heap
    if ptr as usize > heap.len() || ptr as usize + len > heap.len() {
        bail_errno!(__WASI_EFAULT);
    }

    // translate the pointer
    Ok(heap.as_mut_ptr().offset(ptr as isize))
}

pub unsafe fn dec_ptr_to<T>(
    vmctx: &mut Vmctx,
    ptr: wasm32::uintptr_t,
) -> Result<*mut T, host::__wasi_errno_t> {
    // check that the ptr is aligned
    if ptr as usize % align_of::<T>() != 0 {
        bail_errno!(__WASI_EINVAL);
    }
    dec_ptr(vmctx, ptr, size_of::<T>()).map(|p| p as *mut T)
}

pub unsafe fn dec_pointee<T>(
    vmctx: &mut Vmctx,
    ptr: wasm32::uintptr_t,
) -> Result<T, host::__wasi_errno_t> {
    dec_ptr_to::<T>(vmctx, ptr).map(|p| p.read())
}

pub unsafe fn enc_pointee<T>(
    vmctx: &mut Vmctx,
    ptr: wasm32::uintptr_t,
    t: T,
) -> Result<(), host::__wasi_errno_t> {
    dec_ptr_to::<T>(vmctx, ptr).map(|p| p.write(t))
}

pub unsafe fn dec_slice_of<T>(
    vmctx: &mut Vmctx,
    ptr: wasm32::uintptr_t,
    len: wasm32::size_t,
) -> Result<(*mut T, usize), host::__wasi_errno_t> {
    // check alignment, and that length doesn't overflow
    if ptr as usize % align_of::<T>() != 0 {
        return Err(host::__WASI_EINVAL as host::__wasi_errno_t);
    }
    let len = dec_usize(len);
    let len_bytes = if let Some(len) = size_of::<T>().checked_mul(len) {
        len
    } else {
        return Err(host::__WASI_EINVAL as host::__wasi_errno_t);
    };

    let ptr = dec_ptr(vmctx, ptr, len_bytes)? as *mut T;

    Ok((ptr, len))
}

macro_rules! dec_enc_scalar {
    ( $ty:ident, $dec:ident, $dec_byref:ident, $enc:ident, $enc_byref:ident) => {
        pub fn $dec(x: wasm32::$ty) -> host::$ty {
            host::$ty::from_le(x)
        }

        pub unsafe fn $dec_byref(
            vmctx: &mut Vmctx,
            ptr: wasm32::uintptr_t,
        ) -> Result<host::$ty, host::__wasi_errno_t> {
            dec_pointee::<wasm32::$ty>(vmctx, ptr).map($dec)
        }

        pub fn $enc(x: host::$ty) -> wasm32::$ty {
            x.to_le()
        }

        pub unsafe fn $enc_byref(
            vmctx: &mut Vmctx,
            ptr: wasm32::uintptr_t,
            x: host::$ty,
        ) -> Result<(), host::__wasi_errno_t> {
            enc_pointee::<wasm32::$ty>(vmctx, ptr, $enc(x))
        }
    };
}

pub unsafe fn dec_ciovec(
    vmctx: &mut Vmctx,
    ciovec: &wasm32::__wasi_ciovec_t,
) -> Result<host::__wasi_ciovec_t, host::__wasi_errno_t> {
    let len = dec_usize(ciovec.buf_len);
    Ok(host::__wasi_ciovec_t {
        buf: dec_ptr(vmctx, ciovec.buf, len)? as *const host::void,
        buf_len: len,
    })
}

pub unsafe fn dec_ciovec_slice(
    vmctx: &mut Vmctx,
    ptr: wasm32::uintptr_t,
    len: wasm32::size_t,
) -> Result<Vec<host::__wasi_ciovec_t>, host::__wasi_errno_t> {
    let slice = dec_slice_of::<wasm32::__wasi_ciovec_t>(vmctx, ptr, len)?;
    let slice = slice::from_raw_parts(slice.0, slice.1);
    slice.iter().map(|iov| dec_ciovec(vmctx, iov)).collect()
}

dec_enc_scalar!(
    __wasi_errno_t,
    dec_errno,
    dec_errno_byref,
    enc_errno,
    enc_errno_byref
);
dec_enc_scalar!(__wasi_fd_t, dec_fd, dec_fd_byref, enc_fd, enc_fd_byref);
dec_enc_scalar!(
    __wasi_fdflags_t,
    dec_fdflags,
    dec_fdflags_byref,
    enc_fdflags,
    enc_fdflags_byref
);

pub fn dec_fdstat(fdstat: wasm32::__wasi_fdstat_t) -> host::__wasi_fdstat_t {
    host::__wasi_fdstat_t {
        fs_filetype: dec_filetype(fdstat.fs_filetype),
        fs_flags: dec_fdflags(fdstat.fs_flags),
        fs_rights_base: dec_rights(fdstat.fs_rights_base),
        fs_rights_inheriting: dec_rights(fdstat.fs_rights_inheriting),
    }
}

pub unsafe fn dec_fdstat_byref(
    vmctx: &mut Vmctx,
    fdstat_ptr: wasm32::uintptr_t,
) -> Result<host::__wasi_fdstat_t, host::__wasi_errno_t> {
    dec_pointee::<wasm32::__wasi_fdstat_t>(vmctx, fdstat_ptr).map(dec_fdstat)
}

pub fn enc_fdstat(fdstat: host::__wasi_fdstat_t) -> wasm32::__wasi_fdstat_t {
    wasm32::__wasi_fdstat_t {
        fs_filetype: enc_filetype(fdstat.fs_filetype),
        fs_flags: enc_fdflags(fdstat.fs_flags),
        __bindgen_padding_0: 0,
        fs_rights_base: enc_rights(fdstat.fs_rights_base),
        fs_rights_inheriting: enc_rights(fdstat.fs_rights_inheriting),
    }
}

pub unsafe fn enc_fdstat_byref(
    vmctx: &mut Vmctx,
    fdstat_ptr: wasm32::uintptr_t,
    host_fdstat: host::__wasi_fdstat_t,
) -> Result<(), host::__wasi_errno_t> {
    let fdstat = enc_fdstat(host_fdstat);
    enc_pointee::<wasm32::__wasi_fdstat_t>(vmctx, fdstat_ptr, fdstat)
}

dec_enc_scalar!(
    __wasi_filedelta_t,
    dec_filedelta,
    dec_filedelta_byref,
    enc_filedelta,
    enc_filedelta_byref
);
dec_enc_scalar!(
    __wasi_filesize_t,
    dec_filesize,
    dec_filesize_byref,
    enc_filesize,
    enc_filesize_byref
);

dec_enc_scalar!(
    __wasi_filetype_t,
    dec_filetype,
    dec_filetype_byref,
    enc_filetype,
    enc_filetype_byref
);
dec_enc_scalar!(
    __wasi_rights_t,
    dec_rights,
    dec_rights_byref,
    enc_rights,
    enc_rights_byref
);

pub fn dec_usize(size: wasm32::size_t) -> usize {
    cast::usize(u32::from_le(size))
}

pub fn enc_usize(size: usize) -> wasm32::size_t {
    wasm32::size_t::cast(size).unwrap()
}

pub unsafe fn enc_usize_byref(
    vmctx: &mut Vmctx,
    usize_ptr: wasm32::uintptr_t,
    host_usize: usize,
) -> Result<(), host::__wasi_errno_t> {
    enc_pointee::<wasm32::size_t>(vmctx, usize_ptr, enc_usize(host_usize))
}

dec_enc_scalar!(
    __wasi_whence_t,
    dec_whence,
    dec_whence_byref,
    enc_whence,
    enc_whence_byref
);
