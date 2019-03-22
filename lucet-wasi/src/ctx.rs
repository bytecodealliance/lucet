use crate::fdentry::FdEntry;
use crate::host;
use nix::unistd::dup;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::io::{stderr, stdin, stdout};
use std::os::unix::prelude::{AsRawFd, FromRawFd, IntoRawFd, RawFd};

pub struct WasiCtxBuilder {
    fds: HashMap<host::__wasi_fd_t, FdEntry>,
    args: Vec<CString>,
    env: HashMap<CString, CString>,
}

impl WasiCtxBuilder {
    /// Builder for a new `WasiCtx`.
    pub fn new() -> Self {
        WasiCtxBuilder {
            fds: HashMap::new(),
            args: vec![],
            env: HashMap::new(),
        }
    }

    pub fn args(mut self, args: &[&str]) -> Self {
        self.args = args
            .into_iter()
            .map(|arg| CString::new(*arg).expect("argument can be converted to a CString"))
            .collect();
        self
    }

    pub fn arg(mut self, arg: &str) -> Self {
        self.args
            .push(CString::new(arg).expect("argument can be converted to a CString"));
        self
    }

    pub fn c_args<S: AsRef<CStr>>(mut self, args: &[S]) -> Self {
        self.args = args
            .into_iter()
            .map(|arg| arg.as_ref().to_owned())
            .collect();
        self
    }

    pub fn c_arg<S: AsRef<CStr>>(mut self, arg: S) -> Self {
        self.args.push(arg.as_ref().to_owned());
        self
    }

    pub fn inherit_env(mut self) -> Self {
        self.env = std::env::vars()
            .map(|(k, v)| {
                // TODO: handle errors, and possibly assert that the key is valid per POSIX
                (
                    CString::new(k).expect("environment key can be converted to a CString"),
                    CString::new(v).expect("environment value can be converted to a CString"),
                )
            })
            .collect();
        self
    }

    pub fn inherit_stdio(self) -> Self {
        self.fd_dup(0, stdin())
            .fd_dup(1, stdout())
            .fd_dup(2, stderr())
    }

    pub fn env(mut self, k: &str, v: &str) -> Self {
        self.env.insert(
            // TODO: handle errors, and possibly assert that the key is valid per POSIX
            CString::new(k).expect("environment key can be converted to a CString"),
            CString::new(v).expect("environment value can be converted to a CString"),
        );
        self
    }

    pub fn c_env<S, T>(mut self, k: S, v: T) -> Self
    where
        S: AsRef<CStr>,
        T: AsRef<CStr>,
    {
        self.env
            .insert(k.as_ref().to_owned(), v.as_ref().to_owned());
        self
    }

    /// Add an existing file-like object as a file descriptor in the context.
    ///
    /// When the `WasiCtx` is dropped, all of its associated file descriptors are `close`d. If you
    /// do not want this to close the existing object, use `WasiCtxBuilder::fd_dup()`.
    pub fn fd<F: IntoRawFd>(self, wasm_fd: host::__wasi_fd_t, fd: F) -> Self {
        // safe because we're getting a valid RawFd from the F directly
        unsafe { self.raw_fd(wasm_fd, fd.into_raw_fd()) }
    }

    /// Add an existing file-like object as a duplicate file descriptor in the context.
    ///
    /// The underlying file descriptor of this object will be duplicated before being added to the
    /// context, so it will not be closed when the `WasiCtx` is dropped.
    ///
    /// TODO: handle `dup` errors
    pub fn fd_dup<F: AsRawFd>(self, wasm_fd: host::__wasi_fd_t, fd: F) -> Self {
        // safe because we're getting a valid RawFd from the F directly
        unsafe { self.raw_fd(wasm_fd, dup(fd.as_raw_fd()).unwrap()) }
    }

    /// Add an existing file descriptor to the context.
    ///
    /// When the `WasiCtx` is dropped, this file descriptor will be `close`d. If you do not want to
    /// close the existing descriptor, use `WasiCtxBuilder::raw_fd_dup()`.
    pub unsafe fn raw_fd(mut self, wasm_fd: host::__wasi_fd_t, fd: RawFd) -> Self {
        self.fds.insert(wasm_fd, FdEntry::from_raw_fd(fd));
        self
    }

    /// Add a duplicate of an existing file descriptor to the context.
    ///
    /// The file descriptor will be duplicated before being added to the context, so it will not be
    /// closed when the `WasiCtx` is dropped.
    ///
    /// TODO: handle `dup` errors
    pub unsafe fn raw_fd_dup(self, wasm_fd: host::__wasi_fd_t, fd: RawFd) -> Self {
        self.raw_fd(wasm_fd, dup(fd).unwrap())
    }

    pub fn build(self) -> WasiCtx {
        let env = self
            .env
            .into_iter()
            .map(|(k, v)| {
                let mut pair = k.into_bytes();
                pair.extend_from_slice(b"=");
                pair.extend_from_slice(v.to_bytes_with_nul());
                // constructing a new CString from existing CStrings is safe
                unsafe { CString::from_vec_unchecked(pair) }
            })
            .collect();

        WasiCtx {
            fds: self.fds,
            args: self.args,
            env,
        }
    }
}

#[derive(Debug)]
pub struct WasiCtx {
    pub fds: HashMap<host::__wasi_fd_t, FdEntry>,
    pub args: Vec<CString>,
    pub env: Vec<CString>,
}

impl WasiCtx {
    /// Make a new `WasiCtx` with some default settings.
    ///
    /// - File descriptors 0, 1, and 2 inherit stdin, stdout, and stderr from the host process.
    ///
    /// - Environment variables are inherited from the host process.
    ///
    /// To override these behaviors, use `WasiCtxBuilder`.
    pub fn new(args: &[&str]) -> WasiCtx {
        WasiCtxBuilder::new()
            .args(args)
            .inherit_env()
            .inherit_stdio()
            .build()
    }

    pub fn get_fd_entry(
        &self,
        fd: host::__wasi_fd_t,
        rights_base: host::__wasi_rights_t,
        rights_inheriting: host::__wasi_rights_t,
    ) -> Result<&FdEntry, host::__wasi_errno_t> {
        if let Some(fe) = self.fds.get(&fd) {
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
