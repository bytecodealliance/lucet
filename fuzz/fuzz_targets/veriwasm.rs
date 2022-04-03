//! VeriWasm fuzz target: ensure that VeriWasm does not reject the compilation
//! result of any valid Wasm module.

#![no_main]

use anyhow::Error;
use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use lucetc::{Lucetc, LucetcOpts};
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

    fn max_data_segments(&self) -> usize {
        0
    }

    fn max_tables(&self) -> usize {
        0
    }

    fn memory_max_size_required(&self) -> bool {
        true
    }
}

fn build(with_veriwasm: bool, bytes: &[u8], tempdir: &TempDir, suffix: &str) -> Result<(), Error> {
    let lucetc = Lucetc::try_from_bytes(bytes)?.with_veriwasm(with_veriwasm);
    let so_file = tempdir.path().join(format!("out_{}.so", suffix));
    lucetc.shared_object_file(so_file)?;
    Ok(())
}

fn run_test(bytes: &[u8]) -> Result<(), Error> {
    let tempdir = TempDir::new()?;
    if build(
        /* with_veriwasm = */ false, bytes, &tempdir, "baseline",
    )
    .is_err()
    {
        // If baseline Lucet can't build it, then there is some other issue with the Wasm module
        // and we reject it with a silent "pass" result.
        return Ok(());
    }

    build(/* with_veriwasm = */ true, bytes, &tempdir, "veriwasm")?;

    Ok(())
}

fuzz_target!(|module: wasm_smith::ConfiguredModule<WasmSmithConfig>| {
    let _ = env_logger::try_init();
    let bytes = module.to_bytes();
    run_test(&bytes[..]).expect("build with VeriWasm check failed");
});
