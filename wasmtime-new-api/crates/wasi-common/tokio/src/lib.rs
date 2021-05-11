mod dir;
mod file;
pub mod sched;
pub mod stdio;

use std::future::Future;
use std::path::Path;
pub use wasi_cap_std_sync::{clocks_ctx, random_ctx};
use wasi_common::{Error, Table, WasiCtx};

pub use dir::Dir;
pub use file::File;

use crate::sched::sched_ctx;

pub struct WasiCtxBuilder(wasi_common::WasiCtxBuilder);

impl WasiCtxBuilder {
    pub fn new() -> Self {
        WasiCtxBuilder(WasiCtx::builder(
            random_ctx(),
            clocks_ctx(),
            sched_ctx(),
            Table::new(),
        ))
    }
    pub fn env(self, var: &str, value: &str) -> Result<Self, wasi_common::StringArrayError> {
        let s = self.0.env(var, value)?;
        Ok(WasiCtxBuilder(s))
    }
    pub fn envs(self, env: &[(String, String)]) -> Result<Self, wasi_common::StringArrayError> {
        let mut s = self;
        for (k, v) in env {
            s = s.env(k, v)?;
        }
        Ok(s)
    }
    pub fn inherit_env(self) -> Result<Self, wasi_common::StringArrayError> {
        let mut s = self.0;
        for (key, value) in std::env::vars() {
            s = s.env(&key, &value)?;
        }
        Ok(WasiCtxBuilder(s))
    }
    pub fn arg(self, arg: &str) -> Result<Self, wasi_common::StringArrayError> {
        let s = self.0.arg(arg)?;
        Ok(WasiCtxBuilder(s))
    }
    pub fn args(self, arg: &[String]) -> Result<Self, wasi_common::StringArrayError> {
        let mut s = self;
        for a in arg {
            s = s.arg(&a)?;
        }
        Ok(s)
    }
    pub fn inherit_args(self) -> Result<Self, wasi_common::StringArrayError> {
        let mut s = self.0;
        for arg in std::env::args() {
            s = s.arg(&arg)?;
        }
        Ok(WasiCtxBuilder(s))
    }
    pub fn stdin(self, f: Box<dyn wasi_common::WasiFile>) -> Self {
        WasiCtxBuilder(self.0.stdin(f))
    }
    pub fn stdout(self, f: Box<dyn wasi_common::WasiFile>) -> Self {
        WasiCtxBuilder(self.0.stdout(f))
    }
    pub fn stderr(self, f: Box<dyn wasi_common::WasiFile>) -> Self {
        WasiCtxBuilder(self.0.stderr(f))
    }
    pub fn inherit_stdin(self) -> Self {
        self.stdin(Box::new(crate::stdio::stdin()))
    }
    pub fn inherit_stdout(self) -> Self {
        self.stdout(Box::new(crate::stdio::stdout()))
    }
    pub fn inherit_stderr(self) -> Self {
        self.stderr(Box::new(crate::stdio::stderr()))
    }
    pub fn inherit_stdio(self) -> Self {
        self.inherit_stdin().inherit_stdout().inherit_stderr()
    }
    pub fn preopened_dir(
        self,
        dir: cap_std::fs::Dir,
        guest_path: impl AsRef<Path>,
    ) -> Result<Self, wasi_common::Error> {
        let dir = Box::new(Dir::from_cap_std(dir));
        Ok(WasiCtxBuilder(self.0.preopened_dir(dir, guest_path)?))
    }
    pub fn build(self) -> Result<WasiCtx, wasi_common::Error> {
        self.0.build()
    }
}

// Much of this crate is implemented in terms of `async` methods from the
// wasi-cap-std-sync crate. These methods may be async in signature, however,
// they are synchronous in implementation (always Poll::Ready on first poll)
// and perform blocking syscalls.
//
// This function takes this blocking code and executes it using a dummy executor
// to assert its immediate readiness. We tell tokio this is a blocking operation
// with the block_in_place function.
pub(crate) fn block_on_dummy_executor<'a, F, Fut, T>(f: F) -> Result<T, Error>
where
    F: FnOnce() -> Fut + Send + 'a,
    Fut: Future<Output = Result<T, Error>>,
    T: Send + 'static,
{
    tokio::task::block_in_place(move || wiggle::run_in_dummy_executor(f()))
}
