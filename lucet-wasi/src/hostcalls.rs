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
use crate::memory::*;
use crate::{host, wasm32};
use cast::From as _0;
use lucet_runtime::vmctx::{lucet_vmctx, Vmctx};
use std::collections::HashMap;
use std::ffi::CString;
use std::fs::File;
use std::io::{stderr, stdin, stdout};
use std::os::unix::prelude::{AsRawFd, FileTypeExt, FromRawFd, RawFd};

#[no_mangle]
pub extern "C" fn __wasi_proc_exit(_vmctx: *mut lucet_vmctx, rval: wasm32::__wasi_exitcode_t) -> ! {
    std::process::exit(rval as i32)
}

pub struct WasiCtx {
    fds: HashMap<host::__wasi_fd_t, FdEntry>,
    args: Vec<CString>,
    env: Vec<CString>,
}

impl WasiCtx {
    pub fn new(module_path: &str, args: &[&str]) -> WasiCtx {
        WasiCtx::new_with_env(module_path, args, std::env::vars())
    }

    pub fn new_with_env<E, S>(module_path: &str, args: &[&str], env: E) -> WasiCtx
    where
        E: IntoIterator<Item = (S, S)>,
        S: AsRef<str>,
    {
        use nix::unistd::dup;

        let args = std::iter::once(&module_path)
            .chain(args.into_iter())
            .map(|arg| CString::new(*arg).expect("argument can be converted to a CString"))
            .collect();

        let env = env
            .into_iter()
            .map(|(k, v)| {
                CString::new(format!("{}={}", k.as_ref(), v.as_ref()))
                    .expect("environment pair can be converted to a CString")
            })
            .collect();

        let mut ctx = WasiCtx {
            fds: HashMap::new(),
            args,
            env,
        };
        ctx.insert_existing_fd(0, dup(stdin().as_raw_fd()).unwrap());
        ctx.insert_existing_fd(1, dup(stdout().as_raw_fd()).unwrap());
        ctx.insert_existing_fd(2, dup(stderr().as_raw_fd()).unwrap());
        ctx
    }

    fn get_fd_entry(
        &self,
        fd: host::__wasi_fd_t,
        rights_base: host::__wasi_rights_t,
        rights_inheriting: host::__wasi_rights_t,
    ) -> Result<&FdEntry, host::__wasi_errno_t> {
        if let Some(ref fe) = self.fds.get(&fd) {
            // validate rights
            if !fe.rights_base & rights_base != 0 || !fe.rights_inheriting & rights_inheriting != 0
            {
                Err(host::__WASI_ENOTCAPABLE as host::__wasi_errno_t)
            } else {
                Ok(fe)
            }
        } else {
            Err(host::__WASI_EBADF as host::__wasi_errno_t)
        }
    }

    pub fn insert_existing_fd(&mut self, fd: host::__wasi_fd_t, rawfd: RawFd) {
        self.fds.insert(fd, unsafe { FdEntry::from_raw_fd(rawfd) });
    }
}

struct FdEntry {
    fd_object: FdObject,
    rights_base: host::__wasi_rights_t,
    rights_inheriting: host::__wasi_rights_t,
}

impl FromRawFd for FdEntry {
    unsafe fn from_raw_fd(rawfd: RawFd) -> FdEntry {
        let (ty, mut rights_base, rights_inheriting) = {
            let file = File::from_raw_fd(rawfd);
            let ft = file.metadata().unwrap().file_type();
            // we just make a `File` here for convenience; we don't want it to close when it drops
            std::mem::forget(file);
            if ft.is_block_device() {
                (
                    host::__WASI_FILETYPE_BLOCK_DEVICE,
                    host::RIGHTS_BLOCK_DEVICE_BASE,
                    host::RIGHTS_BLOCK_DEVICE_INHERITING,
                )
            } else if ft.is_char_device() {
                if nix::unistd::isatty(rawfd).unwrap() {
                    (
                        host::__WASI_FILETYPE_CHARACTER_DEVICE,
                        host::RIGHTS_TTY_BASE,
                        host::RIGHTS_TTY_BASE,
                    )
                } else {
                    (
                        host::__WASI_FILETYPE_CHARACTER_DEVICE,
                        host::RIGHTS_CHARACTER_DEVICE_BASE,
                        host::RIGHTS_CHARACTER_DEVICE_INHERITING,
                    )
                }
            } else if ft.is_dir() {
                (
                    host::__WASI_FILETYPE_DIRECTORY,
                    host::RIGHTS_DIRECTORY_BASE,
                    host::RIGHTS_DIRECTORY_INHERITING,
                )
            } else if ft.is_file() {
                (
                    host::__WASI_FILETYPE_REGULAR_FILE,
                    host::RIGHTS_REGULAR_FILE_BASE,
                    host::RIGHTS_REGULAR_FILE_INHERITING,
                )
            } else if ft.is_socket() {
                use nix::sys::socket;
                match socket::getsockopt(rawfd, socket::sockopt::SockType).unwrap() {
                    socket::SockType::Datagram => (
                        host::__WASI_FILETYPE_SOCKET_DGRAM,
                        host::RIGHTS_SOCKET_BASE,
                        host::RIGHTS_SOCKET_INHERITING,
                    ),
                    socket::SockType::Stream => (
                        host::__WASI_FILETYPE_SOCKET_STREAM,
                        host::RIGHTS_SOCKET_BASE,
                        host::RIGHTS_SOCKET_INHERITING,
                    ),
                    s => panic!("unsupported socket type: {:?}", s),
                }
            } else if ft.is_fifo() {
                (
                    host::__WASI_FILETYPE_SOCKET_STREAM,
                    host::RIGHTS_SOCKET_BASE,
                    host::RIGHTS_SOCKET_INHERITING,
                )
            } else {
                panic!("unsupported file type: {:?}", ft);
            }
        };

        use nix::fcntl::{fcntl, OFlag, F_GETFL};
        let flags_bits = fcntl(rawfd, F_GETFL).expect("fcntl succeeds");
        let flags = OFlag::from_bits_truncate(flags_bits);
        let accmode = flags & OFlag::O_ACCMODE;
        if accmode == OFlag::O_RDONLY {
            rights_base &= !host::__WASI_RIGHT_FD_WRITE as host::__wasi_rights_t;
        } else if accmode == OFlag::O_WRONLY {
            rights_base &= !host::__WASI_RIGHT_FD_READ as host::__wasi_rights_t;
        }

        FdEntry {
            fd_object: FdObject {
                ty: ty as u8,
                rawfd,
            },
            rights_base,
            rights_inheriting,
        }
    }
}

struct FdObject {
    ty: host::__wasi_filetype_t,
    rawfd: RawFd,
    // TODO: directories
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
    let errno = if let Some(ref fe) = ctx.fds.get(&host_fd) {
        host_fdstat.fs_filetype = fe.fd_object.ty;
        host_fdstat.fs_rights_base = fe.rights_base;
        host_fdstat.fs_rights_inheriting = fe.rights_inheriting;
        use nix::fcntl::{fcntl, OFlag, F_GETFL};
        match fcntl(fe.fd_object.rawfd, F_GETFL).map(OFlag::from_bits) {
            Ok(Some(flags)) => {
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
            Ok(None) => wasm32::__WASI_ENOSYS,
            Err(e) => wasm32::errno_from_nix(e.as_errno().unwrap()),
        }
    } else {
        return wasm32::__WASI_EBADF;
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
