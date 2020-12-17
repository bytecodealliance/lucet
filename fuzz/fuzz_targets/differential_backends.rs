// N.B.: In order to successfully run this, one must invoke:
//
//     `RUSTFLAGS="-C link-args=-rdynamic" cargo +nightly fuzz run differential_backends ...`
//
// The reason that the `rustflags` option in ../.cargo/config is not used is
// that cargo-fuzz always passes in a RUSTFLAGS environment variable to its
// internal cargo invocation, and this environment variable overrides rather
// than augments the `rustflags` from the cargo config. (See
// rust-lang/cargo#6338 for details.)

#![no_main]

use anyhow::Error;
use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use lucet_runtime::{DlModule, Limits, MmapRegion, Region};
use lucetc::{BackendVariant, Lucetc, LucetcOpts};
use std::sync::Arc;
use tempfile::TempDir;

#[derive(Clone, Copy, Default, Debug, Arbitrary)]
struct WasmSmithConfig;

impl wasm_smith::Config for WasmSmithConfig {
    fn allow_start_export(&self) -> bool {
        true
    }

    fn min_funcs(&self) -> usize {
        1
    }

    fn max_funcs(&self) -> usize {
        1
    }

    fn min_memories(&self) -> u32 {
        1
    }

    fn max_memories(&self) -> usize {
        1
    }

    fn max_imports(&self) -> usize {
        0
    }

    fn min_exports(&self) -> usize {
        2
    }

    fn max_memory_pages(&self) -> u32 {
        1
    }

    fn memory_max_size_required(&self) -> bool {
        true
    }
}

fn build(
    variant: BackendVariant,
    bytes: &[u8],
    tempdir: &TempDir,
    suffix: &str,
) -> Result<Arc<DlModule>, Error> {
    let lucetc = Lucetc::try_from_bytes(bytes)?.with_backend_variant(variant);
    let so_file = tempdir.path().join(format!("out_{}.so", suffix));
    lucetc.shared_object_file(so_file.clone())?;
    let module = DlModule::load(so_file)?;
    Ok(module)
}

fn run_test(bytes: &[u8]) -> Result<(), Error> {
    let region = MmapRegion::create(2, &Limits::default())?;
    let tempdir = TempDir::new()?;
    let module_legacy = match build(BackendVariant::Legacy, bytes, &tempdir, "legacy") {
        Ok(m) => m,
        // Allow build to fail with legacy backend: wasm-smith can generate
        // modules that Lucet won't compile due to missing features, such as
        // multi-value support.
        Err(_) => {
            return Ok(());
        }
    };
    let module_machinst = build(BackendVariant::MachInst, bytes, &tempdir, "machinst")?;

    let mut inst_legacy = match region.new_instance_builder(module_legacy).build() {
        Ok(i) => i,
        // Likewise: allow instantiation of module compiled by legacy backend to
        // fail.
        Err(_) => {
            return Ok(());
        }
    };
    let mut inst_machinst = region.new_instance_builder(module_machinst).build()?;

    match inst_legacy.run_start() {
        Ok(_) => {}
        // If legacy backend fails, then module must have an issue (legacy
        // backend is our oracle for this fuzz target); return early success for
        // this case.
        Err(_) => {
            return Ok(());
        }
    };
    inst_machinst.run_start()?;

    assert_eq!(inst_legacy.heap(), inst_machinst.heap());
    assert_eq!(inst_legacy.globals().len(), inst_machinst.globals().len());
    for (g1, g2) in inst_legacy
        .globals()
        .iter()
        .zip(inst_machinst.globals().iter())
    {
        let g1 = unsafe { g1.i_64 };
        let g2 = unsafe { g2.i_64 };
        assert_eq!(g1, g2);
    }

    Ok(())
}

fuzz_target!(|module: wasm_smith::ConfiguredModule<WasmSmithConfig>| {
    // Ensure the hostcalls are available.
    lucet_runtime::lucet_internal_ensure_linked();

    // Ensure termination by requesting that wasm-smith instrument the module's
    // functions with a termination counter.
    //
    // N.B.: Lucet detects stack overflows by using a guard page below the
    // stack. This is a perfectly safe and legitimate thing to do, but libFuzzer
    // doesn't like segfaults -- it will cause a fuzzing failure and we cannot
    // catch it. So we must instead ensure that our maximum test-case runtime is
    // low enough that even an immediate recursive call will not go deep enough
    // to overflow the stack. 100 seems sufficient for this; wasm-smith's
    // termination counter is decremented by 1 on each function call and 100
    // stack frames fits in a 64K stack.
    let mut module = module;
    module.ensure_termination(100);
    let bytes = module.to_bytes();
    run_test(&bytes[..]).expect("differential test failed");
});
