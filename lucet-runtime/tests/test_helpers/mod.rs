use failure::Error;
use lazy_static::lazy_static;
use lucet_runtime::module::DlModule;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

lazy_static! {
    static ref EXCLUSIVE_TEST: RwLock<()> = RwLock::default();
}

/// Run a test non-exclusively with other `test_nonex` tests.
///
/// This function _must_ wrap any uses of `DlModule` or `Instance::run()`.
#[allow(dead_code)]
pub fn test_nonex<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let lock = EXCLUSIVE_TEST.read().unwrap();
    let r = f();
    drop(lock);
    r
}

/// Run a test exclusively, so that no other `test_nonex` or `test_ex` tests will run concurrently.
///
/// This function _must_ wrap any tests that use `fork` or that set a custom `sigaction`.
#[allow(dead_code)]
pub fn test_ex<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let lock = EXCLUSIVE_TEST.write().unwrap();
    let r = f();
    drop(lock);
    r
}

pub fn guest_module_path<P: AsRef<Path>>(path: P) -> PathBuf {
    if let Some(prefix) = env::var_os("GUEST_MODULE_PREFIX") {
        Path::new(&prefix).join(path)
    } else {
        Path::new("/isolation/public").join(path)
    }
}

pub trait DlModuleExt {
    fn load_test<P: AsRef<Path>>(so_path: P) -> Result<DlModule, Error>;
}

impl DlModuleExt for DlModule {
    fn load_test<P: AsRef<Path>>(so_path: P) -> Result<DlModule, Error> {
        DlModule::load(guest_module_path(so_path))
    }
}
