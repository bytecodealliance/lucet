use lucet_runtime::vmctx::Vmctx;
use lucet_wiggle::{GuestError, GuestPtr};
use std::cell::Ref;
use std::convert::TryInto;
use tracing::debug;
use wasi_common::snapshots::preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1;
use wasi_common::{Error, WasiCtx};

lucet_wasi_generate::bindings!({
    // The context type, which we will implement the GuestErrorConversion and
    // WasiSnapshotPreview1 traits.
    ctx: LucetWasiCtx,
    // Describe how to construct the context type. The expression inside the first set
    // of braces will be used each time LucetWasiCtx needs to be constructed.
    // `vmctx: &Vmctx` is a free variable at the construction site.
    constructor: { crate::LucetWasiCtx { vmctx } },
    // Need to use the exact same error mapping as wasi-common does:
    errors: { errno => Error },
});

pub mod types {
    pub use wasi_common::snapshots::preview_1::types::*;
}

pub fn export_wasi_funcs() {
    hostcalls::init()
}

pub struct LucetWasiCtx<'a> {
    vmctx: &'a Vmctx,
}

impl<'a> LucetWasiCtx<'a> {
    pub fn wasi(&self) -> Ref<WasiCtx> {
        self.vmctx.get_embed_ctx()
    }
}

impl<'a> types::GuestErrorConversion for LucetWasiCtx<'a> {
    fn into_errno(&self, e: GuestError) -> types::Errno {
        debug!("Guest error: {:?}", e);
        e.into()
    }
}

impl<'a> types::UserErrorConversion for LucetWasiCtx<'a> {
    fn errno_from_error(&self, e: Error) -> Result<types::Errno, lucet_wiggle::Trap> {
        debug!("Error: {:?}", e);
        e.try_into()
            .map_err(|e| lucet_wiggle::Trap::String(format!("{:?}", e)))
    }
}

impl<'a> wasi_snapshot_preview1::WasiSnapshotPreview1 for LucetWasiCtx<'a> {
    fn args_get<'b>(
        &self,
        argv: &GuestPtr<'b, GuestPtr<'b, u8>>,
        argv_buf: &GuestPtr<'b, u8>,
    ) -> Result<(), Error> {
        self.wasi().args_get(argv, argv_buf)
    }

    fn args_sizes_get(&self) -> Result<(types::Size, types::Size), Error> {
        self.wasi().args_sizes_get()
    }

    fn environ_get<'b>(
        &self,
        environ: &GuestPtr<'b, GuestPtr<'b, u8>>,
        environ_buf: &GuestPtr<'b, u8>,
    ) -> Result<(), Error> {
        self.wasi().environ_get(environ, environ_buf)
    }

    fn environ_sizes_get(&self) -> Result<(types::Size, types::Size), Error> {
        self.wasi().environ_sizes_get()
    }

    fn clock_res_get(&self, id: types::Clockid) -> Result<types::Timestamp, Error> {
        self.wasi().clock_res_get(id)
    }

    fn clock_time_get(
        &self,
        id: types::Clockid,
        precision: types::Timestamp,
    ) -> Result<types::Timestamp, Error> {
        self.wasi().clock_time_get(id, precision)
    }

    fn fd_advise(
        &self,
        fd: types::Fd,
        offset: types::Filesize,
        len: types::Filesize,
        advice: types::Advice,
    ) -> Result<(), Error> {
        self.wasi().fd_advise(fd, offset, len, advice)
    }

    fn fd_allocate(
        &self,
        fd: types::Fd,
        offset: types::Filesize,
        len: types::Filesize,
    ) -> Result<(), Error> {
        self.wasi().fd_allocate(fd, offset, len)
    }

    fn fd_close(&self, fd: types::Fd) -> Result<(), Error> {
        self.wasi().fd_close(fd)
    }

    fn fd_datasync(&self, fd: types::Fd) -> Result<(), Error> {
        self.wasi().fd_datasync(fd)
    }

    fn fd_fdstat_get(&self, fd: types::Fd) -> Result<types::Fdstat, Error> {
        self.wasi().fd_fdstat_get(fd)
    }

    fn fd_fdstat_set_flags(&self, fd: types::Fd, flags: types::Fdflags) -> Result<(), Error> {
        self.wasi().fd_fdstat_set_flags(fd, flags)
    }

    fn fd_fdstat_set_rights(
        &self,
        fd: types::Fd,
        fs_rights_base: types::Rights,
        fs_rights_inheriting: types::Rights,
    ) -> Result<(), Error> {
        self.wasi()
            .fd_fdstat_set_rights(fd, fs_rights_base, fs_rights_inheriting)
    }

    fn fd_filestat_get(&self, fd: types::Fd) -> Result<types::Filestat, Error> {
        self.wasi().fd_filestat_get(fd)
    }

    fn fd_filestat_set_size(&self, fd: types::Fd, size: types::Filesize) -> Result<(), Error> {
        self.wasi().fd_filestat_set_size(fd, size)
    }

    fn fd_filestat_set_times(
        &self,
        fd: types::Fd,
        atim: types::Timestamp,
        mtim: types::Timestamp,
        fst_flags: types::Fstflags,
    ) -> Result<(), Error> {
        self.wasi().fd_filestat_set_times(fd, atim, mtim, fst_flags)
    }

    fn fd_pread(
        &self,
        fd: types::Fd,
        iovs: &types::IovecArray<'_>,
        offset: types::Filesize,
    ) -> Result<types::Size, Error> {
        self.wasi().fd_pread(fd, iovs, offset)
    }

    fn fd_prestat_get(&self, fd: types::Fd) -> Result<types::Prestat, Error> {
        self.wasi().fd_prestat_get(fd)
    }

    fn fd_prestat_dir_name(
        &self,
        fd: types::Fd,
        path: &GuestPtr<u8>,
        path_len: types::Size,
    ) -> Result<(), Error> {
        self.wasi().fd_prestat_dir_name(fd, path, path_len)
    }

    fn fd_pwrite(
        &self,
        fd: types::Fd,
        ciovs: &types::CiovecArray<'_>,
        offset: types::Filesize,
    ) -> Result<types::Size, Error> {
        self.wasi().fd_pwrite(fd, ciovs, offset)
    }

    fn fd_read(&self, fd: types::Fd, iovs: &types::IovecArray<'_>) -> Result<types::Size, Error> {
        self.wasi().fd_read(fd, iovs)
    }

    fn fd_readdir(
        &self,
        fd: types::Fd,
        buf: &GuestPtr<u8>,
        buf_len: types::Size,
        cookie: types::Dircookie,
    ) -> Result<types::Size, Error> {
        self.wasi().fd_readdir(fd, buf, buf_len, cookie)
    }

    fn fd_renumber(&self, from: types::Fd, to: types::Fd) -> Result<(), Error> {
        self.wasi().fd_renumber(from, to)
    }

    fn fd_seek(
        &self,
        fd: types::Fd,
        offset: types::Filedelta,
        whence: types::Whence,
    ) -> Result<types::Filesize, Error> {
        self.wasi().fd_seek(fd, offset, whence)
    }

    fn fd_sync(&self, fd: types::Fd) -> Result<(), Error> {
        self.wasi().fd_sync(fd)
    }

    fn fd_tell(&self, fd: types::Fd) -> Result<types::Filesize, Error> {
        self.wasi().fd_tell(fd)
    }

    fn fd_write(
        &self,
        fd: types::Fd,
        ciovs: &types::CiovecArray<'_>,
    ) -> Result<types::Size, Error> {
        self.wasi().fd_write(fd, ciovs)
    }

    fn path_create_directory(
        &self,
        dirfd: types::Fd,
        path: &GuestPtr<'_, str>,
    ) -> Result<(), Error> {
        self.wasi().path_create_directory(dirfd, path)
    }

    fn path_filestat_get(
        &self,
        dirfd: types::Fd,
        flags: types::Lookupflags,
        path: &GuestPtr<'_, str>,
    ) -> Result<types::Filestat, Error> {
        self.wasi().path_filestat_get(dirfd, flags, path)
    }

    fn path_filestat_set_times(
        &self,
        dirfd: types::Fd,
        flags: types::Lookupflags,
        path: &GuestPtr<'_, str>,
        atim: types::Timestamp,
        mtim: types::Timestamp,
        fst_flags: types::Fstflags,
    ) -> Result<(), Error> {
        self.wasi()
            .path_filestat_set_times(dirfd, flags, path, atim, mtim, fst_flags)
    }

    fn path_link(
        &self,
        old_fd: types::Fd,
        old_flags: types::Lookupflags,
        old_path: &GuestPtr<'_, str>,
        new_fd: types::Fd,
        new_path: &GuestPtr<'_, str>,
    ) -> Result<(), Error> {
        self.wasi()
            .path_link(old_fd, old_flags, old_path, new_fd, new_path)
    }

    fn path_open(
        &self,
        dirfd: types::Fd,
        dirflags: types::Lookupflags,
        path: &GuestPtr<'_, str>,
        oflags: types::Oflags,
        fs_rights_base: types::Rights,
        fs_rights_inheriting: types::Rights,
        fdflags: types::Fdflags,
    ) -> Result<types::Fd, Error> {
        self.wasi().path_open(
            dirfd,
            dirflags,
            path,
            oflags,
            fs_rights_base,
            fs_rights_inheriting,
            fdflags,
        )
    }

    fn path_readlink(
        &self,
        dirfd: types::Fd,
        path: &GuestPtr<'_, str>,
        buf: &GuestPtr<u8>,
        buf_len: types::Size,
    ) -> Result<types::Size, Error> {
        self.wasi().path_readlink(dirfd, path, buf, buf_len)
    }

    fn path_remove_directory(
        &self,
        dirfd: types::Fd,
        path: &GuestPtr<'_, str>,
    ) -> Result<(), Error> {
        self.wasi().path_remove_directory(dirfd, path)
    }

    fn path_rename(
        &self,
        old_fd: types::Fd,
        old_path: &GuestPtr<'_, str>,
        new_fd: types::Fd,
        new_path: &GuestPtr<'_, str>,
    ) -> Result<(), Error> {
        self.wasi().path_rename(old_fd, old_path, new_fd, new_path)
    }

    fn path_symlink(
        &self,
        old_path: &GuestPtr<'_, str>,
        dirfd: types::Fd,
        new_path: &GuestPtr<'_, str>,
    ) -> Result<(), Error> {
        self.wasi().path_symlink(old_path, dirfd, new_path)
    }

    fn path_unlink_file(&self, dirfd: types::Fd, path: &GuestPtr<'_, str>) -> Result<(), Error> {
        self.wasi().path_unlink_file(dirfd, path)
    }

    fn poll_oneoff(
        &self,
        in_: &GuestPtr<types::Subscription>,
        out: &GuestPtr<types::Event>,
        nsubscriptions: types::Size,
    ) -> Result<types::Size, Error> {
        self.wasi().poll_oneoff(in_, out, nsubscriptions)
    }

    fn proc_exit(&self, rval: types::Exitcode) -> lucet_wiggle::Trap {
        self.wasi().proc_exit(rval)
    }

    fn proc_raise(&self, sig: types::Signal) -> Result<(), Error> {
        self.wasi().proc_raise(sig)
    }

    fn sched_yield(&self) -> Result<(), Error> {
        self.wasi().sched_yield()
    }

    fn random_get(&self, buf: &GuestPtr<u8>, buf_len: types::Size) -> Result<(), Error> {
        self.wasi().random_get(buf, buf_len)
    }

    fn sock_recv(
        &self,
        fd: types::Fd,
        ri_data: &types::IovecArray<'_>,
        ri_flags: types::Riflags,
    ) -> Result<(types::Size, types::Roflags), Error> {
        self.wasi().sock_recv(fd, ri_data, ri_flags)
    }

    fn sock_send(
        &self,
        fd: types::Fd,
        si_data: &types::CiovecArray<'_>,
        si_flags: types::Siflags,
    ) -> Result<types::Size, Error> {
        self.wasi().sock_send(fd, si_data, si_flags)
    }

    fn sock_shutdown(&self, fd: types::Fd, how: types::Sdflags) -> Result<(), Error> {
        self.wasi().sock_shutdown(fd, how)
    }
}
