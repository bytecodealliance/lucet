use lucet_runtime::{DlModule, Error};

#[test]
pub fn reject_old_modules() {
    let err = DlModule::load("./tests/version_checks/old_module.so")
        .err()
        .unwrap();

    if let Error::ModuleError(e) = err {
        let msg = format!("{}", e);
        assert!(msg.contains("reserved bit is not set"));
        assert!(msg.contains("module is likely too old"));
    } else {
        panic!("unexpected error loading module: {}", err);
    }
}
