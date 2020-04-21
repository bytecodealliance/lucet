use lucet_runtime::vmctx::Vmctx;
use lucet_runtime::{DlModule, Limits, MmapRegion, Region};
use lucet_wasi_sdk::{CompileOpts, Link, LinkOpt, LinkOpts};
use lucetc::{Lucetc, LucetcOpts};
use std::cell::{RefCell, RefMut};
use tempfile::TempDir;
use wiggle::{GuestError, GuestErrorType, GuestPtr};

/// Context struct used to implement the wiggle trait:
pub struct LucetWasiCtx<'a> {
    guest_errors: RefCell<Vec<GuestError>>,
    vmctx: &'a Vmctx,
}

impl<'a> LucetWasiCtx<'a> {
    /// Constructor from vmctx. Given to lucet_wiggle_generate by the `constructor` field in proc
    /// macro.
    pub fn build(vmctx: &'a Vmctx) -> Self {
        LucetWasiCtx {
            guest_errors: RefCell::new(Vec::new()),
            vmctx,
        }
    }

    /// Getter for embed ctx, used in trait implementation
    pub fn get_test_ctx(&self) -> RefMut<TestCtx> {
        self.vmctx.get_embed_ctx_mut()
    }
}

/// Embedding ctx object for this particular test code.
/// We just keep two results to give in `args_sizes_get`
/// and a tally of how many times that method got called.
#[derive(Debug, Clone)]
pub struct TestCtx {
    a: types::Size,
    b: types::Size,
    times_called: usize,
}

// Invoke the lucet_wiggle proc macro!  Generate code from the snapshot 1 witx
// file in the Wasi repo.  The Wasi snapshot 1 spec was selected for maximum
// coverage of the code generator (uses each kind of type definable by witx).
// Types described in the witx spec will end up in `pub mod types`. Functions
// and the trait definition for the snapshot will end up in `pub mod
// wasi_snapshot_preview1`.
//
// `ctx`: Dispatch method calls to the LucetWasiCtx struct defined here.
// `constructor`: Show how to construct a ctx struct.
// `vmctx` is in scope at use sites.
lucet_wiggle::from_witx!({
    witx: ["../wasi/phases/snapshot/witx/wasi_snapshot_preview1.witx"],
    ctx: LucetWasiCtx,
    constructor: { LucetWasiCtx::build(vmctx) },
});

/// Convenience type for writing the trait result types.
type Result<T> = std::result::Result<T, types::Errno>;

/// Required implementation: show wiggle how to convert
/// its GuestError into the Errno returned by these calls.
impl GuestErrorType for types::Errno {
    fn success() -> types::Errno {
        types::Errno::Success
    }
}

impl<'a> types::GuestErrorConversion for LucetWasiCtx<'a> {
    fn into_errno(&self, e: GuestError) -> types::Errno {
        eprintln!("GUEST ERROR: {:?}", e);
        self.guest_errors.borrow_mut().push(e);
        types::Errno::Io
    }
}

/// Implementation of thw wasi_snapshot_preview1 trait.  The generated code
/// defines this trait, and expects it to be implemented by `LucetWasiCtx`.
///
/// Since this trait is huge, we don't actually implement very much of it.
/// This test harness only ends up calling the `args_sizes_get` method.
impl<'a> crate::wasi_snapshot_preview1::WasiSnapshotPreview1 for LucetWasiCtx<'a> {
    fn args_get(&self, _argv: &GuestPtr<GuestPtr<u8>>, _argv_buf: &GuestPtr<u8>) -> Result<()> {
        unimplemented!("args_get")
    }

    fn args_sizes_get(&self) -> Result<(types::Size, types::Size)> {
        let mut test_ctx = self.get_test_ctx();
        test_ctx.times_called += 1;
        Ok((test_ctx.a, test_ctx.b))
    }

    fn environ_get(
        &self,
        _environ: &GuestPtr<GuestPtr<u8>>,
        _environ_buf: &GuestPtr<u8>,
    ) -> Result<()> {
        unimplemented!("environ_get")
    }

    fn environ_sizes_get(&self) -> Result<(types::Size, types::Size)> {
        unimplemented!("environ_sizes_get")
    }

    fn clock_res_get(&self, _id: types::Clockid) -> Result<types::Timestamp> {
        unimplemented!("clock_res_get")
    }

    fn clock_time_get(
        &self,
        _id: types::Clockid,
        _precision: types::Timestamp,
    ) -> Result<types::Timestamp> {
        unimplemented!("clock_time_get")
    }

    fn fd_advise(
        &self,
        _fd: types::Fd,
        _offset: types::Filesize,
        _len: types::Filesize,
        _advice: types::Advice,
    ) -> Result<()> {
        unimplemented!("fd_advise")
    }

    fn fd_allocate(
        &self,
        _fd: types::Fd,
        _offset: types::Filesize,
        _len: types::Filesize,
    ) -> Result<()> {
        unimplemented!("fd_allocate")
    }

    fn fd_close(&self, _fd: types::Fd) -> Result<()> {
        unimplemented!("fd_close")
    }

    fn fd_datasync(&self, _fd: types::Fd) -> Result<()> {
        unimplemented!("fd_datasync")
    }

    fn fd_fdstat_get(&self, _fd: types::Fd) -> Result<types::Fdstat> {
        unimplemented!("fd_fdstat_get")
    }

    fn fd_fdstat_set_flags(&self, _fd: types::Fd, _flags: types::Fdflags) -> Result<()> {
        unimplemented!("fd_fdstat_set_flags")
    }

    fn fd_fdstat_set_rights(
        &self,
        _fd: types::Fd,
        _fs_rights_base: types::Rights,
        _fs_rights_inherting: types::Rights,
    ) -> Result<()> {
        unimplemented!("fd_fdstat_set_rights")
    }

    fn fd_filestat_get(&self, _fd: types::Fd) -> Result<types::Filestat> {
        unimplemented!("fd_filestat_get")
    }

    fn fd_filestat_set_size(&self, _fd: types::Fd, _size: types::Filesize) -> Result<()> {
        unimplemented!("fd_filestat_set_size")
    }

    fn fd_filestat_set_times(
        &self,
        _fd: types::Fd,
        _atim: types::Timestamp,
        _mtim: types::Timestamp,
        _fst_flags: types::Fstflags,
    ) -> Result<()> {
        unimplemented!("fd_filestat_set_times")
    }

    fn fd_pread(
        &self,
        _fd: types::Fd,
        _iovs: &types::IovecArray<'_>,
        _offset: types::Filesize,
    ) -> Result<types::Size> {
        unimplemented!("fd_pread")
    }

    fn fd_prestat_get(&self, _fd: types::Fd) -> Result<types::Prestat> {
        unimplemented!("fd_prestat_get")
    }

    fn fd_prestat_dir_name(
        &self,
        _fd: types::Fd,
        _path: &GuestPtr<u8>,
        _path_len: types::Size,
    ) -> Result<()> {
        unimplemented!("fd_prestat_dir_name")
    }

    fn fd_pwrite(
        &self,
        _fd: types::Fd,
        _ciovs: &types::CiovecArray<'_>,
        _offset: types::Filesize,
    ) -> Result<types::Size> {
        unimplemented!("fd_pwrite")
    }

    fn fd_read(&self, _fd: types::Fd, _iovs: &types::IovecArray<'_>) -> Result<types::Size> {
        unimplemented!("fd_read")
    }

    fn fd_readdir(
        &self,
        _fd: types::Fd,
        _buf: &GuestPtr<u8>,
        _buf_len: types::Size,
        _cookie: types::Dircookie,
    ) -> Result<types::Size> {
        unimplemented!("fd_readdir")
    }

    fn fd_renumber(&self, _fd: types::Fd, _to: types::Fd) -> Result<()> {
        unimplemented!("fd_renumber")
    }

    fn fd_seek(
        &self,
        _fd: types::Fd,
        _offset: types::Filedelta,
        _whence: types::Whence,
    ) -> Result<types::Filesize> {
        unimplemented!("fd_seek")
    }

    fn fd_sync(&self, _fd: types::Fd) -> Result<()> {
        unimplemented!("fd_sync")
    }

    fn fd_tell(&self, _fd: types::Fd) -> Result<types::Filesize> {
        unimplemented!("fd_tell")
    }

    fn fd_write(&self, _fd: types::Fd, _ciovs: &types::CiovecArray<'_>) -> Result<types::Size> {
        unimplemented!("fd_write")
    }

    fn path_create_directory(&self, _fd: types::Fd, _path: &GuestPtr<'_, str>) -> Result<()> {
        unimplemented!("path_create_directory")
    }

    fn path_filestat_get(
        &self,
        _fd: types::Fd,
        _flags: types::Lookupflags,
        _path: &GuestPtr<'_, str>,
    ) -> Result<types::Filestat> {
        unimplemented!("path_filestat_get")
    }

    fn path_filestat_set_times(
        &self,
        _fd: types::Fd,
        _flags: types::Lookupflags,
        _path: &GuestPtr<'_, str>,
        _atim: types::Timestamp,
        _mtim: types::Timestamp,
        _fst_flags: types::Fstflags,
    ) -> Result<()> {
        unimplemented!("path_filestat_set_times")
    }

    fn path_link(
        &self,
        _old_fd: types::Fd,
        _old_flags: types::Lookupflags,
        _old_path: &GuestPtr<'_, str>,
        _new_fd: types::Fd,
        _new_path: &GuestPtr<'_, str>,
    ) -> Result<()> {
        unimplemented!("path_link")
    }

    fn path_open(
        &self,
        _fd: types::Fd,
        _dirflags: types::Lookupflags,
        _path: &GuestPtr<'_, str>,
        _oflags: types::Oflags,
        _fs_rights_base: types::Rights,
        _fs_rights_inherting: types::Rights,
        _fdflags: types::Fdflags,
    ) -> Result<types::Fd> {
        unimplemented!("path_open")
    }

    fn path_readlink(
        &self,
        _fd: types::Fd,
        _path: &GuestPtr<'_, str>,
        _buf: &GuestPtr<u8>,
        _buf_len: types::Size,
    ) -> Result<types::Size> {
        unimplemented!("path_readlink")
    }

    fn path_remove_directory(&self, _fd: types::Fd, _path: &GuestPtr<'_, str>) -> Result<()> {
        unimplemented!("path_remove_directory")
    }

    fn path_rename(
        &self,
        _fd: types::Fd,
        _old_path: &GuestPtr<'_, str>,
        _new_fd: types::Fd,
        _new_path: &GuestPtr<'_, str>,
    ) -> Result<()> {
        unimplemented!("path_rename")
    }

    fn path_symlink(
        &self,
        _old_path: &GuestPtr<'_, str>,
        _fd: types::Fd,
        _new_path: &GuestPtr<'_, str>,
    ) -> Result<()> {
        unimplemented!("path_symlink")
    }

    fn path_unlink_file(&self, _fd: types::Fd, _path: &GuestPtr<'_, str>) -> Result<()> {
        unimplemented!("path_unlink_file")
    }

    fn poll_oneoff(
        &self,
        _in_: &GuestPtr<types::Subscription>,
        _out: &GuestPtr<types::Event>,
        _nsubscriptions: types::Size,
    ) -> Result<types::Size> {
        unimplemented!("poll_oneoff")
    }

    fn proc_exit(&self, _rval: types::Exitcode) -> std::result::Result<(), ()> {
        unimplemented!("proc_exit")
    }

    fn proc_raise(&self, _sig: types::Signal) -> Result<()> {
        unimplemented!("proc_raise")
    }

    fn sched_yield(&self) -> Result<()> {
        unimplemented!("sched_yield")
    }

    fn random_get(&self, _buf: &GuestPtr<u8>, _buf_len: types::Size) -> Result<()> {
        unimplemented!("random_get")
    }

    fn sock_recv(
        &self,
        _fd: types::Fd,
        _ri_data: &types::IovecArray<'_>,
        _ri_flags: types::Riflags,
    ) -> Result<(types::Size, types::Roflags)> {
        unimplemented!("sock_recv")
    }

    fn sock_send(
        &self,
        _fd: types::Fd,
        _si_data: &types::CiovecArray<'_>,
        _si_flags: types::Siflags,
    ) -> Result<types::Size> {
        unimplemented!("sock_send")
    }

    fn sock_shutdown(&self, _fd: types::Fd, _how: types::Sdflags) -> Result<()> {
        unimplemented!("sock_shutdown")
    }
}

/// Test the above generated code by running Wasm code that calls into it.
#[test]
fn main() {
    // The `init` function ensures that all of the host call functions are
    // linked into the executable.
    crate::hostcalls::init();
    // Same for lucet-runtime:
    lucet_runtime::lucet_internal_ensure_linked();

    // Temporary directory for outputs.
    let workdir = TempDir::new().expect("create working directory");

    // Build a C file into a Wasm module. Use the wasi-sdk compiler, but do
    // not use the wasi libc or the ordinary start files, which will together
    // expect various aspects of the Wasi trait to actually work. The C file
    // only imports one function, `args_sizes_get`.
    let wasm_build = Link::new(&["tests/wasi_guest.c"])
        .with_cflag("-nostartfiles")
        .with_link_opt(LinkOpt::NoDefaultEntryPoint)
        .with_link_opt(LinkOpt::AllowUndefinedAll)
        .with_link_opt(LinkOpt::ExportAll);
    let wasm_file = workdir.path().join("out.wasm");
    wasm_build.link(wasm_file.clone()).expect("link wasm");

    // We used lucet_wiggle to define the hostcall functions, so we must use
    // it to define our bindings as well. This is a good thing! No more
    // bindings json files to keep in sync with implementations.
    let witx_doc = witx::load(&["../wasi/phases/snapshot/witx/wasi_snapshot_preview1.witx"])
        .expect("load snapshot 1 witx");
    let bindings = lucet_wiggle_generate::bindings(&witx_doc);

    // Build a shared object with Lucetc:
    let native_build = Lucetc::new(wasm_file).with_bindings(bindings);
    let so_file = workdir.path().join("out.so");
    native_build
        .shared_object_file(so_file.clone())
        .expect("build so");

    // Load shared object into this executable.
    let module = DlModule::load(so_file).expect("load so");

    // Create an instance:
    let region = MmapRegion::create(1, &Limits::default()).expect("create region");
    let mut inst = region.new_instance(module).expect("create instance");

    // Define the TestCtx. This gets put into the embed ctx and is usable from
    // the trait method calls.
    let input_a: u32 = 123;
    let input_b: u32 = 567;
    let test_ctx = TestCtx {
        a: input_a,
        b: input_b,
        times_called: 0,
    };

    inst.insert_embed_ctx(test_ctx);

    // Call the `sum_of_arg_sizes` func defined in our C file. It in turn
    // calls `args_sizes_get`, and returns the sum of the two return values
    // from that function.
    let res = inst
        .run("sum_of_arg_sizes", &[])
        .expect("run sum_of_arg_sizes")
        .unwrap_returned();

    // Check that the return value is what we expected:
    assert_eq!(res.as_u32(), input_a + input_b);

    let tctx = inst
        .get_embed_ctx::<TestCtx>()
        .expect("get test ctx")
        .expect("borrow");
    // The test ctx `a` and `b` fields should be the same as they were
    // initialized to:
    assert_eq!(tctx.a, input_a);
    assert_eq!(tctx.b, input_b);
    // The `arg_sizes_get` trait method implementation should have incremented
    // `times_called` one time.
    assert_eq!(tctx.times_called, 1);
}
