//! Hostcalls that implement
//! [WASI](https://github.com/CraneStation/wasmtime-wasi/blob/wasi/docs/WASI-overview.md).
//!
//! This code borrows heavily from [wasmtime-wasi](https://github.com/CraneStation/wasmtime-wasi),
//! which in turn borrows from cloudabi-utils. See `LICENSE.wasmtime-wasi` for license information.
//!
//! This is currently a very incomplete prototype, only supporting the hostcalls required to run
//! `/examples/hello.c`, and using a bare-bones translation of the capabilities system rather than
//! something nice.

#![allow(non_camel_case_types)]
use crate::ctx::WasiCtx;
use crate::memory::*;
use crate::{host, wasm32};
use cast::From as _0;
use lucet_runtime::vmctx::{lucet_vmctx, Vmctx};
use std::os::unix::prelude::OsStrExt;

#[no_mangle]
pub extern "C" fn __wasi_proc_exit(vmctx: *mut lucet_vmctx, rval: wasm32::__wasi_exitcode_t) -> ! {
    let mut vmctx = unsafe { Vmctx::from_raw(vmctx) };
    vmctx.terminate(dec_exitcode(rval))
}

#[no_mangle]
pub extern "C" fn __wasi_args_get(
    vmctx_raw: *mut lucet_vmctx,
    argv_ptr: wasm32::uintptr_t,
    argv_buf: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let mut vmctx = unsafe { Vmctx::from_raw(vmctx_raw) };
    let ctx: &WasiCtx = vmctx.get_embed_ctx();

    let mut argv_buf_offset = 0;
    let mut argv = vec![];

    for arg in ctx.args.iter() {
        let arg_bytes = arg.as_bytes_with_nul();
        let arg_ptr = argv_buf + argv_buf_offset;

        // nasty aliasing here, but we aren't interfering with the borrow for `ctx`
        // TODO: rework vmctx interface to avoid this
        let mut vmctx = unsafe { Vmctx::from_raw(vmctx_raw) };
        if let Err(e) = unsafe { enc_slice_of(&mut vmctx, arg_bytes, arg_ptr) } {
            return enc_errno(e);
        }

        argv.push(arg_ptr);

        argv_buf_offset = if let Some(new_offset) = argv_buf_offset.checked_add(
            wasm32::uintptr_t::cast(arg_bytes.len())
                .expect("cast overflow would have been caught by `enc_slice_of` above"),
        ) {
            new_offset
        } else {
            return wasm32::__WASI_EOVERFLOW;
        }
    }

    unsafe {
        enc_slice_of(&mut vmctx, argv.as_slice(), argv_ptr)
            .map(|_| wasm32::__WASI_ESUCCESS)
            .unwrap_or_else(|e| e)
    }
}

#[no_mangle]
pub extern "C" fn __wasi_args_sizes_get(
    vmctx: *mut lucet_vmctx,
    argc_ptr: wasm32::uintptr_t,
    argv_buf_size_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let mut vmctx = unsafe { Vmctx::from_raw(vmctx) };

    let ctx: &WasiCtx = vmctx.get_embed_ctx();

    let argc = ctx.args.len();
    let argv_size = ctx
        .args
        .iter()
        .map(|arg| arg.as_bytes_with_nul().len())
        .sum();

    unsafe {
        if let Err(e) = enc_usize_byref(&mut vmctx, argc_ptr, argc) {
            return enc_errno(e);
        }
        if let Err(e) = enc_usize_byref(&mut vmctx, argv_buf_size_ptr, argv_size) {
            return enc_errno(e);
        }
    }
    wasm32::__WASI_ESUCCESS
}

#[no_mangle]
pub extern "C" fn __wasi_clock_res_get(
    vmctx: *mut lucet_vmctx,
    clock_id: wasm32::__wasi_clockid_t,
    resolution_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let mut vmctx = unsafe { Vmctx::from_raw(vmctx) };

    // convert the supported clocks to the libc types, or return EINVAL
    let clock_id = match dec_clockid(clock_id) {
        host::__WASI_CLOCK_REALTIME => libc::CLOCK_REALTIME,
        host::__WASI_CLOCK_MONOTONIC => libc::CLOCK_MONOTONIC,
        host::__WASI_CLOCK_PROCESS_CPUTIME_ID => libc::CLOCK_PROCESS_CPUTIME_ID,
        host::__WASI_CLOCK_THREAD_CPUTIME_ID => libc::CLOCK_THREAD_CPUTIME_ID,
        _ => return wasm32::__WASI_EINVAL,
    };

    // no `nix` wrapper for clock_getres, so we do it ourselves
    let mut timespec = unsafe { std::mem::uninitialized::<libc::timespec>() };
    let res = unsafe { libc::clock_getres(clock_id, &mut timespec as *mut libc::timespec) };
    if res != 0 {
        return wasm32::errno_from_nix(nix::errno::Errno::last());
    }

    // convert to nanoseconds, returning EOVERFLOW in case of overflow; this is freelancing a bit
    // from the spec but seems like it'll be an unusual situation to hit
    (timespec.tv_sec as host::__wasi_timestamp_t)
        .checked_mul(1_000_000_000)
        .and_then(|sec_ns| sec_ns.checked_add(timespec.tv_nsec as host::__wasi_timestamp_t))
        .map(|resolution| {
            // a supported clock can never return zero; this case will probably never get hit, but
            // make sure we follow the spec
            if resolution == 0 {
                wasm32::__WASI_EINVAL
            } else {
                unsafe {
                    enc_timestamp_byref(&mut vmctx, resolution_ptr, resolution)
                        .map(|_| wasm32::__WASI_ESUCCESS)
                        .unwrap_or_else(|e| e)
                }
            }
        })
        .unwrap_or(wasm32::__WASI_EOVERFLOW)
}

#[no_mangle]
pub extern "C" fn __wasi_clock_time_get(
    vmctx: *mut lucet_vmctx,
    clock_id: wasm32::__wasi_clockid_t,
    // ignored for now, but will be useful once we put optional limits on precision to reduce side
    // channels
    _precision: wasm32::__wasi_timestamp_t,
    time_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let mut vmctx = unsafe { Vmctx::from_raw(vmctx) };

    // convert the supported clocks to the libc types, or return EINVAL
    let clock_id = match dec_clockid(clock_id) {
        host::__WASI_CLOCK_REALTIME => libc::CLOCK_REALTIME,
        host::__WASI_CLOCK_MONOTONIC => libc::CLOCK_MONOTONIC,
        host::__WASI_CLOCK_PROCESS_CPUTIME_ID => libc::CLOCK_PROCESS_CPUTIME_ID,
        host::__WASI_CLOCK_THREAD_CPUTIME_ID => libc::CLOCK_THREAD_CPUTIME_ID,
        _ => return wasm32::__WASI_EINVAL,
    };

    // no `nix` wrapper for clock_getres, so we do it ourselves
    let mut timespec = unsafe { std::mem::uninitialized::<libc::timespec>() };
    let res = unsafe { libc::clock_gettime(clock_id, &mut timespec as *mut libc::timespec) };
    if res != 0 {
        return wasm32::errno_from_nix(nix::errno::Errno::last());
    }

    // convert to nanoseconds, returning EOVERFLOW in case of overflow; this is freelancing a bit
    // from the spec but seems like it'll be an unusual situation to hit
    (timespec.tv_sec as host::__wasi_timestamp_t)
        .checked_mul(1_000_000_000)
        .and_then(|sec_ns| sec_ns.checked_add(timespec.tv_nsec as host::__wasi_timestamp_t))
        .map(|time| unsafe {
            enc_timestamp_byref(&mut vmctx, time_ptr, time)
                .map(|_| wasm32::__WASI_ESUCCESS)
                .unwrap_or_else(|e| e)
        })
        .unwrap_or(wasm32::__WASI_EOVERFLOW)
}

#[no_mangle]
pub extern "C" fn __wasi_environ_get(
    vmctx_raw: *mut lucet_vmctx,
    environ_ptr: wasm32::uintptr_t,
    environ_buf: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let mut vmctx = unsafe { Vmctx::from_raw(vmctx_raw) };
    let ctx: &WasiCtx = vmctx.get_embed_ctx();

    let mut environ_buf_offset = 0;
    let mut environ = vec![];

    for pair in ctx.env.iter() {
        let env_bytes = pair.as_bytes_with_nul();
        let env_ptr = environ_buf + environ_buf_offset;

        // nasty aliasing here, but we aren't interfering with the borrow for `ctx`
        // TODO: rework vmctx interface to avoid this
        let mut vmctx = unsafe { Vmctx::from_raw(vmctx_raw) };
        if let Err(e) = unsafe { enc_slice_of(&mut vmctx, env_bytes, env_ptr) } {
            return enc_errno(e);
        }

        environ.push(env_ptr);

        environ_buf_offset = if let Some(new_offset) = environ_buf_offset.checked_add(
            wasm32::uintptr_t::cast(env_bytes.len())
                .expect("cast overflow would have been caught by `enc_slice_of` above"),
        ) {
            new_offset
        } else {
            return wasm32::__WASI_EOVERFLOW;
        }
    }

    unsafe {
        enc_slice_of(&mut vmctx, environ.as_slice(), environ_ptr)
            .map(|_| wasm32::__WASI_ESUCCESS)
            .unwrap_or_else(|e| e)
    }
}

#[no_mangle]
pub extern "C" fn __wasi_environ_sizes_get(
    vmctx: *mut lucet_vmctx,
    environ_count_ptr: wasm32::uintptr_t,
    environ_size_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let mut vmctx = unsafe { Vmctx::from_raw(vmctx) };

    let ctx: &WasiCtx = vmctx.get_embed_ctx();

    let environ_count = ctx.env.len();
    let environ_size = ctx
        .env
        .iter()
        .map(|pair| pair.as_bytes_with_nul().len())
        .sum();

    unsafe {
        if let Err(e) = enc_usize_byref(&mut vmctx, environ_count_ptr, environ_count) {
            return enc_errno(e);
        }
        if let Err(e) = enc_usize_byref(&mut vmctx, environ_size_ptr, environ_size) {
            return enc_errno(e);
        }
    }
    wasm32::__WASI_ESUCCESS
}

#[no_mangle]
pub extern "C" fn __wasi_fd_close(
    vmctx: *mut lucet_vmctx,
    fd: wasm32::__wasi_fd_t,
) -> wasm32::__wasi_errno_t {
    let mut vmctx = unsafe { Vmctx::from_raw(vmctx) };
    let ctx: &mut WasiCtx = vmctx.get_embed_ctx_mut();
    let fd = dec_fd(fd);
    if let Some(fdent) = ctx.fds.remove(&fd) {
        match nix::unistd::close(fdent.fd_object.rawfd) {
            Ok(_) => wasm32::__WASI_ESUCCESS,
            Err(e) => wasm32::errno_from_nix(e.as_errno().unwrap()),
        }
    } else {
        wasm32::__WASI_EBADF
    }
}

#[no_mangle]
pub extern "C" fn __wasi_fd_fdstat_get(
    vmctx: *mut lucet_vmctx,
    fd: wasm32::__wasi_fd_t,
    fdstat_ptr: wasm32::uintptr_t, // *mut wasm32::__wasi_fdstat_t
) -> wasm32::__wasi_errno_t {
    let mut vmctx = unsafe { Vmctx::from_raw(vmctx) };

    let host_fd = dec_fd(fd);
    let mut host_fdstat = match unsafe { dec_fdstat_byref(&mut vmctx, fdstat_ptr) } {
        Ok(host_fdstat) => host_fdstat,
        Err(e) => return enc_errno(e),
    };

    let ctx: &mut WasiCtx = vmctx.get_embed_ctx_mut();
    let errno = if let Some(fe) = ctx.fds.get(&host_fd) {
        host_fdstat.fs_filetype = fe.fd_object.ty;
        host_fdstat.fs_rights_base = fe.rights_base;
        host_fdstat.fs_rights_inheriting = fe.rights_inheriting;
        use nix::fcntl::{fcntl, OFlag, F_GETFL};
        match fcntl(fe.fd_object.rawfd, F_GETFL).map(OFlag::from_bits_truncate) {
            Ok(flags) => {
                if flags.contains(OFlag::O_APPEND) {
                    host_fdstat.fs_flags |= wasm32::__WASI_FDFLAG_APPEND;
                }
                if flags.contains(OFlag::O_DSYNC) {
                    host_fdstat.fs_flags |= wasm32::__WASI_FDFLAG_DSYNC;
                }
                if flags.contains(OFlag::O_NONBLOCK) {
                    host_fdstat.fs_flags |= wasm32::__WASI_FDFLAG_NONBLOCK;
                }
                if flags.contains(OFlag::O_RSYNC) {
                    host_fdstat.fs_flags |= wasm32::__WASI_FDFLAG_RSYNC;
                }
                if flags.contains(OFlag::O_SYNC) {
                    host_fdstat.fs_flags |= wasm32::__WASI_FDFLAG_SYNC;
                }
                wasm32::__WASI_ESUCCESS
            }
            Err(e) => wasm32::errno_from_nix(e.as_errno().unwrap()),
        }
    } else {
        wasm32::__WASI_EBADF
    };

    unsafe {
        enc_fdstat_byref(&mut vmctx, fdstat_ptr, host_fdstat)
            .expect("can write back into the pointer we read from");
    }

    errno
}

#[no_mangle]
pub extern "C" fn __wasi_fd_seek(
    vmctx: *mut lucet_vmctx,
    fd: wasm32::__wasi_fd_t,
    offset: wasm32::__wasi_filedelta_t,
    whence: wasm32::__wasi_whence_t,
    newoffset: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let mut vmctx = unsafe { Vmctx::from_raw(vmctx) };
    let ctx: &mut WasiCtx = vmctx.get_embed_ctx_mut();
    let fd = dec_fd(fd);
    let offset = dec_filedelta(offset);
    let whence = dec_whence(whence);

    let host_newoffset = {
        use nix::unistd::{lseek, Whence};
        let nwhence = match whence as u32 {
            host::__WASI_WHENCE_CUR => Whence::SeekCur,
            host::__WASI_WHENCE_END => Whence::SeekEnd,
            host::__WASI_WHENCE_SET => Whence::SeekSet,
            _ => return wasm32::__WASI_EINVAL,
        };

        let rights = if offset == 0 && whence as u32 == host::__WASI_WHENCE_CUR {
            host::__WASI_RIGHT_FD_TELL
        } else {
            host::__WASI_RIGHT_FD_SEEK | host::__WASI_RIGHT_FD_TELL
        };
        match ctx.get_fd_entry(fd, rights.into(), 0) {
            Ok(fe) => match lseek(fe.fd_object.rawfd, offset, nwhence) {
                Ok(newoffset) => newoffset,
                Err(e) => return wasm32::errno_from_nix(e.as_errno().unwrap()),
            },
            Err(e) => return enc_errno(e),
        }
    };

    unsafe {
        enc_filesize_byref(&mut vmctx, newoffset, host_newoffset as u64)
            .map(|_| wasm32::__WASI_ESUCCESS)
            .unwrap_or_else(|e| e)
    }
}

#[no_mangle]
pub extern "C" fn __wasi_fd_prestat_get(
    vmctx_raw: *mut lucet_vmctx,
    fd: wasm32::__wasi_fd_t,
    prestat_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let vmctx = unsafe { Vmctx::from_raw(vmctx_raw) };
    let ctx: &WasiCtx = vmctx.get_embed_ctx();
    let fd = dec_fd(fd);
    // TODO: is this the correct right for this?
    match ctx.get_fd_entry(fd, host::__WASI_RIGHT_PATH_OPEN.into(), 0) {
        Ok(fe) => {
            if let Some(po_path) = &fe.preopen_path {
                if fe.fd_object.ty != host::__WASI_FILETYPE_DIRECTORY as host::__wasi_filetype_t {
                    return wasm32::__WASI_ENOTDIR;
                }
                // nasty aliasing here, but we aren't interfering with the borrow for `ctx`
                // TODO: rework vmctx interface to avoid this
                unsafe {
                    enc_prestat_byref(
                        &mut Vmctx::from_raw(vmctx_raw),
                        prestat_ptr,
                        host::__wasi_prestat_t {
                            pr_type: host::__WASI_PREOPENTYPE_DIR as host::__wasi_preopentype_t,
                            u: host::__wasi_prestat_t___wasi_prestat_u {
                                dir:
                                    host::__wasi_prestat_t___wasi_prestat_u___wasi_prestat_u_dir_t {
                                        pr_name_len: po_path.as_os_str().as_bytes().len(),
                                    },
                            },
                        },
                    )
                    .map(|_| wasm32::__WASI_ESUCCESS)
                    .unwrap_or_else(|e| e)
                }
            } else {
                wasm32::__WASI_ENOTSUP
            }
        }
        Err(e) => enc_errno(e),
    }
}

#[no_mangle]
pub extern "C" fn __wasi_fd_prestat_dir_name(
    vmctx_raw: *mut lucet_vmctx,
    fd: wasm32::__wasi_fd_t,
    path_ptr: wasm32::uintptr_t,
    path_len: wasm32::size_t,
) -> wasm32::__wasi_errno_t {
    let vmctx = unsafe { Vmctx::from_raw(vmctx_raw) };
    let ctx: &WasiCtx = vmctx.get_embed_ctx();
    let fd = dec_fd(fd);
    match ctx.get_fd_entry(fd, host::__WASI_RIGHT_PATH_OPEN.into(), 0) {
        Ok(fe) => {
            if let Some(po_path) = &fe.preopen_path {
                if fe.fd_object.ty != host::__WASI_FILETYPE_DIRECTORY as host::__wasi_filetype_t {
                    return wasm32::__WASI_ENOTDIR;
                }
                let path_bytes = po_path.as_os_str().as_bytes();
                if path_bytes.len() > dec_usize(path_len) {
                    return wasm32::__WASI_ENAMETOOLONG;
                }
                // nasty aliasing here, but we aren't interfering with the borrow for `ctx`
                // TODO: rework vmctx interface to avoid this
                unsafe {
                    enc_slice_of(&mut Vmctx::from_raw(vmctx_raw), path_bytes, path_ptr)
                        .map(|_| wasm32::__WASI_ESUCCESS)
                        .unwrap_or_else(|e| e)
                }
            } else {
                wasm32::__WASI_ENOTSUP
            }
        }
        Err(e) => enc_errno(e),
    }
}

#[no_mangle]
pub extern "C" fn __wasi_fd_read(
    vmctx: *mut lucet_vmctx,
    fd: wasm32::__wasi_fd_t,
    iovs_ptr: wasm32::uintptr_t,
    iovs_len: wasm32::size_t,
    nread: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    use nix::sys::uio::{readv, IoVec};

    let mut vmctx = unsafe { Vmctx::from_raw(vmctx) };
    let fd = dec_fd(fd);
    let mut iovs = match unsafe { dec_ciovec_slice(&mut vmctx, iovs_ptr, iovs_len) } {
        Ok(iovs) => iovs,
        Err(e) => return enc_errno(e),
    };

    let ctx: &WasiCtx = vmctx.get_embed_ctx();
    let fe = match ctx.get_fd_entry(fd, host::__WASI_RIGHT_FD_READ.into(), 0) {
        Ok(fe) => fe,
        Err(e) => return enc_errno(e),
    };

    let mut iovs: Vec<IoVec<&mut [u8]>> = iovs
        .iter_mut()
        .map(|iov| unsafe { host::ciovec_to_nix_mut(iov) })
        .collect();

    let host_nread = match readv(fe.fd_object.rawfd, &mut iovs) {
        Ok(len) => len,
        Err(e) => return wasm32::errno_from_nix(e.as_errno().unwrap()),
    };

    unsafe {
        enc_usize_byref(&mut vmctx, nread, host_nread)
            .map(|_| wasm32::__WASI_ESUCCESS)
            .unwrap_or_else(|e| e)
    }
}

#[no_mangle]
pub extern "C" fn __wasi_fd_write(
    vmctx: *mut lucet_vmctx,
    fd: wasm32::__wasi_fd_t,
    iovs_ptr: wasm32::uintptr_t,
    iovs_len: wasm32::size_t,
    nwritten: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    use nix::sys::uio::{writev, IoVec};

    let mut vmctx = unsafe { Vmctx::from_raw(vmctx) };
    let fd = dec_fd(fd);
    let iovs = match unsafe { dec_ciovec_slice(&mut vmctx, iovs_ptr, iovs_len) } {
        Ok(iovs) => iovs,
        Err(e) => return enc_errno(e),
    };

    let ctx: &mut WasiCtx = vmctx.get_embed_ctx_mut();
    let fe = match ctx.get_fd_entry(fd, host::__WASI_RIGHT_FD_WRITE.into(), 0) {
        Ok(fe) => fe,
        Err(e) => return enc_errno(e),
    };

    let iovs: Vec<IoVec<&[u8]>> = iovs
        .iter()
        .map(|iov| unsafe { host::ciovec_to_nix(iov) })
        .collect();

    let host_nwritten = match writev(fe.fd_object.rawfd, &iovs) {
        Ok(len) => len,
        Err(e) => return wasm32::errno_from_nix(e.as_errno().unwrap()),
    };

    unsafe {
        enc_usize_byref(&mut vmctx, nwritten, host_nwritten)
            .map(|_| wasm32::__WASI_ESUCCESS)
            .unwrap_or_else(|e| e)
    }
}

#[no_mangle]
pub extern "C" fn __wasi_random_get(
    vmctx: *mut lucet_vmctx,
    buf_ptr: wasm32::uintptr_t,
    buf_len: wasm32::size_t,
) -> wasm32::__wasi_errno_t {
    use rand::{thread_rng, RngCore};

    let mut vmctx = unsafe { Vmctx::from_raw(vmctx) };

    let buf_len = dec_usize(buf_len);
    let buf_ptr = match unsafe { dec_ptr(&mut vmctx, buf_ptr, buf_len) } {
        Ok(ptr) => ptr,
        Err(e) => return enc_errno(e),
    };

    let buf = unsafe { std::slice::from_raw_parts_mut(buf_ptr, buf_len) };

    thread_rng().fill_bytes(buf);

    return wasm32::__WASI_ESUCCESS;
}

#[doc(hidden)]
pub fn ensure_linked() {
    unsafe {
        std::ptr::read_volatile(__wasi_proc_exit as *const extern "C" fn());
    }
}
