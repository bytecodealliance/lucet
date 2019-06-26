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

pub fn dec_ptr(
    vmctx: &Vmctx,
    ptr: wasm32::uintptr_t,
    len: usize,
) -> Result<*const u8, host::__wasi_errno_t> {
    let heap = vmctx.heap();

    // check for overflow
    let checked_len = (ptr as usize)
        .checked_add(len)
        .ok_or(host::__WASI_EFAULT as host::__wasi_errno_t)?;
    if checked_len > heap.len() {
        bail_errno!(__WASI_EFAULT);
    }
    // translate the pointer
    Ok(unsafe { heap.as_ptr().offset(ptr as isize) })
}

pub fn dec_ptr_mut(
    vmctx: &Vmctx,
    ptr: wasm32::uintptr_t,
    len: usize,
) -> Result<*mut u8, host::__wasi_errno_t> {
    let mut heap = vmctx.heap_mut();

    // check for overflow
    let checked_len = (ptr as usize)
        .checked_add(len)
        .ok_or(host::__WASI_EFAULT as host::__wasi_errno_t)?;
    if checked_len > heap.len() {
        bail_errno!(__WASI_EFAULT);
    }
    // translate the pointer
    Ok(unsafe { heap.as_mut_ptr().offset(ptr as isize) })
}

pub fn dec_ptr_to<T>(
    vmctx: &Vmctx,
    ptr: wasm32::uintptr_t,
) -> Result<*const T, host::__wasi_errno_t> {
    // check that the ptr is aligned
    if ptr as usize % align_of::<T>() != 0 {
        bail_errno!(__WASI_EINVAL);
    }
    dec_ptr(vmctx, ptr, size_of::<T>()).map(|p| p as *const T)
}

pub fn dec_ptr_to_mut<T>(
    vmctx: &Vmctx,
    ptr: wasm32::uintptr_t,
) -> Result<*mut T, host::__wasi_errno_t> {
    // check that the ptr is aligned
    if ptr as usize % align_of::<T>() != 0 {
        bail_errno!(__WASI_EINVAL);
    }
    dec_ptr_mut(vmctx, ptr, size_of::<T>()).map(|p| p as *mut T)
}

pub fn dec_pointee<T>(vmctx: &Vmctx, ptr: wasm32::uintptr_t) -> Result<T, host::__wasi_errno_t> {
    dec_ptr_to::<T>(vmctx, ptr).map(|p| unsafe { p.read() })
}

pub fn enc_pointee<T>(
    vmctx: &Vmctx,
    ptr: wasm32::uintptr_t,
    t: T,
) -> Result<(), host::__wasi_errno_t> {
    dec_ptr_to_mut::<T>(vmctx, ptr).map(|p| unsafe { p.write(t) })
}

fn check_slice_of<T>(
    ptr: wasm32::uintptr_t,
    len: wasm32::size_t,
) -> Result<(usize, usize), host::__wasi_errno_t> {
    // check alignment, and that length doesn't overflow
    if ptr as usize % align_of::<T>() != 0 {
        bail_errno!(__WASI_EINVAL);
    }
    let len = dec_usize(len);
    let len_bytes = if let Some(len) = size_of::<T>().checked_mul(len) {
        len
    } else {
        bail_errno!(__WASI_EOVERFLOW);
    };
    Ok((len, len_bytes))
}

pub fn dec_slice_of<'vmctx, T>(
    vmctx: &'vmctx Vmctx,
    ptr: wasm32::uintptr_t,
    len: wasm32::size_t,
) -> Result<&'vmctx [T], host::__wasi_errno_t> {
    let (len, len_bytes) = check_slice_of::<T>(ptr, len)?;
    let ptr = dec_ptr(vmctx, ptr, len_bytes)? as *const T;
    Ok(unsafe { slice::from_raw_parts(ptr, len) })
}

pub fn dec_slice_of_mut<'vmctx, T>(
    vmctx: &'vmctx Vmctx,
    ptr: wasm32::uintptr_t,
    len: wasm32::size_t,
) -> Result<&'vmctx mut [T], host::__wasi_errno_t> {
    let (len, len_bytes) = check_slice_of::<T>(ptr, len)?;
    let ptr = dec_ptr_mut(vmctx, ptr, len_bytes)? as *mut T;
    Ok(unsafe { slice::from_raw_parts_mut(ptr, len) })
}

pub fn enc_slice_of<T>(
    vmctx: &Vmctx,
    slice: &[T],
    ptr: wasm32::uintptr_t,
) -> Result<(), host::__wasi_errno_t> {
    // check alignment
    if ptr as usize % align_of::<T>() != 0 {
        return Err(host::__WASI_EINVAL as host::__wasi_errno_t);
    }
    // check that length doesn't overflow
    let len_bytes = if let Some(len) = size_of::<T>().checked_mul(slice.len()) {
        len
    } else {
        return Err(host::__WASI_EOVERFLOW as host::__wasi_errno_t);
    };

    // get the pointer into guest memory, and copy the bytes
    let ptr = dec_ptr(vmctx, ptr, len_bytes)? as *mut libc::c_void;
    unsafe { std::ptr::copy_nonoverlapping(slice.as_ptr() as *const libc::c_void, ptr, len_bytes) };

    Ok(())
}

macro_rules! dec_enc_scalar {
    ( $ty:ident, $dec:ident, $dec_byref:ident, $enc:ident, $enc_byref:ident) => {
        pub fn $dec(x: wasm32::$ty) -> host::$ty {
            host::$ty::from_le(x)
        }

        pub fn $dec_byref(
            vmctx: &Vmctx,
            ptr: wasm32::uintptr_t,
        ) -> Result<host::$ty, host::__wasi_errno_t> {
            dec_pointee::<wasm32::$ty>(vmctx, ptr).map($dec)
        }

        pub fn $enc(x: host::$ty) -> wasm32::$ty {
            x.to_le()
        }

        pub fn $enc_byref(
            vmctx: &Vmctx,
            ptr: wasm32::uintptr_t,
            x: host::$ty,
        ) -> Result<(), host::__wasi_errno_t> {
            enc_pointee::<wasm32::$ty>(vmctx, ptr, $enc(x))
        }
    };
}

pub fn dec_ciovec(
    vmctx: &Vmctx,
    ciovec: &wasm32::__wasi_ciovec_t,
) -> Result<host::__wasi_ciovec_t, host::__wasi_errno_t> {
    let len = dec_usize(ciovec.buf_len);
    Ok(host::__wasi_ciovec_t {
        buf: dec_ptr(vmctx, ciovec.buf, len)? as *const host::void,
        buf_len: len,
    })
}

pub fn dec_ciovec_slice(
    vmctx: &Vmctx,
    ptr: wasm32::uintptr_t,
    len: wasm32::size_t,
) -> Result<Vec<host::__wasi_ciovec_t>, host::__wasi_errno_t> {
    let slice = dec_slice_of::<wasm32::__wasi_ciovec_t>(vmctx, ptr, len)?;
    slice.iter().map(|iov| dec_ciovec(vmctx, iov)).collect()
}

pub fn dec_iovec(
    vmctx: &Vmctx,
    iovec: &wasm32::__wasi_iovec_t,
) -> Result<host::__wasi_iovec_t, host::__wasi_errno_t> {
    let len = dec_usize(iovec.buf_len);
    Ok(host::__wasi_iovec_t {
        buf: dec_ptr(vmctx, iovec.buf, len)? as *mut host::void,
        buf_len: len,
    })
}

pub fn dec_iovec_slice(
    vmctx: &Vmctx,
    ptr: wasm32::uintptr_t,
    len: wasm32::size_t,
) -> Result<Vec<host::__wasi_iovec_t>, host::__wasi_errno_t> {
    let slice = dec_slice_of::<wasm32::__wasi_iovec_t>(vmctx, ptr, len)?;
    slice.iter().map(|iov| dec_iovec(vmctx, iov)).collect()
}

dec_enc_scalar!(
    __wasi_clockid_t,
    dec_clockid,
    dec_clockid_byref,
    enc_clockid,
    enc_clockid_byref
);
dec_enc_scalar!(
    __wasi_errno_t,
    dec_errno,
    dec_errno_byref,
    enc_errno,
    enc_errno_byref
);
dec_enc_scalar!(
    __wasi_exitcode_t,
    dec_exitcode,
    dec_exitcode_byref,
    enc_exitcode,
    enc_exitcode_byref
);
dec_enc_scalar!(__wasi_fd_t, dec_fd, dec_fd_byref, enc_fd, enc_fd_byref);
dec_enc_scalar!(
    __wasi_fdflags_t,
    dec_fdflags,
    dec_fdflags_byref,
    enc_fdflags,
    enc_fdflags_byref
);
dec_enc_scalar!(
    __wasi_device_t,
    dec_device,
    dev_device_byref,
    enc_device,
    enc_device_byref
);
dec_enc_scalar!(
    __wasi_inode_t,
    dec_inode,
    dev_inode_byref,
    enc_inode,
    enc_inode_byref
);
dec_enc_scalar!(
    __wasi_linkcount_t,
    dec_linkcount,
    dev_linkcount_byref,
    enc_linkcount,
    enc_linkcount_byref
);

pub fn dec_filestat(filestat: wasm32::__wasi_filestat_t) -> host::__wasi_filestat_t {
    host::__wasi_filestat_t {
        st_dev: dec_device(filestat.st_dev),
        st_ino: dec_inode(filestat.st_ino),
        st_filetype: dec_filetype(filestat.st_filetype),
        st_nlink: dec_linkcount(filestat.st_nlink),
        st_size: dec_filesize(filestat.st_size),
        st_atim: dec_timestamp(filestat.st_atim),
        st_mtim: dec_timestamp(filestat.st_mtim),
        st_ctim: dec_timestamp(filestat.st_ctim),
    }
}

pub fn dec_filestat_byref(
    vmctx: &Vmctx,
    filestat_ptr: wasm32::uintptr_t,
) -> Result<host::__wasi_filestat_t, host::__wasi_errno_t> {
    dec_pointee::<wasm32::__wasi_filestat_t>(vmctx, filestat_ptr).map(dec_filestat)
}

pub fn enc_filestat(filestat: host::__wasi_filestat_t) -> wasm32::__wasi_filestat_t {
    wasm32::__wasi_filestat_t {
        st_dev: enc_device(filestat.st_dev),
        st_ino: enc_inode(filestat.st_ino),
        st_filetype: enc_filetype(filestat.st_filetype),
        st_nlink: enc_linkcount(filestat.st_nlink),
        st_size: enc_filesize(filestat.st_size),
        st_atim: enc_timestamp(filestat.st_atim),
        st_mtim: enc_timestamp(filestat.st_mtim),
        st_ctim: enc_timestamp(filestat.st_ctim),
    }
}

pub fn enc_filestat_byref(
    vmctx: &Vmctx,
    filestat_ptr: wasm32::uintptr_t,
    host_filestat: host::__wasi_filestat_t,
) -> Result<(), host::__wasi_errno_t> {
    let filestat = enc_filestat(host_filestat);
    enc_pointee::<wasm32::__wasi_filestat_t>(vmctx, filestat_ptr, filestat)
}

pub fn dec_fdstat(fdstat: wasm32::__wasi_fdstat_t) -> host::__wasi_fdstat_t {
    host::__wasi_fdstat_t {
        fs_filetype: dec_filetype(fdstat.fs_filetype),
        fs_flags: dec_fdflags(fdstat.fs_flags),
        fs_rights_base: dec_rights(fdstat.fs_rights_base),
        fs_rights_inheriting: dec_rights(fdstat.fs_rights_inheriting),
    }
}

pub fn dec_fdstat_byref(
    vmctx: &Vmctx,
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

pub fn enc_fdstat_byref(
    vmctx: &Vmctx,
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
    __wasi_lookupflags_t,
    dec_lookupflags,
    dec_lookupflags_byref,
    enc_lookupflags,
    enc_lookupflags_byref
);

dec_enc_scalar!(
    __wasi_oflags_t,
    dec_oflags,
    dec_oflags_byref,
    enc_oflags,
    enc_oflags_byref
);

pub fn dec_prestat(
    prestat: wasm32::__wasi_prestat_t,
) -> Result<host::__wasi_prestat_t, host::__wasi_errno_t> {
    match prestat.pr_type {
        wasm32::__WASI_PREOPENTYPE_DIR => {
            let u = host::__wasi_prestat_t___wasi_prestat_u {
                dir: host::__wasi_prestat_t___wasi_prestat_u___wasi_prestat_u_dir_t {
                    pr_name_len: dec_usize(unsafe { prestat.u.dir.pr_name_len }),
                },
            };
            Ok(host::__wasi_prestat_t {
                pr_type: host::__WASI_PREOPENTYPE_DIR as host::__wasi_preopentype_t,
                u,
            })
        }
        _ => Err(host::__WASI_EINVAL as host::__wasi_errno_t),
    }
}

pub fn dec_prestat_byref(
    vmctx: &Vmctx,
    prestat_ptr: wasm32::uintptr_t,
) -> Result<host::__wasi_prestat_t, host::__wasi_errno_t> {
    dec_pointee::<wasm32::__wasi_prestat_t>(vmctx, prestat_ptr).and_then(dec_prestat)
}

pub fn enc_prestat(
    prestat: host::__wasi_prestat_t,
) -> Result<wasm32::__wasi_prestat_t, host::__wasi_errno_t> {
    match u32::from(prestat.pr_type) {
        host::__WASI_PREOPENTYPE_DIR => {
            let u = wasm32::__wasi_prestat_t___wasi_prestat_u {
                dir: wasm32::__wasi_prestat_t___wasi_prestat_u___wasi_prestat_u_dir_t {
                    pr_name_len: enc_usize(unsafe { prestat.u.dir.pr_name_len }),
                },
            };
            Ok(wasm32::__wasi_prestat_t {
                pr_type: wasm32::__WASI_PREOPENTYPE_DIR as wasm32::__wasi_preopentype_t,
                u,
            })
        }
        _ => Err(host::__WASI_EINVAL as host::__wasi_errno_t),
    }
}

pub fn enc_prestat_byref(
    vmctx: &Vmctx,
    prestat_ptr: wasm32::uintptr_t,
    host_prestat: host::__wasi_prestat_t,
) -> Result<(), host::__wasi_errno_t> {
    let prestat = enc_prestat(host_prestat)?;
    enc_pointee::<wasm32::__wasi_prestat_t>(vmctx, prestat_ptr, prestat)
}

dec_enc_scalar!(
    __wasi_rights_t,
    dec_rights,
    dec_rights_byref,
    enc_rights,
    enc_rights_byref
);

dec_enc_scalar!(
    __wasi_timestamp_t,
    dec_timestamp,
    dec_timestamp_byref,
    enc_timestamp,
    enc_timestamp_byref
);

pub fn dec_u32(x: u32) -> u32 {
    u32::from_le(x)
}

pub fn enc_u32(x: u32) -> u32 {
    x.to_le()
}

pub fn dec_usize(size: wasm32::size_t) -> usize {
    cast::usize(u32::from_le(size))
}

pub fn enc_usize(size: usize) -> wasm32::size_t {
    wasm32::size_t::cast(size).unwrap()
}

pub fn enc_usize_byref(
    vmctx: &Vmctx,
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

dec_enc_scalar!(
    __wasi_subclockflags_t,
    dec_subclockflags,
    dec_subclockflags_byref,
    enc_subclockflags,
    enc_subclockflags_byref
);

dec_enc_scalar!(
    __wasi_eventrwflags_t,
    dec_eventrwflags,
    dec_eventrwflags_byref,
    enc_eventrwflags,
    enc_eventrwflags_byref
);

dec_enc_scalar!(
    __wasi_eventtype_t,
    dec_eventtype,
    dec_eventtype_byref,
    enc_eventtype,
    enc_eventtype_byref
);

dec_enc_scalar!(
    __wasi_userdata_t,
    dec_userdata,
    dec_userdata_byref,
    enc_userdata,
    enc_userdata_byref
);

pub fn dec_subscription(
    subscription: &wasm32::__wasi_subscription_t,
) -> Result<host::__wasi_subscription_t, host::__wasi_errno_t> {
    let userdata = dec_userdata(subscription.userdata);
    let type_ = dec_eventtype(subscription.type_);
    let u_orig = subscription.__bindgen_anon_1;
    let u = match type_ {
        wasm32::__WASI_EVENTTYPE_CLOCK => host::__wasi_subscription_t___wasi_subscription_u {
            clock: unsafe {
                host::__wasi_subscription_t___wasi_subscription_u___wasi_subscription_u_clock_t {
                    identifier: dec_userdata(u_orig.clock.identifier),
                    clock_id: dec_clockid(u_orig.clock.clock_id),
                    timeout: dec_timestamp(u_orig.clock.timeout),
                    precision: dec_timestamp(u_orig.clock.precision),
                    flags: dec_subclockflags(u_orig.clock.flags),
                }
            },
        },
        wasm32::__WASI_EVENTTYPE_FD_READ | wasm32::__WASI_EVENTTYPE_FD_WRITE =>  host::__wasi_subscription_t___wasi_subscription_u {
            fd_readwrite:  host::__wasi_subscription_t___wasi_subscription_u___wasi_subscription_u_fd_readwrite_t {
                fd: dec_fd(unsafe{u_orig.fd_readwrite.fd})
            }
        },
        _  => return Err(wasm32::__WASI_EINVAL)
    };
    Ok(host::__wasi_subscription_t { userdata, type_, u })
}

pub fn enc_event(event: host::__wasi_event_t) -> wasm32::__wasi_event_t {
    let fd_readwrite = unsafe { event.u.fd_readwrite };
    wasm32::__wasi_event_t {
        userdata: enc_userdata(event.userdata),
        type_: enc_eventtype(event.type_),
        error: enc_errno(event.error),
        __bindgen_anon_1: wasm32::__wasi_event_t__bindgen_ty_1 {
            fd_readwrite: wasm32::__wasi_event_t__bindgen_ty_1__bindgen_ty_1 {
                nbytes: enc_filesize(fd_readwrite.nbytes),
                flags: enc_eventrwflags(fd_readwrite.flags),
                __bindgen_padding_0: [0; 3],
            },
        },
        __bindgen_padding_0: 0,
    }
}

dec_enc_scalar!(
    __wasi_advice_t,
    dec_advice,
    dec_advice_byref,
    enc_advice,
    enc_advice_byref
);

dec_enc_scalar!(
    __wasi_fstflags_t,
    dec_fstflags,
    dec_fstflags_byref,
    enc_fstflags,
    enc_fstflags_byref
);

dec_enc_scalar!(
    __wasi_dircookie_t,
    dec_dircookie,
    dec_dircookie_byref,
    enc_dircookie,
    enc_dircookie_byref
);

#[cfg(target_os = "linux")]
pub fn dirent_from_host(
    host_entry: &nix::libc::dirent,
) -> Result<wasm32::__wasi_dirent_t, host::__wasi_errno_t> {
    let mut entry = unsafe { std::mem::zeroed::<wasm32::__wasi_dirent_t>() };
    let d_namlen = unsafe { std::ffi::CStr::from_ptr(host_entry.d_name.as_ptr()) }
        .to_bytes()
        .len();
    if d_namlen > u32::max_value() as usize {
        return Err(host::__WASI_EIO as host::__wasi_errno_t);
    }
    entry.d_ino = enc_inode(host_entry.d_ino);
    entry.d_next = enc_dircookie(host_entry.d_off as u64);
    entry.d_namlen = enc_u32(d_namlen as u32);
    entry.d_type = enc_filetype(host_entry.d_type);
    Ok(entry)
}

#[cfg(not(target_os = "linux"))]
pub fn dirent_from_host(
    host_entry: &nix::libc::dirent,
) -> Result<wasm32::__wasi_dirent_t, host::__wasi_errno_t> {
    let mut entry = unsafe { std::mem::zeroed::<wasm32::__wasi_dirent_t>() };
    entry.d_ino = enc_inode(host_entry.d_ino);
    entry.d_next = enc_dircookie(host_entry.d_seekoff);
    entry.d_namlen = enc_u32(u32::from(host_entry.d_namlen));
    entry.d_type = enc_filetype(host_entry.d_type);
    Ok(entry)
}
