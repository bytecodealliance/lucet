use crate::module::DlModule;
use failure::Error;
use std::env;
use std::path::{Path, PathBuf};

fn guest_module_path<P: AsRef<Path>>(path: P) -> PathBuf {
    if let Some(prefix) = env::var_os("GUEST_MODULE_PREFIX") {
        Path::new(&prefix).join(path)
    } else {
        Path::new("/isolation/public").join(path)
    }
}

impl DlModule {
    pub(crate) fn load_test<P: AsRef<Path>>(so_path: P) -> Result<Self, Error> {
        DlModule::load(guest_module_path(so_path))
    }
}
