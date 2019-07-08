#![allow(non_camel_case_types)]
#![allow(unused_unsafe)]

use crate::ctx::WasiCtx;
use crate::fdentry::{determine_type_rights, FdEntry};
use crate::memory::*;
use crate::{host, wasm32};

use super::fs_helpers::*;
use lucet_runtime::vmctx::Vmctx;

use nix::libc::{self, c_long, c_void, off_t};
use std::ffi::OsStr;
use std::mem::MaybeUninit;
use std::os::unix::prelude::{FromRawFd, OsStrExt};

pub fn wasi_fd_close(vmctx: &mut Vmctx, fd: wasm32::__wasi_fd_t) -> wasm32::__wasi_errno_t {
    let mut ctx = vmctx.get_embed_ctx_mut::<WasiCtx>();
    let fd = dec_fd(fd);
    if let Some(fdent) = ctx.fds.get(&fd) {
        // can't close preopened files
        if fdent.preopen_path.is_some() {
            return wasm32::__WASI_ENOTSUP;
        }
    }
    if let Some(mut fdent) = ctx.fds.remove(&fd) {
        fdent.fd_object.needs_close = false;
        match nix::unistd::close(fdent.fd_object.rawfd) {
            Ok(_) => wasm32::__WASI_ESUCCESS,
            Err(e) => wasm32::errno_from_nix(e.as_errno().unwrap()),
        }
    } else {
        wasm32::__WASI_EBADF
    }
}

pub fn wasi_fd_fdstat_get(
    vmctx: &mut Vmctx,
    fd: wasm32::__wasi_fd_t,
    fdstat_ptr: wasm32::uintptr_t, // *mut wasm32::__wasi_fdstat_t
) -> wasm32::__wasi_errno_t {
    let host_fd = dec_fd(fd);
    let mut host_fdstat = match dec_fdstat_byref(vmctx, fdstat_ptr) {
        Ok(host_fdstat) => host_fdstat,
        Err(e) => return enc_errno(e),
    };

    let ctx = vmctx.get_embed_ctx::<WasiCtx>();
    let errno = if let Some(fe) = ctx.fds.get(&host_fd) {
        host_fdstat.fs_filetype = fe.fd_object.ty;
        host_fdstat.fs_rights_base = fe.rights_base;
        host_fdstat.fs_rights_inheriting = fe.rights_inheriting;
        use nix::fcntl::{fcntl, OFlag, F_GETFL};
        match fcntl(fe.fd_object.rawfd, F_GETFL).map(OFlag::from_bits_truncate) {
            Ok(flags) => {
                host_fdstat.fs_flags = host::fdflags_from_nix(flags);
                wasm32::__WASI_ESUCCESS
            }
            Err(e) => wasm32::errno_from_nix(e.as_errno().unwrap()),
        }
    } else {
        wasm32::__WASI_EBADF
    };
    enc_fdstat_byref(vmctx, fdstat_ptr, host_fdstat)
        .expect("can write back into the pointer we read from");
    errno
}

pub fn wasi_fd_fdstat_set_flags(
    vmctx: &mut Vmctx,
    fd: wasm32::__wasi_fd_t,
    fdflags: wasm32::__wasi_fdflags_t,
) -> wasm32::__wasi_errno_t {
    let host_fd = dec_fd(fd);
    let host_fdflags = dec_fdflags(fdflags);
    let nix_flags = host::nix_from_fdflags(host_fdflags);

    let ctx = vmctx.get_embed_ctx::<WasiCtx>();
    if let Some(fe) = ctx.fds.get(&host_fd) {
        match nix::fcntl::fcntl(fe.fd_object.rawfd, nix::fcntl::F_SETFL(nix_flags)) {
            Ok(_) => wasm32::__WASI_ESUCCESS,
            Err(e) => wasm32::errno_from_nix(e.as_errno().unwrap()),
        }
    } else {
        wasm32::__WASI_EBADF
    }
}

pub fn wasi_fd_tell(
    vmctx: &mut Vmctx,
    fd: wasm32::__wasi_fd_t,
    offset: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let ctx = vmctx.get_embed_ctx::<WasiCtx>();
    let fd = dec_fd(fd);

    let host_offset = {
        use nix::unistd::{lseek, Whence};

        let rights = host::__WASI_RIGHT_FD_TELL;
        match ctx.get_fd_entry(fd, rights.into(), 0) {
            Ok(fe) => match lseek(fe.fd_object.rawfd, 0, Whence::SeekCur) {
                Ok(newoffset) => newoffset,
                Err(e) => return wasm32::errno_from_nix(e.as_errno().unwrap()),
            },
            Err(e) => return enc_errno(e),
        }
    };
    enc_filesize_byref(vmctx, offset, host_offset as u64)
        .map(|_| wasm32::__WASI_ESUCCESS)
        .unwrap_or_else(|e| e)
}

pub fn wasi_fd_seek(
    vmctx: &mut Vmctx,
    fd: wasm32::__wasi_fd_t,
    offset: wasm32::__wasi_filedelta_t,
    whence: wasm32::__wasi_whence_t,
    newoffset: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let ctx = vmctx.get_embed_ctx::<WasiCtx>();
    let fd = dec_fd(fd);
    let offset = dec_filedelta(offset);
    let whence = dec_whence(whence);

    let host_newoffset = {
        use nix::unistd::{lseek, Whence};
        let nwhence = match u32::from(whence) {
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
    enc_filesize_byref(vmctx, newoffset, host_newoffset as u64)
        .map(|_| wasm32::__WASI_ESUCCESS)
        .unwrap_or_else(|e| e)
}

pub fn wasi_fd_prestat_get(
    vmctx: &mut Vmctx,
    fd: wasm32::__wasi_fd_t,
    prestat_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let ctx = vmctx.get_embed_ctx::<WasiCtx>();
    let fd = dec_fd(fd);

    let rights = host::__WASI_RIGHT_PATH_OPEN;
    match ctx.get_fd_entry(fd, rights.into(), 0) {
        Ok(fe) => {
            if let Some(po_path) = &fe.preopen_path {
                if fe.fd_object.ty != host::__WASI_FILETYPE_DIRECTORY as host::__wasi_filetype_t {
                    return wasm32::__WASI_ENOTDIR;
                }
                enc_prestat_byref(
                    vmctx,
                    prestat_ptr,
                    host::__wasi_prestat_t {
                        pr_type: host::__WASI_PREOPENTYPE_DIR as host::__wasi_preopentype_t,
                        u: host::__wasi_prestat_t___wasi_prestat_u {
                            dir: host::__wasi_prestat_t___wasi_prestat_u___wasi_prestat_u_dir_t {
                                pr_name_len: po_path.as_os_str().as_bytes().len(),
                            },
                        },
                    },
                )
                .map(|_| wasm32::__WASI_ESUCCESS)
                .unwrap_or_else(|e| e)
            } else {
                wasm32::__WASI_ENOTSUP
            }
        }
        Err(e) => enc_errno(e),
    }
}

pub fn wasi_fd_prestat_dir_name(
    vmctx: &mut Vmctx,
    fd: wasm32::__wasi_fd_t,
    path_ptr: wasm32::uintptr_t,
    path_len: wasm32::size_t,
) -> wasm32::__wasi_errno_t {
    let ctx = vmctx.get_embed_ctx::<WasiCtx>();
    let fd = dec_fd(fd);
    let rights = host::__WASI_RIGHT_PATH_OPEN;
    match ctx.get_fd_entry(fd, rights.into(), 0) {
        Ok(fe) => {
            if let Some(po_path) = &fe.preopen_path {
                if fe.fd_object.ty != host::__WASI_FILETYPE_DIRECTORY as host::__wasi_filetype_t {
                    return wasm32::__WASI_ENOTDIR;
                }
                let path_bytes = po_path.as_os_str().as_bytes();
                if path_bytes.len() > dec_usize(path_len) {
                    return wasm32::__WASI_ENAMETOOLONG;
                }
                enc_slice_of(vmctx, path_bytes, path_ptr)
                    .map(|_| wasm32::__WASI_ESUCCESS)
                    .unwrap_or_else(|e| e)
            } else {
                wasm32::__WASI_ENOTSUP
            }
        }
        Err(e) => enc_errno(e),
    }
}

pub fn wasi_fd_read(
    vmctx: &mut Vmctx,
    fd: wasm32::__wasi_fd_t,
    iovs_ptr: wasm32::uintptr_t,
    iovs_len: wasm32::size_t,
    nread: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    use nix::sys::uio::{readv, IoVec};

    let fd = dec_fd(fd);
    let mut iovs = match dec_iovec_slice(vmctx, iovs_ptr, iovs_len) {
        Ok(iovs) => iovs,
        Err(e) => return enc_errno(e),
    };

    let mut ctx = vmctx.get_embed_ctx_mut::<WasiCtx>();
    let rights = host::__WASI_RIGHT_FD_READ;
    let fe = match ctx.get_fd_entry(fd, rights.into(), 0) {
        Ok(fe) => fe,
        Err(e) => return enc_errno(e),
    };

    let mut iovs: Vec<IoVec<&mut [u8]>> = iovs
        .iter_mut()
        .map(|iov| unsafe { host::iovec_to_nix_mut(iov) })
        .collect();

    let host_nread = match readv(fe.fd_object.rawfd, &mut iovs) {
        Ok(len) => len,
        Err(e) => return wasm32::errno_from_nix(e.as_errno().unwrap()),
    };

    if host_nread == 0 {
        // we hit eof, so remove the fdentry from the context
        let mut fe = ctx.fds.remove(&fd).expect("file entry is still there");
        fe.fd_object.needs_close = false;
    }
    enc_usize_byref(vmctx, nread, host_nread)
        .map(|_| wasm32::__WASI_ESUCCESS)
        .unwrap_or_else(|e| e)
}

pub fn wasi_fd_write(
    vmctx: &mut Vmctx,
    fd: wasm32::__wasi_fd_t,
    iovs_ptr: wasm32::uintptr_t,
    iovs_len: wasm32::size_t,
    nwritten: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    use nix::sys::uio::{writev, IoVec};

    let fd = dec_fd(fd);
    let iovs = match dec_ciovec_slice(vmctx, iovs_ptr, iovs_len) {
        Ok(iovs) => iovs,
        Err(e) => return enc_errno(e),
    };

    let ctx = vmctx.get_embed_ctx::<WasiCtx>();
    let rights = host::__WASI_RIGHT_FD_WRITE;
    let fe = match ctx.get_fd_entry(fd, rights.into(), 0) {
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
    enc_usize_byref(vmctx, nwritten, host_nwritten)
        .map(|_| wasm32::__WASI_ESUCCESS)
        .unwrap_or_else(|e| e)
}

pub fn wasi_path_open(
    vmctx: &mut Vmctx,
    dirfd: wasm32::__wasi_fd_t,
    dirflags: wasm32::__wasi_lookupflags_t,
    path_ptr: wasm32::uintptr_t,
    path_len: wasm32::size_t,
    oflags: wasm32::__wasi_oflags_t,
    fs_rights_base: wasm32::__wasi_rights_t,
    fs_rights_inheriting: wasm32::__wasi_rights_t,
    fs_flags: wasm32::__wasi_fdflags_t,
    fd_out_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    use nix::errno::Errno;
    use nix::fcntl::{openat, AtFlags, OFlag};
    use nix::sys::stat::{fstatat, Mode, SFlag};

    let dirfd = dec_fd(dirfd);
    let dirflags = dec_lookupflags(dirflags);
    let oflags = dec_oflags(oflags);
    let fs_rights_base = dec_rights(fs_rights_base);
    let fs_rights_inheriting = dec_rights(fs_rights_inheriting);
    let fs_flags = dec_fdflags(fs_flags);

    // which open mode do we need?
    let read = fs_rights_base
        & ((host::__WASI_RIGHT_FD_READ | host::__WASI_RIGHT_FD_READDIR) as host::__wasi_rights_t)
        != 0;
    let write = fs_rights_base
        & ((host::__WASI_RIGHT_FD_DATASYNC
            | host::__WASI_RIGHT_FD_WRITE
            | host::__WASI_RIGHT_FD_ALLOCATE
            | host::__WASI_RIGHT_FD_FILESTAT_SET_SIZE) as host::__wasi_rights_t)
        != 0;

    let mut nix_all_oflags = if read && write {
        OFlag::O_RDWR
    } else if read {
        OFlag::O_RDONLY
    } else {
        OFlag::O_WRONLY
    };

    // on non-Capsicum systems, we always want nofollow
    nix_all_oflags.insert(OFlag::O_NOFOLLOW);

    // which rights are needed on the dirfd?
    let mut needed_base = host::__WASI_RIGHT_PATH_OPEN as host::__wasi_rights_t;
    let mut needed_inheriting = fs_rights_base | fs_rights_inheriting;

    // convert open flags
    let nix_oflags = host::nix_from_oflags(oflags);
    nix_all_oflags.insert(nix_oflags);
    if nix_all_oflags.contains(OFlag::O_CREAT) {
        needed_base |= host::__WASI_RIGHT_PATH_CREATE_FILE as host::__wasi_rights_t;
    }
    if nix_all_oflags.contains(OFlag::O_TRUNC) {
        needed_base |= host::__WASI_RIGHT_PATH_FILESTAT_SET_SIZE as host::__wasi_rights_t;
    }

    // convert file descriptor flags
    nix_all_oflags.insert(host::nix_from_fdflags(fs_flags));
    if nix_all_oflags.contains(OFlag::O_DSYNC) {
        needed_inheriting |= host::__WASI_RIGHT_FD_DATASYNC as host::__wasi_rights_t;
    }
    if nix_all_oflags.intersects(O_RSYNC | OFlag::O_SYNC) {
        needed_inheriting |= host::__WASI_RIGHT_FD_SYNC as host::__wasi_rights_t;
    }
    if nix_all_oflags.contains(OFlag::O_DIRECTORY) {
        nix_all_oflags.remove(OFlag::O_RDWR);
        nix_all_oflags.remove(OFlag::O_WRONLY);
        nix_all_oflags.insert(OFlag::O_RDONLY);
    }
    let path = match dec_slice_of::<u8>(vmctx, path_ptr, path_len) {
        Ok(path_bytes) => OsStr::from_bytes(path_bytes),
        Err(e) => return enc_errno(e),
    };

    let (dir, path) = match path_get(
        &vmctx,
        dirfd,
        dirflags,
        path,
        needed_base,
        needed_inheriting,
        nix_oflags.contains(OFlag::O_CREAT),
    ) {
        Ok((dir, path)) => (dir, path),
        Err(e) => return enc_errno(e),
    };

    // Call openat. Use mode 0o666 so that we follow whatever the user's
    // umask is, but don't set the executable flag, because it isn't yet
    // meaningful for WASI programs to create executable files.
    let new_fd = match openat(
        dir,
        path.as_os_str(),
        nix_all_oflags,
        Mode::from_bits_truncate(0o666),
    ) {
        Ok(fd) => fd,
        Err(e) => {
            match e.as_errno() {
                // Linux returns ENXIO instead of EOPNOTSUPP when opening a socket
                Some(Errno::ENXIO) => {
                    if let Ok(stat) = fstatat(dir, path.as_os_str(), AtFlags::AT_SYMLINK_NOFOLLOW) {
                        if SFlag::from_bits_truncate(stat.st_mode).contains(SFlag::S_IFSOCK) {
                            return wasm32::__WASI_ENOTSUP;
                        } else {
                            return wasm32::__WASI_ENXIO;
                        }
                    } else {
                        return wasm32::__WASI_ENXIO;
                    }
                }
                // Linux returns ENOTDIR instead of ELOOP when using O_NOFOLLOW|O_DIRECTORY
                // on a symlink.
                Some(Errno::ENOTDIR)
                    if !(nix_all_oflags & (OFlag::O_NOFOLLOW | OFlag::O_DIRECTORY)).is_empty() =>
                {
                    if let Ok(stat) = fstatat(dir, path.as_os_str(), AtFlags::AT_SYMLINK_NOFOLLOW) {
                        if SFlag::from_bits_truncate(stat.st_mode).contains(SFlag::S_IFLNK) {
                            return wasm32::__WASI_ELOOP;
                        }
                    }
                    return wasm32::__WASI_ENOTDIR;
                }
                // FreeBSD returns EMLINK instead of ELOOP when using O_NOFOLLOW on
                // a symlink.
                Some(Errno::EMLINK) if !(nix_all_oflags & OFlag::O_NOFOLLOW).is_empty() => {
                    return wasm32::__WASI_ELOOP;
                }
                Some(e) => return wasm32::errno_from_nix(e),
                None => return wasm32::__WASI_ENOSYS,
            }
        }
    };

    // Determine the type of the new file descriptor and which rights contradict with this type
    let guest_fd = match unsafe { determine_type_rights(new_fd) } {
        Err(e) => {
            // if `close` fails, note it but do not override the underlying errno
            nix::unistd::close(new_fd).unwrap_or_else(|e| {
                dbg!(e);
            });
            if let Err(e) = enc_fd_byref(vmctx, fd_out_ptr, wasm32::__wasi_fd_t::max_value()) {
                return enc_errno(e);
            }
            return enc_errno(e);
        }
        Ok((_ty, max_base, max_inheriting)) => {
            let mut fe = unsafe { FdEntry::from_raw_fd(new_fd) };
            fe.rights_base &= max_base;
            fe.rights_inheriting &= max_inheriting;
            match vmctx.get_embed_ctx_mut::<WasiCtx>().insert_fd_entry(fe) {
                Ok(fd) => fd,
                Err(e) => return enc_errno(e),
            }
        }
    };
    enc_fd_byref(vmctx, fd_out_ptr, guest_fd)
        .map(|_| wasm32::__WASI_ESUCCESS)
        .unwrap_or_else(|e| e)
}

pub fn wasi_fd_filestat_get(
    vmctx: &mut Vmctx,
    fd: wasm32::__wasi_fd_t,
    filestat_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    use nix::sys::stat::fstat;

    let host_fd = dec_fd(fd);
    let ctx = vmctx.get_embed_ctx::<WasiCtx>();

    let rights = host::__WASI_RIGHT_FD_FILESTAT_GET;
    match ctx.get_fd_entry(host_fd, rights.into(), 0) {
        Ok(fe) => match fstat(fe.fd_object.rawfd) {
            Err(e) => return wasm32::errno_from_nix(e.as_errno().unwrap()),
            Ok(filestat) => {
                let host_filestat = host::filestat_from_nix(filestat);
                enc_filestat_byref(vmctx, filestat_ptr, host_filestat)
                    .expect("can write into the pointer");
            }
        },
        Err(e) => return enc_errno(e),
    }
    wasm32::__WASI_ESUCCESS
}

pub fn wasi_path_filestat_get(
    vmctx: &mut Vmctx,
    dirfd: wasm32::__wasi_fd_t,
    dirflags: wasm32::__wasi_lookupflags_t,
    path_ptr: wasm32::uintptr_t,
    path_len: wasm32::size_t,
    filestat_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    use nix::fcntl::AtFlags;
    use nix::sys::stat::fstatat;

    let dirfd = dec_fd(dirfd);
    let dirflags = dec_lookupflags(dirflags);
    let path = match dec_slice_of::<u8>(vmctx, path_ptr, path_len) {
        Ok(path_bytes) => OsStr::from_bytes(path_bytes),
        Err(e) => return enc_errno(e),
    };
    let rights = host::__WASI_RIGHT_PATH_FILESTAT_GET;
    let (dir, path) = match path_get(&vmctx, dirfd, dirflags, path, rights.into(), 0, false) {
        Ok((dir, path)) => (dir, path),
        Err(e) => return enc_errno(e),
    };
    let atflags = match dirflags {
        wasm32::__WASI_LOOKUP_SYMLINK_FOLLOW => AtFlags::empty(),
        _ => AtFlags::AT_SYMLINK_NOFOLLOW,
    };
    match fstatat(dir, path.as_os_str(), atflags) {
        Err(e) => wasm32::errno_from_nix(e.as_errno().unwrap()),
        Ok(filestat) => {
            let host_filestat = host::filestat_from_nix(filestat);
            enc_filestat_byref(vmctx, filestat_ptr, host_filestat)
                .expect("can write into the pointer");
            wasm32::__WASI_ESUCCESS
        }
    }
}

pub fn wasi_path_create_directory(
    vmctx: &mut Vmctx,
    dirfd: wasm32::__wasi_fd_t,
    path_ptr: wasm32::uintptr_t,
    path_len: wasm32::size_t,
) -> wasm32::__wasi_errno_t {
    use nix::errno;
    use nix::libc::mkdirat;

    let dirfd = dec_fd(dirfd);
    let path = match dec_slice_of::<u8>(vmctx, path_ptr, path_len) {
        Ok(path_bytes) => OsStr::from_bytes(path_bytes),
        Err(e) => return enc_errno(e),
    };
    let rights = host::__WASI_RIGHT_PATH_CREATE_DIRECTORY;
    let (dir, path) = match path_get(&vmctx, dirfd, 0, path, rights.into(), 0, false) {
        Ok((dir, path)) => (dir, path),
        Err(e) => return enc_errno(e),
    };
    let path_cstr = match std::ffi::CString::new(path.as_os_str().as_bytes()) {
        Ok(path_cstr) => path_cstr,
        Err(_) => return wasm32::__WASI_EINVAL,
    };
    // nix doesn't expose mkdirat() yet
    match unsafe { mkdirat(dir, path_cstr.as_ptr(), 0o777) } {
        0 => wasm32::__WASI_ESUCCESS,
        _ => wasm32::errno_from_nix(errno::Errno::last()),
    }
}

pub fn wasi_path_unlink_file(
    vmctx: &mut Vmctx,
    dirfd: wasm32::__wasi_fd_t,
    path_ptr: wasm32::uintptr_t,
    path_len: wasm32::size_t,
) -> wasm32::__wasi_errno_t {
    use nix::errno;
    use nix::libc::unlinkat;

    let dirfd = dec_fd(dirfd);
    let path = match dec_slice_of::<u8>(vmctx, path_ptr, path_len) {
        Ok(path_bytes) => OsStr::from_bytes(path_bytes),
        Err(e) => return enc_errno(e),
    };
    let rights = host::__WASI_RIGHT_PATH_UNLINK_FILE;
    let (dir, path) = match path_get(&vmctx, dirfd, 0, path, rights.into(), 0, false) {
        Ok((dir, path)) => (dir, path),
        Err(e) => return enc_errno(e),
    };
    let path_cstr = match std::ffi::CString::new(path.as_os_str().as_bytes()) {
        Ok(path_cstr) => path_cstr,
        Err(_) => return wasm32::__WASI_EINVAL,
    };
    // nix doesn't expose unlinkat() yet
    match unsafe { unlinkat(dir, path_cstr.as_ptr(), 0) } {
        0 => wasm32::__WASI_ESUCCESS,
        _ => {
            let mut e = errno::Errno::last();
            // Non-Linux implementations may return EPERM when attempting to remove a
            // directory without `REMOVEDIR`. For WASI, adjust this to `EISDIR`.
            #[cfg(not(linux))]
            {
                use nix::fcntl::AtFlags;
                use nix::sys::stat::{fstatat, SFlag};
                if e == errno::Errno::EPERM {
                    if let Ok(stat) = fstatat(dir, path.as_os_str(), AtFlags::AT_SYMLINK_NOFOLLOW) {
                        if SFlag::from_bits_truncate(stat.st_mode).contains(SFlag::S_IFDIR) {
                            e = errno::Errno::EISDIR;
                        }
                    } else {
                        e = errno::Errno::last();
                    }
                }
            }
            wasm32::errno_from_nix(e)
        }
    }
}

pub fn wasi_fd_allocate(
    vmctx: &mut Vmctx,
    fd: wasm32::__wasi_fd_t,
    offset: wasm32::__wasi_filesize_t,
    len: wasm32::__wasi_filesize_t,
) -> wasm32::__wasi_errno_t {
    let host_fd = dec_fd(fd);
    let ctx = vmctx.get_embed_ctx::<WasiCtx>();
    let rights = host::__WASI_RIGHT_FD_ALLOCATE;
    let fe = match ctx.get_fd_entry(host_fd, rights.into(), 0) {
        Ok(fe) => fe,
        Err(e) => return enc_errno(e),
    };
    let offset = dec_filesize(offset);
    let len = dec_filesize(len);

    #[cfg(target_os = "linux")]
    {
        let res =
            unsafe { libc::posix_fallocate(fe.fd_object.rawfd, offset as off_t, len as off_t) };
        if res != 0 {
            return wasm32::errno_from_nix(nix::errno::Errno::last());
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        use nix::sys::stat::fstat;
        use nix::unistd::ftruncate;

        match fstat(fe.fd_object.rawfd) {
            Err(e) => return wasm32::errno_from_nix(e.as_errno().unwrap()),
            Ok(st) => {
                let current_size = st.st_size as u64;
                let wanted_size = match offset.checked_add(len) {
                    Some(wanted_size) => wanted_size,
                    None => return wasm32::__WASI_E2BIG,
                };
                if wanted_size > i64::max_value() as u64 {
                    return wasm32::__WASI_E2BIG;
                }
                if wanted_size > current_size {
                    if let Err(e) = ftruncate(fe.fd_object.rawfd, wanted_size as off_t) {
                        return wasm32::errno_from_nix(e.as_errno().unwrap());
                    }
                }
            }
        }
    }

    wasm32::__WASI_ESUCCESS
}

pub fn wasi_fd_advise(
    vmctx: &mut Vmctx,
    fd: wasm32::__wasi_fd_t,
    offset: wasm32::__wasi_filesize_t,
    len: wasm32::__wasi_filesize_t,
    advice: wasm32::__wasi_advice_t,
) -> wasm32::__wasi_errno_t {
    let host_fd = dec_fd(fd);
    let ctx = vmctx.get_embed_ctx::<WasiCtx>();
    let rights = host::__WASI_RIGHT_FD_ADVISE;
    let fe = match ctx.get_fd_entry(host_fd, rights.into(), 0) {
        Ok(fe) => fe,
        Err(e) => return enc_errno(e),
    };
    let advice = dec_advice(advice);

    #[cfg(target_os = "linux")]
    {
        let host_advice = match advice as u32 {
            host::__WASI_ADVICE_DONTNEED => libc::POSIX_FADV_DONTNEED,
            host::__WASI_ADVICE_SEQUENTIAL => libc::POSIX_FADV_SEQUENTIAL,
            host::__WASI_ADVICE_WILLNEED => libc::POSIX_FADV_DONTNEED,
            host::__WASI_ADVICE_NOREUSE => libc::POSIX_FADV_NOREUSE,
            host::__WASI_ADVICE_RANDOM => libc::POSIX_FADV_RANDOM,
            host::__WASI_ADVICE_NORMAL => libc::POSIX_FADV_NORMAL,
            _ => return wasm32::__WASI_EINVAL,
        };
        let offset = dec_filesize(offset);
        let len = dec_filesize(len);
        let res = unsafe {
            libc::posix_fadvise(
                fe.fd_object.rawfd,
                offset as off_t,
                len as off_t,
                host_advice,
            )
        };
        if res != 0 {
            return wasm32::errno_from_nix(nix::errno::Errno::last());
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = (fe, offset, len);
        match advice as u32 {
            host::__WASI_ADVICE_DONTNEED
            | host::__WASI_ADVICE_SEQUENTIAL
            | host::__WASI_ADVICE_WILLNEED
            | host::__WASI_ADVICE_NOREUSE
            | host::__WASI_ADVICE_RANDOM
            | host::__WASI_ADVICE_NORMAL => {}
            _ => return wasm32::__WASI_EINVAL,
        }
    }

    wasm32::__WASI_ESUCCESS
}

pub fn wasi_fd_datasync(vmctx: &mut Vmctx, fd: wasm32::__wasi_fd_t) -> wasm32::__wasi_errno_t {
    let host_fd = dec_fd(fd);
    let ctx = vmctx.get_embed_ctx::<WasiCtx>();
    let rights = host::__WASI_RIGHT_FD_DATASYNC;
    let fe = match ctx.get_fd_entry(host_fd, rights.into(), 0) {
        Ok(fe) => fe,
        Err(e) => return enc_errno(e),
    };
    let res;

    #[cfg(target_os = "linux")]
    {
        res = nix::unistd::fdatasync(fe.fd_object.rawfd);
    }

    #[cfg(not(target_os = "linux"))]
    {
        res = nix::unistd::fsync(fe.fd_object.rawfd);
    }

    if let Err(e) = res {
        return wasm32::errno_from_nix(e.as_errno().unwrap());
    }
    wasm32::__WASI_ESUCCESS
}

pub fn wasi_fd_sync(vmctx: &mut Vmctx, fd: wasm32::__wasi_fd_t) -> wasm32::__wasi_errno_t {
    let host_fd = dec_fd(fd);
    let ctx = vmctx.get_embed_ctx::<WasiCtx>();
    let rights = host::__WASI_RIGHT_FD_SYNC;
    let fe = match ctx.get_fd_entry(host_fd, rights.into(), 0) {
        Ok(fe) => fe,
        Err(e) => return enc_errno(e),
    };
    let res = nix::unistd::fsync(fe.fd_object.rawfd);
    if let Err(e) = res {
        return wasm32::errno_from_nix(e.as_errno().unwrap());
    }
    wasm32::__WASI_ESUCCESS
}

pub fn wasi_fd_fdstat_set_rights(
    vmctx: &mut Vmctx,
    fd: wasm32::__wasi_fd_t,
    fs_rights_base: wasm32::__wasi_rights_t,
    fs_rights_inheriting: wasm32::__wasi_rights_t,
) -> wasm32::__wasi_errno_t {
    let host_fd = dec_fd(fd);
    let mut ctx = vmctx.get_embed_ctx_mut::<WasiCtx>();
    let fe = match ctx.fds.get_mut(&host_fd) {
        Some(fe) => fe,
        None => return wasm32::__WASI_EBADF,
    };
    if fe.rights_base & fs_rights_base != fs_rights_base
        || fe.rights_inheriting & fs_rights_inheriting != fs_rights_inheriting
    {
        return wasm32::__WASI_ENOTCAPABLE;
    }
    fe.rights_base = fs_rights_base;
    fe.rights_inheriting = fs_rights_inheriting;
    wasm32::__WASI_ESUCCESS
}

pub fn wasi_fd_filestat_set_size(
    vmctx: &mut Vmctx,
    fd: wasm32::__wasi_fd_t,
    st_size: wasm32::__wasi_filesize_t,
) -> wasm32::__wasi_errno_t {
    use nix::unistd::ftruncate;

    let host_fd = dec_fd(fd);
    let ctx = vmctx.get_embed_ctx::<WasiCtx>();
    let rights = host::__WASI_RIGHT_FD_FILESTAT_SET_SIZE;
    let fe = match ctx.get_fd_entry(host_fd, rights.into(), 0) {
        Ok(fe) => fe,
        Err(e) => return enc_errno(e),
    };
    let st_size = dec_filesize(st_size);
    if st_size > i64::max_value() as u64 {
        return wasm32::__WASI_E2BIG;
    }
    if let Err(e) = ftruncate(fe.fd_object.rawfd, st_size as off_t) {
        return wasm32::errno_from_nix(e.as_errno().unwrap());
    }
    wasm32::__WASI_ESUCCESS
}

pub fn wasi_fd_filestat_set_times(
    vmctx: &mut Vmctx,
    fd: wasm32::__wasi_fd_t,
    st_atim: wasm32::__wasi_timestamp_t,
    st_mtim: wasm32::__wasi_timestamp_t,
    fst_flags: wasm32::__wasi_fstflags_t,
) -> wasm32::__wasi_errno_t {
    use nix::sys::time::{TimeSpec, TimeValLike};

    let host_fd = dec_fd(fd);
    let ctx = vmctx.get_embed_ctx::<WasiCtx>();
    let rights = host::__WASI_RIGHT_FD_FILESTAT_SET_TIMES;
    let fe = match ctx.get_fd_entry(host_fd, rights.into(), 0) {
        Ok(fe) => fe,
        Err(e) => return enc_errno(e),
    };
    let st_atim = dec_timestamp(st_atim);
    let mut st_mtim = dec_timestamp(st_mtim);
    let fst_flags = dec_fstflags(fst_flags);
    if fst_flags & (host::__WASI_FILESTAT_SET_MTIM_NOW as host::__wasi_fstflags_t) != 0 {
        let clock_id = libc::CLOCK_REALTIME;
        let mut timespec = MaybeUninit::<libc::timespec>::uninit();
        let res = unsafe { libc::clock_gettime(clock_id, timespec.as_mut_ptr()) };
        if res != 0 {
            return wasm32::errno_from_nix(nix::errno::Errno::last());
        }
        let timespec = unsafe { timespec.assume_init() };
        let time_ns = match (timespec.tv_sec as host::__wasi_timestamp_t)
            .checked_mul(1_000_000_000)
            .and_then(|sec_ns| sec_ns.checked_add(timespec.tv_nsec as host::__wasi_timestamp_t))
        {
            Some(time_ns) => time_ns,
            None => return wasm32::__WASI_EOVERFLOW,
        };
        st_mtim = time_ns;
    }
    let ts_atime = match fst_flags as u32 {
        f if f & host::__WASI_FILESTAT_SET_ATIM_NOW != 0 => libc::timespec {
            tv_sec: 0,
            tv_nsec: utime_now(),
        },
        f if f & host::__WASI_FILESTAT_SET_ATIM != 0 => {
            *TimeSpec::nanoseconds(st_atim as i64).as_ref()
        }
        _ => libc::timespec {
            tv_sec: 0,
            tv_nsec: utime_omit(),
        },
    };
    let ts_mtime = *TimeSpec::nanoseconds(st_mtim as i64).as_ref();
    let times = [ts_atime, ts_mtime];
    let res = unsafe { libc::futimens(fe.fd_object.rawfd, times.as_ptr()) };
    if res != 0 {
        return wasm32::errno_from_nix(nix::errno::Errno::last());
    }
    wasm32::__WASI_ESUCCESS
}

pub fn wasi_path_filestat_set_times(
    vmctx: &mut Vmctx,
    dirfd: wasm32::__wasi_fd_t,
    dirflags: wasm32::__wasi_lookupflags_t,
    path_ptr: wasm32::uintptr_t,
    path_len: wasm32::size_t,
    st_atim: wasm32::__wasi_timestamp_t,
    st_mtim: wasm32::__wasi_timestamp_t,
    fst_flags: wasm32::__wasi_fstflags_t,
) -> wasm32::__wasi_errno_t {
    use nix::sys::time::{TimeSpec, TimeValLike};

    let dirfd = dec_fd(dirfd);
    let dirflags = dec_lookupflags(dirflags);
    let path = match dec_slice_of::<u8>(vmctx, path_ptr, path_len) {
        Ok(path_bytes) => OsStr::from_bytes(path_bytes),
        Err(e) => return enc_errno(e),
    };
    let rights = host::__WASI_RIGHT_PATH_FILESTAT_SET_TIMES;
    let (dir, path) = match path_get(&vmctx, dirfd, dirflags, path, rights.into(), 0, false) {
        Ok((dir, path)) => (dir, path),
        Err(e) => return enc_errno(e),
    };
    let atflags = match dirflags {
        wasm32::__WASI_LOOKUP_SYMLINK_FOLLOW => 0,
        _ => libc::AT_SYMLINK_NOFOLLOW,
    };
    let st_atim = dec_timestamp(st_atim);
    let mut st_mtim = dec_timestamp(st_mtim);
    let fst_flags = dec_fstflags(fst_flags);
    if fst_flags & (host::__WASI_FILESTAT_SET_MTIM_NOW as host::__wasi_fstflags_t) != 0 {
        let clock_id = libc::CLOCK_REALTIME;
        let mut timespec = MaybeUninit::<libc::timespec>::uninit();
        let res = unsafe { libc::clock_gettime(clock_id, timespec.as_mut_ptr()) };
        if res != 0 {
            return wasm32::errno_from_nix(nix::errno::Errno::last());
        }
        let timespec = unsafe { timespec.assume_init() };
        let time_ns = match (timespec.tv_sec as host::__wasi_timestamp_t)
            .checked_mul(1_000_000_000)
            .and_then(|sec_ns| sec_ns.checked_add(timespec.tv_nsec as host::__wasi_timestamp_t))
        {
            Some(time_ns) => time_ns,
            None => return wasm32::__WASI_EOVERFLOW,
        };
        st_mtim = time_ns;
    }
    let ts_atime = match fst_flags as u32 {
        f if f & host::__WASI_FILESTAT_SET_ATIM_NOW != 0 => libc::timespec {
            tv_sec: 0,
            tv_nsec: utime_now(),
        },
        f if f & host::__WASI_FILESTAT_SET_ATIM != 0 => {
            *TimeSpec::nanoseconds(st_atim as i64).as_ref()
        }
        _ => libc::timespec {
            tv_sec: 0,
            tv_nsec: utime_omit(),
        },
    };
    let ts_mtime = *TimeSpec::nanoseconds(st_mtim as i64).as_ref();
    let times = [ts_atime, ts_mtime];
    let path_cstr = match std::ffi::CString::new(path.as_os_str().as_bytes()) {
        Ok(path_cstr) => path_cstr,
        Err(_) => return wasm32::__WASI_EINVAL,
    };
    let res = unsafe { libc::utimensat(dir, path_cstr.as_ptr(), times.as_ptr(), atflags) };
    if res != 0 {
        return wasm32::errno_from_nix(nix::errno::Errno::last());
    }
    wasm32::__WASI_ESUCCESS
}

pub fn wasi_fd_pread(
    vmctx: &mut Vmctx,
    fd: wasm32::__wasi_fd_t,
    iovs_ptr: wasm32::uintptr_t,
    iovs_len: wasm32::size_t,
    offset: wasm32::__wasi_filesize_t,
    nread: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    use nix::sys::uio::pread;
    use std::cmp;

    let fd = dec_fd(fd);
    let iovs = match dec_iovec_slice(vmctx, iovs_ptr, iovs_len) {
        Ok(iovs) => iovs,
        Err(e) => return enc_errno(e),
    };
    let ctx = vmctx.get_embed_ctx::<WasiCtx>();
    let rights = host::__WASI_RIGHT_FD_READ;
    let fe = match ctx.get_fd_entry(fd, rights.into(), 0) {
        Ok(fe) => fe,
        Err(e) => return enc_errno(e),
    };
    let offset = dec_filesize(offset);
    if offset > i64::max_value() as u64 {
        return wasm32::__WASI_EIO;
    }
    let buf_size = iovs.iter().map(|v| v.buf_len).sum();
    let mut buf = vec![0; buf_size];
    let host_nread = match pread(fe.fd_object.rawfd, &mut buf, offset as off_t) {
        Ok(len) => len,
        Err(e) => return wasm32::errno_from_nix(e.as_errno().unwrap()),
    };
    let mut buf_offset = 0;
    let mut left = host_nread;
    for iov in &iovs {
        if left == 0 {
            break;
        }
        let vec_len = cmp::min(iov.buf_len, left);
        unsafe { std::slice::from_raw_parts_mut(iov.buf as *mut u8, vec_len) }
            .copy_from_slice(&buf[buf_offset..buf_offset + vec_len]);
        buf_offset += vec_len;
        left -= vec_len;
    }
    enc_usize_byref(vmctx, nread, host_nread)
        .map(|_| wasm32::__WASI_ESUCCESS)
        .unwrap_or_else(|e| e)
}

pub fn wasi_fd_pwrite(
    vmctx: &mut Vmctx,
    fd: wasm32::__wasi_fd_t,
    iovs_ptr: wasm32::uintptr_t,
    iovs_len: wasm32::size_t,
    offset: wasm32::__wasi_filesize_t,
    nwritten: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    use nix::sys::uio::pwrite;

    let fd = dec_fd(fd);
    let iovs = match dec_iovec_slice(vmctx, iovs_ptr, iovs_len) {
        Ok(iovs) => iovs,
        Err(e) => return enc_errno(e),
    };
    let ctx = vmctx.get_embed_ctx::<WasiCtx>();
    let rights = host::__WASI_RIGHT_FD_READ;
    let fe = match ctx.get_fd_entry(fd, rights.into(), 0) {
        Ok(fe) => fe,
        Err(e) => return enc_errno(e),
    };
    let offset = dec_filesize(offset);
    if offset > i64::max_value() as u64 {
        return wasm32::__WASI_EIO;
    }
    let buf_size = iovs.iter().map(|v| v.buf_len).sum();
    let mut buf = Vec::with_capacity(buf_size);
    for iov in &iovs {
        buf.extend_from_slice(unsafe {
            std::slice::from_raw_parts(iov.buf as *const u8, iov.buf_len)
        });
    }
    let host_nwritten = match pwrite(fe.fd_object.rawfd, &buf, offset as off_t) {
        Ok(len) => len,
        Err(e) => return wasm32::errno_from_nix(e.as_errno().unwrap()),
    };
    enc_usize_byref(vmctx, nwritten, host_nwritten)
        .map(|_| wasm32::__WASI_ESUCCESS)
        .unwrap_or_else(|e| e)
}

pub fn wasi_fd_readdir(
    vmctx: &mut Vmctx,
    fd: wasm32::__wasi_fd_t,
    buf: wasm32::uintptr_t,
    buf_len: wasm32::size_t,
    cookie: wasm32::__wasi_dircookie_t,
    bufused: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    use libc::{dirent, fdopendir, readdir_r, seekdir};

    match enc_usize_byref(vmctx, bufused, 0) {
        Ok(_) => {}
        Err(e) => return enc_errno(e),
    };
    let fd = dec_fd(fd);
    let ctx = vmctx.get_embed_ctx::<WasiCtx>();
    let rights = host::__WASI_RIGHT_FD_READDIR;
    let fe = match ctx.get_fd_entry(fd, rights.into(), 0) {
        Ok(fe) => fe,
        Err(e) => return enc_errno(e),
    };
    let host_buf = match dec_slice_of::<u8>(vmctx, buf, buf_len) {
        Ok(host_buf) => host_buf,
        Err(e) => return enc_errno(e),
    };
    let host_buf_ptr = host_buf.as_ptr();
    let host_buf_len = host_buf.len();
    let dir = unsafe { fdopendir(fe.fd_object.rawfd) };
    if dir.is_null() {
        return wasm32::errno_from_nix(nix::errno::Errno::last());
    }
    let cookie = dec_dircookie(cookie);
    if cookie != wasm32::__WASI_DIRCOOKIE_START {
        unsafe { seekdir(dir, cookie as c_long) };
    }
    let mut entry_buf = MaybeUninit::<dirent>::uninit();
    let mut left = host_buf_len;
    let mut host_buf_offset: usize = 0;
    while left > 0 {
        let mut host_entry: *mut dirent = std::ptr::null_mut();
        let res = unsafe { readdir_r(dir, entry_buf.as_mut_ptr(), &mut host_entry) };
        if res == -1 {
            return wasm32::errno_from_nix(nix::errno::Errno::last());
        }
        if host_entry.is_null() {
            break;
        }
        unsafe { entry_buf.assume_init() };
        let entry: wasm32::__wasi_dirent_t = match dirent_from_host(&unsafe { *host_entry }) {
            Ok(entry) => entry,
            Err(e) => return enc_errno(e),
        };
        let name_len = entry.d_namlen as usize;
        let required_space = std::mem::size_of_val(&entry) + name_len;
        if required_space > left {
            break;
        }
        unsafe {
            let ptr = host_buf_ptr.offset(host_buf_offset as isize) as *mut c_void
                as *mut wasm32::__wasi_dirent_t;
            *ptr = entry;
        }
        host_buf_offset += std::mem::size_of_val(&entry);
        let name_ptr = unsafe { *host_entry }.d_name.as_ptr();
        unsafe {
            std::ptr::copy_nonoverlapping(
                name_ptr as *const _,
                host_buf_ptr.offset(host_buf_offset as isize) as *mut _,
                name_len,
            )
        };
        host_buf_offset += name_len;
        left -= required_space;
    }
    let host_bufused = host_buf_len - left;
    enc_usize_byref(vmctx, bufused, host_bufused)
        .map(|_| wasm32::__WASI_ESUCCESS)
        .unwrap_or_else(|e| e)
}

pub fn wasi_fd_renumber(
    vmctx: &mut Vmctx,
    from: wasm32::__wasi_fd_t,
    to: wasm32::__wasi_fd_t,
) -> wasm32::__wasi_errno_t {
    let from = dec_fd(from);
    let to = dec_fd(to);
    let mut ctx = vmctx.get_embed_ctx_mut::<WasiCtx>();
    let fe_from = match ctx.fds.get(&from) {
        Some(fe_from) => fe_from,
        None => return wasm32::__WASI_EBADF,
    };
    let fe_to = match ctx.fds.get(&to) {
        Some(fe_to) => fe_to,
        None => return wasm32::__WASI_EBADF,
    };
    if let Err(e) = nix::unistd::dup2(fe_from.fd_object.rawfd, fe_to.fd_object.rawfd) {
        return wasm32::errno_from_nix(e.as_errno().unwrap());
    }
    let fe_from_rawfd = fe_from.fd_object.rawfd;
    ctx.fds.remove(&(fe_from_rawfd as host::__wasi_fd_t));

    wasm32::__WASI_ESUCCESS
}

pub fn wasi_path_link(
    vmctx: &mut Vmctx,
    old_dirfd: wasm32::__wasi_fd_t,
    _old_flags: wasm32::__wasi_lookupflags_t,
    old_path_ptr: wasm32::uintptr_t,
    old_path_len: wasm32::size_t,
    new_dirfd: wasm32::__wasi_fd_t,
    new_path_ptr: wasm32::uintptr_t,
    new_path_len: wasm32::size_t,
) -> wasm32::__wasi_errno_t {
    use nix::libc::linkat;

    let old_dirfd = dec_fd(old_dirfd);
    let new_dirfd = dec_fd(new_dirfd);
    let old_path = match dec_slice_of::<u8>(vmctx, old_path_ptr, old_path_len) {
        Ok(old_path_bytes) => OsStr::from_bytes(old_path_bytes),
        Err(e) => return enc_errno(e),
    };
    let new_path = match dec_slice_of::<u8>(vmctx, new_path_ptr, new_path_len) {
        Ok(new_path_bytes) => OsStr::from_bytes(new_path_bytes),
        Err(e) => return enc_errno(e),
    };
    let rights = host::__WASI_RIGHT_PATH_LINK_SOURCE;
    let (old_dir, old_path) =
        match path_get(&vmctx, old_dirfd, 0, old_path, rights.into(), 0, false) {
            Ok((dir, path)) => (dir, path),
            Err(e) => return enc_errno(e),
        };
    let rights = host::__WASI_RIGHT_PATH_LINK_TARGET;
    let (new_dir, new_path) =
        match path_get(&vmctx, new_dirfd, 0, new_path, rights.into(), 0, false) {
            Ok((dir, path)) => (dir, path),
            Err(e) => return enc_errno(e),
        };
    let old_path_cstr = match std::ffi::CString::new(old_path.as_bytes()) {
        Ok(old_path_cstr) => old_path_cstr,
        Err(_) => return wasm32::__WASI_EINVAL,
    };
    let new_path_cstr = match std::ffi::CString::new(new_path.as_bytes()) {
        Ok(new_path_cstr) => new_path_cstr,
        Err(_) => return wasm32::__WASI_EINVAL,
    };

    // Not setting AT_SYMLINK_FOLLOW fails on most filesystems
    let atflags = libc::AT_SYMLINK_FOLLOW;
    let res = unsafe {
        linkat(
            old_dir,
            old_path_cstr.as_ptr(),
            new_dir,
            new_path_cstr.as_ptr(),
            atflags,
        )
    };
    if res != 0 {
        return wasm32::errno_from_nix(nix::errno::Errno::last());
    }
    wasm32::__WASI_ESUCCESS
}

pub fn wasi_path_readlink(
    vmctx: &mut Vmctx,
    dirfd: wasm32::__wasi_fd_t,
    path_ptr: wasm32::uintptr_t,
    path_len: wasm32::size_t,
    buf_ptr: wasm32::uintptr_t,
    buf_len: wasm32::size_t,
    bufused: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    use nix::fcntl::readlinkat;
    use std::cmp;

    match enc_usize_byref(vmctx, bufused, 0) {
        Ok(_) => {}
        Err(e) => return enc_errno(e),
    };
    let dirfd = dec_fd(dirfd);
    let path = match dec_slice_of::<u8>(vmctx, path_ptr, path_len) {
        Ok(path_bytes) => OsStr::from_bytes(path_bytes),
        Err(e) => return enc_errno(e),
    };
    let rights = host::__WASI_RIGHT_PATH_READLINK;
    let (dir, path) = match path_get(&vmctx, dirfd, 0, path, rights.into(), 0, false) {
        Ok((dir, path)) => (dir, path),
        Err(e) => return enc_errno(e),
    };
    let dummy_buf = &mut [0u8];
    let mut buf = if buf_len > 0 {
        match dec_slice_of_mut::<u8>(vmctx, buf_ptr, buf_len) {
            Ok(buf) => buf,
            Err(e) => return enc_errno(e),
        }
    } else {
        dummy_buf
    };
    let target_path = match readlinkat(dir, path.as_os_str(), &mut buf) {
        Err(e) => return wasm32::errno_from_nix(e.as_errno().unwrap()),
        Ok(target_path) => target_path,
    };
    let host_bufused = cmp::min(buf_len as usize, target_path.len());
    match enc_usize_byref(vmctx, bufused, host_bufused) {
        Ok(_) => {}
        Err(e) => return enc_errno(e),
    };
    wasm32::__WASI_ESUCCESS
}

pub fn wasi_path_remove_directory(
    vmctx: &mut Vmctx,
    dirfd: wasm32::__wasi_fd_t,
    path_ptr: wasm32::uintptr_t,
    path_len: wasm32::size_t,
) -> wasm32::__wasi_errno_t {
    use nix::errno;
    use nix::libc::{unlinkat, AT_REMOVEDIR};

    let dirfd = dec_fd(dirfd);
    let path = match dec_slice_of::<u8>(vmctx, path_ptr, path_len) {
        Ok(path_bytes) => OsStr::from_bytes(path_bytes),
        Err(e) => return enc_errno(e),
    };
    let rights = host::__WASI_RIGHT_PATH_REMOVE_DIRECTORY;
    let (dir, path) = match path_get(&vmctx, dirfd, 0, path, rights.into(), 0, false) {
        Ok((dir, path)) => (dir, path),
        Err(e) => return enc_errno(e),
    };
    let path_cstr = match std::ffi::CString::new(path.as_os_str().as_bytes()) {
        Ok(path_cstr) => path_cstr,
        Err(_) => return wasm32::__WASI_EINVAL,
    };
    // nix doesn't expose unlinkat() yet
    match unsafe { unlinkat(dir, path_cstr.as_ptr(), AT_REMOVEDIR) } {
        0 => wasm32::__WASI_ESUCCESS,
        _ => wasm32::errno_from_nix(errno::Errno::last()),
    }
}

pub fn wasi_path_rename(
    vmctx: &mut Vmctx,
    old_dirfd: wasm32::__wasi_fd_t,
    old_path_ptr: wasm32::uintptr_t,
    old_path_len: wasm32::size_t,
    new_dirfd: wasm32::__wasi_fd_t,
    new_path_ptr: wasm32::uintptr_t,
    new_path_len: wasm32::size_t,
) -> wasm32::__wasi_errno_t {
    use nix::libc::renameat;

    let old_dirfd = dec_fd(old_dirfd);
    let new_dirfd = dec_fd(new_dirfd);
    let old_path = match dec_slice_of::<u8>(vmctx, old_path_ptr, old_path_len) {
        Ok(old_path_bytes) => OsStr::from_bytes(old_path_bytes),
        Err(e) => return enc_errno(e),
    };
    let new_path = match dec_slice_of::<u8>(vmctx, new_path_ptr, new_path_len) {
        Ok(new_path_bytes) => OsStr::from_bytes(new_path_bytes),
        Err(e) => return enc_errno(e),
    };
    let rights = host::__WASI_RIGHT_PATH_RENAME_SOURCE;
    let (old_dir, old_path) =
        match path_get(&vmctx, old_dirfd, 0, old_path, rights.into(), 0, false) {
            Ok((dir, path)) => (dir, path),
            Err(e) => return enc_errno(e),
        };
    let rights = host::__WASI_RIGHT_PATH_RENAME_TARGET;
    let (new_dir, new_path) =
        match path_get(&vmctx, new_dirfd, 0, new_path, rights.into(), 0, false) {
            Ok((dir, path)) => (dir, path),
            Err(e) => return enc_errno(e),
        };
    let old_path_cstr = match std::ffi::CString::new(old_path.as_bytes()) {
        Ok(old_path_cstr) => old_path_cstr,
        Err(_) => return wasm32::__WASI_EINVAL,
    };
    let new_path_cstr = match std::ffi::CString::new(new_path.as_bytes()) {
        Ok(new_path_cstr) => new_path_cstr,
        Err(_) => return wasm32::__WASI_EINVAL,
    };
    let res = unsafe {
        renameat(
            old_dir,
            old_path_cstr.as_ptr(),
            new_dir,
            new_path_cstr.as_ptr(),
        )
    };
    if res != 0 {
        return wasm32::errno_from_nix(nix::errno::Errno::last());
    }
    wasm32::__WASI_ESUCCESS
}

pub fn wasi_path_symlink(
    vmctx: &mut Vmctx,
    old_path_ptr: wasm32::uintptr_t,
    old_path_len: wasm32::size_t,
    dirfd: wasm32::__wasi_fd_t,
    new_path_ptr: wasm32::uintptr_t,
    new_path_len: wasm32::size_t,
) -> wasm32::__wasi_errno_t {
    use nix::libc::symlinkat;

    let dirfd = dec_fd(dirfd);
    let old_path = match dec_slice_of::<u8>(vmctx, old_path_ptr, old_path_len) {
        Ok(old_path_bytes) => OsStr::from_bytes(old_path_bytes),
        Err(e) => return enc_errno(e),
    };
    let new_path = match dec_slice_of::<u8>(vmctx, new_path_ptr, new_path_len) {
        Ok(new_path_bytes) => OsStr::from_bytes(new_path_bytes),
        Err(e) => return enc_errno(e),
    };
    let rights = host::__WASI_RIGHT_PATH_SYMLINK;
    let (dir, new_path) = match path_get(&vmctx, dirfd, 0, new_path, rights.into(), 0, false) {
        Ok((dir, path)) => (dir, path),
        Err(e) => return enc_errno(e),
    };
    let old_path_cstr = match std::ffi::CString::new(old_path.as_bytes()) {
        Ok(old_path_cstr) => old_path_cstr,
        Err(_) => return wasm32::__WASI_EINVAL,
    };
    let new_path_cstr = match std::ffi::CString::new(new_path.as_bytes()) {
        Ok(new_path_cstr) => new_path_cstr,
        Err(_) => return wasm32::__WASI_EINVAL,
    };
    let res = unsafe { symlinkat(old_path_cstr.as_ptr(), dir, new_path_cstr.as_ptr()) };
    if res != 0 {
        return wasm32::errno_from_nix(nix::errno::Errno::last());
    }
    wasm32::__WASI_ESUCCESS
}
