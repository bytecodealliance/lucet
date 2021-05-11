use std::convert::TryInto;

lucet_wiggle::from_witx!({
    witx: ["$CARGO_MANIFEST_DIR/tests/atoms.witx"],
    constructor: { crate::Ctx },
    async_: {
        atoms::double_int_return_float
    }
});

pub struct Ctx;
impl wiggle::GuestErrorType for types::Errno {
    fn success() -> Self {
        types::Errno::Ok
    }
}

#[lucet_wiggle::async_trait]
impl atoms::Atoms for Ctx {
    fn int_float_args(&self, an_int: u32, an_float: f32) -> Result<(), types::Errno> {
        println!("INT FLOAT ARGS: {} {}", an_int, an_float);
        Ok(())
    }
    async fn double_int_return_float(
        &self,
        an_int: u32,
    ) -> Result<types::AliasToFloat, types::Errno> {
        println!("DOUBLE INT RETURN FLOAT: {}", an_int);
        Ok((an_int as f32) * 2.0)
    }
}

/// Test the above generated code by running Wasm code that calls into it.
#[test]
fn main() {
    use tempfile::TempDir;
    use std::path::PathBuf;
    use lucetc::{Lucetc, LucetcOpts};
    use lucet_runtime::{DlModule, Limits, MmapRegion, Region};
    // The `init` function ensures that all of the host call functions are
    // linked into the executable.
    crate::hostcalls::init();
    // Same for lucet-runtime:
    lucet_runtime::lucet_internal_ensure_linked();

    // Temporary directory for outputs.
    let workdir = TempDir::new().expect("create working directory");

    // We used lucet_wiggle to define the hostcall functions, so we must use
    // it to define our bindings as well. This is a good thing! No more
    // bindings json files to keep in sync with implementations.
    let witx_path = PathBuf::from("tests/atoms.witx");
    let witx_doc = lucet_wiggle::witx::load(&[witx_path]).expect("load atoms.witx");
    let bindings = lucet_wiggle_generate::bindings(&witx_doc);

    // Build a shared object with Lucetc:
    let native_build = Lucetc::new("tests/atoms_async_guest.wat").with_bindings(bindings);
    let so_file = workdir.path().join("out.so");
    native_build
        .shared_object_file(so_file.clone())
        .expect("build so");

    // Load shared object into this executable.
    let module = DlModule::load(so_file).expect("load so");

    // Create an instance:
    let region = MmapRegion::create(1, &Limits::default()).expect("create region");
    let mut inst = region.new_instance(module).expect("create instance");
    inst.run_start().expect("start section runs");

    // Create a Ctx used by the host calls, an
    let ctx = Ctx;
    inst.insert_embed_ctx(ctx);

    // Synchronously run a function that does not make an async hostcall.
    let res = inst.run("int_float_args_shim", &[0i32.into(), 123.45f32.into()]).expect("run int_float_args_shim").unwrap_returned();

    assert_eq!(res.as_u32(),types::Errno::Ok as u32);

    inst.reset().expect("can reset instance");

    let input = 123;
    let result_location = 0;

    let results = futures_executor::block_on(inst
        .run_async("double_int_return_float_shim", &[input.into(), result_location.into()], Some(10000)))
    .expect("run_async double_int_return_float_shim");

    assert_eq!(
        results.as_i32(),
        types::Errno::Ok as i32,
        "double_int_return_float errno"
    );

    // The actual result is in memory:
    let r = result_location as usize..(result_location as usize + 4);
    let result = f32::from_le_bytes(inst.heap()[r].try_into().unwrap());
    assert_eq!((input * 2) as f32, result);

}
