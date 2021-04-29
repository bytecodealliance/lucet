use lucet_runtime::{DlModule, Error};
use lucetc::{Lucetc, LucetcOpts, VersionInfo};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use tempfile::TempDir;

#[test]
pub fn reject_old_modules() {
    // for platforms where modules are ELF (BSD, Linux, ...), exclude macos as it uses a different
    // file format (MachO)
    #[cfg(all(unix, not(target_os = "macos")))]
    const MODULE_PATH: &'static str = "./tests/version_checks/old_module.so";
    #[cfg(target_os = "macos")]
    const MODULE_PATH: &str = "./tests/version_checks/old_module.dylib";

    let err = DlModule::load(MODULE_PATH).err().unwrap();

    if let Error::ModuleError(e) = err {
        let msg = format!("{}", e);
        assert!(msg.contains("reserved bit is not set"));
        assert!(msg.contains("module is likely too old"));
    } else {
        panic!("unexpected error loading module: {}", err);
    }
}

pub fn wasm_test<P: AsRef<Path>>(
    workdir: &TempDir,
    wasm_file: P,
    version_info: VersionInfo,
) -> Result<PathBuf, lucetc::Error> {
    let native_build = Lucetc::new(wasm_file).with_version_info(version_info);

    let so_file = workdir.path().join("out.so");

    native_build.shared_object_file(so_file.clone())?;

    Ok(so_file)
}

#[test]
pub fn reject_incorrect_version_info() {
    let workdir = TempDir::new().expect("create working directory");

    let so_file = wasm_test(
        &workdir,
        "./tests/version_checks/trivial.wat",
        VersionInfo::from_str("1.2.3-456789ab").expect("parse version info"),
    )
    .expect("create object file from trivial.wat");

    let err = DlModule::load(so_file).err().unwrap();

    if let Error::ModuleError(e) = err {
        let msg = format!("{}", e);
        assert!(msg.contains("Incorrect module definition: version mismatch"));
    } else {
        panic!("unexpected error loading module: {}", err);
    }
}

#[test]
pub fn accept_incorrect_version_info() {
    let workdir = TempDir::new().expect("create working directory");

    let so_file = wasm_test(
        &workdir,
        "./tests/version_checks/trivial.wat",
        VersionInfo::from_str("1.2.3-456789ab").expect("parse version info"),
    )
    .expect("create object file from trivial.wat");

    let _module = DlModule::load_with_version_match(so_file, false).expect("load module");
}

#[test]
fn ensure_linked() {
    lucet_runtime::lucet_internal_ensure_linked();
}
