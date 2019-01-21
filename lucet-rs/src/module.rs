use crate::errors::LucetError;
use lucet_sys::*;
use std::ffi::CString;
use std::path::Path;
use std::ptr;
use xfailure::xbail;

pub struct Module {
    pub(crate) lucet_module: *mut lucet_module,
}

unsafe impl Send for Module {}
unsafe impl Sync for Module {}

impl Module {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, LucetError> {
        let path_c = CString::new(
            path.as_ref()
                .to_str()
                .ok_or(LucetError::InternalError("No path"))?,
        )?;
        unsafe { lucet_report_load_errors(true) };
        let lucet_module = unsafe { lucet_module_load(path_c.as_ptr()) };
        if lucet_module.is_null() {
            xbail!(LucetError::RuntimeError("Unable to load the module"));
        }
        let module = Module { lucet_module };
        Ok(module)
    }
}

impl Drop for Module {
    fn drop(&mut self) {
        unsafe { lucet_module_unload(self.lucet_module) };
        self.lucet_module = ptr::null_mut();
    }
}
