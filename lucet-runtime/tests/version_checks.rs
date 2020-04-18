use lucet_runtime::{DlModule, Error};

#[test]
pub fn reject_old_modules() {
    // for platforms where modules are ELF (BSD, Linux, ...), exclude macos as it uses a different
    // file format (MachO)
    #[cfg(all(unix, not(target_os = "macos")))]
    const MODULE_PATH: &'static str = "./tests/version_checks/old_module.so";
    #[cfg(target_os = "macos")]
    const MODULE_PATH: &'static str = "./tests/version_checks/old_module.dylib";

    let err = DlModule::load(MODULE_PATH).err().unwrap();

    if let Error::ModuleError(e) = err {
        let msg = format!("{}", e);
        assert!(msg.contains("reserved bit is not set"));
        assert!(msg.contains("module is likely too old"));
    } else {
        panic!("unexpected error loading module: {}", err);
    }
}

#[test]
fn ensure_linked() {
    lucet_runtime::lucet_internal_ensure_linked();
}
