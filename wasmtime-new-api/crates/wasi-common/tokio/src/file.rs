use crate::block_on_dummy_executor;
use std::any::Any;
use std::io;
#[cfg(windows)]
use std::os::windows::io::{AsRawHandle, RawHandle};
use wasi_common::{
    file::{Advice, FdFlags, FileType, Filestat, WasiFile},
    Error,
};

pub struct File(wasi_cap_std_sync::file::File);

impl File {
    pub(crate) fn from_inner(file: wasi_cap_std_sync::file::File) -> Self {
        File(file)
    }
    pub fn from_cap_std(file: cap_std::fs::File) -> Self {
        Self::from_inner(wasi_cap_std_sync::file::File::from_cap_std(file))
    }
}

pub struct Stdin(wasi_cap_std_sync::stdio::Stdin);

pub fn stdin() -> Stdin {
    Stdin(wasi_cap_std_sync::stdio::stdin())
}

pub struct Stdout(wasi_cap_std_sync::stdio::Stdout);

pub fn stdout() -> Stdout {
    Stdout(wasi_cap_std_sync::stdio::stdout())
}

pub struct Stderr(wasi_cap_std_sync::stdio::Stderr);

pub fn stderr() -> Stderr {
    Stderr(wasi_cap_std_sync::stdio::stderr())
}

macro_rules! wasi_file_impl {
    ($ty:ty) => {
        #[wiggle::async_trait]
        impl WasiFile for $ty {
            fn as_any(&self) -> &dyn Any {
                self
            }
            async fn datasync(&self) -> Result<(), Error> {
                block_on_dummy_executor(|| self.0.datasync())
            }
            async fn sync(&self) -> Result<(), Error> {
                block_on_dummy_executor(|| self.0.sync())
            }
            async fn get_filetype(&self) -> Result<FileType, Error> {
                block_on_dummy_executor(|| self.0.get_filetype())
            }
            async fn get_fdflags(&self) -> Result<FdFlags, Error> {
                block_on_dummy_executor(|| self.0.get_fdflags())
            }
            async fn set_fdflags(&mut self, fdflags: FdFlags) -> Result<(), Error> {
                block_on_dummy_executor(|| self.0.set_fdflags(fdflags))
            }
            async fn get_filestat(&self) -> Result<Filestat, Error> {
                block_on_dummy_executor(|| self.0.get_filestat())
            }
            async fn set_filestat_size(&self, size: u64) -> Result<(), Error> {
                block_on_dummy_executor(move || self.0.set_filestat_size(size))
            }
            async fn advise(&self, offset: u64, len: u64, advice: Advice) -> Result<(), Error> {
                block_on_dummy_executor(move || self.0.advise(offset, len, advice))
            }
            async fn allocate(&self, offset: u64, len: u64) -> Result<(), Error> {
                block_on_dummy_executor(move || self.0.allocate(offset, len))
            }
            async fn read_vectored<'a>(
                &self,
                bufs: &mut [io::IoSliceMut<'a>],
            ) -> Result<u64, Error> {
                block_on_dummy_executor(move || self.0.read_vectored(bufs))
            }
            async fn read_vectored_at<'a>(
                &self,
                bufs: &mut [io::IoSliceMut<'a>],
                offset: u64,
            ) -> Result<u64, Error> {
                block_on_dummy_executor(move || self.0.read_vectored_at(bufs, offset))
            }
            async fn write_vectored<'a>(&self, bufs: &[io::IoSlice<'a>]) -> Result<u64, Error> {
                block_on_dummy_executor(move || self.0.write_vectored(bufs))
            }
            async fn write_vectored_at<'a>(
                &self,
                bufs: &[io::IoSlice<'a>],
                offset: u64,
            ) -> Result<u64, Error> {
                block_on_dummy_executor(move || self.0.write_vectored_at(bufs, offset))
            }
            async fn seek(&self, pos: std::io::SeekFrom) -> Result<u64, Error> {
                block_on_dummy_executor(move || self.0.seek(pos))
            }
            async fn peek(&self, buf: &mut [u8]) -> Result<u64, Error> {
                block_on_dummy_executor(move || self.0.peek(buf))
            }
            async fn set_times(
                &self,
                atime: Option<wasi_common::SystemTimeSpec>,
                mtime: Option<wasi_common::SystemTimeSpec>,
            ) -> Result<(), Error> {
                block_on_dummy_executor(move || self.0.set_times(atime, mtime))
            }
            async fn num_ready_bytes(&self) -> Result<u64, Error> {
                block_on_dummy_executor(|| self.0.num_ready_bytes())
            }

            #[cfg(not(windows))]
            async fn readable(&self) -> Result<(), Error> {
                // The Inner impls OwnsRaw, which asserts exclusive use of the handle by the owned object.
                // AsyncFd needs to wrap an owned `impl std::os::unix::io::AsRawFd`. Rather than introduce
                // mutability to let it own the `Inner`, we are depending on the `&mut self` bound on this
                // async method to ensure this is the only Future which can access the RawFd during the
                // lifetime of the AsyncFd.
                use tokio::io::{unix::AsyncFd, Interest};
                use unsafe_io::os::posish::AsRawFd;
                let rawfd = self.0.as_raw_fd();
                match AsyncFd::with_interest(rawfd, Interest::READABLE) {
                    Ok(asyncfd) => {
                        let _ = asyncfd.readable().await?;
                        Ok(())
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                        // if e is EPERM, this file isnt supported by epoll because it is immediately
                        // available for reading:
                        Ok(())
                    }
                    Err(e) => Err(e.into()),
                }
            }
            #[cfg(windows)]
            async fn readable(&self) -> Result<(), Error> {
                // Windows uses a rawfd based scheduler :(
                use wasi_common::ErrorExt;
                Err(Error::badf())
            }

            #[cfg(not(windows))]
            async fn writable(&self) -> Result<(), Error> {
                // The Inner impls OwnsRaw, which asserts exclusive use of the handle by the owned object.
                // AsyncFd needs to wrap an owned `impl std::os::unix::io::AsRawFd`. Rather than introduce
                // mutability to let it own the `Inner`, we are depending on the `&mut self` bound on this
                // async method to ensure this is the only Future which can access the RawFd during the
                // lifetime of the AsyncFd.
                use tokio::io::{unix::AsyncFd, Interest};
                use unsafe_io::os::posish::AsRawFd;
                let rawfd = self.0.as_raw_fd();
                match AsyncFd::with_interest(rawfd, Interest::WRITABLE) {
                    Ok(asyncfd) => {
                        let _ = asyncfd.writable().await?;
                        Ok(())
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                        // if e is EPERM, this file isnt supported by epoll because it is immediately
                        // available for writing:
                        Ok(())
                    }
                    Err(e) => Err(e.into()),
                }
            }
            #[cfg(windows)]
            async fn writable(&self) -> Result<(), Error> {
                // Windows uses a rawfd based scheduler :(
                use wasi_common::ErrorExt;
                Err(Error::badf())
            }
        }
        #[cfg(windows)]
        impl AsRawHandle for $ty {
            fn as_raw_handle(&self) -> RawHandle {
                self.0.as_raw_handle()
            }
        }
    };
}

wasi_file_impl!(File);
wasi_file_impl!(Stdin);
wasi_file_impl!(Stdout);
wasi_file_impl!(Stderr);
