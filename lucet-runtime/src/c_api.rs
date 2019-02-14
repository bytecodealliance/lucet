#![allow(non_camel_case_types)]

use crate::{DlModule, InstanceHandle, Limits, MmapRegion, Module, Region};
use std::ffi::CStr;
use std::os::raw::{c_char, c_void};
use std::ptr;
use std::sync::Arc;

macro_rules! with_ffi_arcs {
    ( [ $($name:ident),+ ], $body:block ) => {{
        $(
            let $name = Arc::from_raw($name);
        )+
        let res = $body;
        $(
            Arc::into_raw($name);
        )+
        res
    }}
}

#[no_mangle]
pub extern "C" fn lucet_mmap_region_create(
    instance_capacity: usize,
    limits: &Limits,
) -> *const MmapRegion {
    MmapRegion::create(instance_capacity, limits)
        .map(Arc::into_raw)
        .unwrap_or(ptr::null())
}

#[no_mangle]
pub unsafe extern "C" fn lucet_mmap_region_release(region: *const MmapRegion) {
    Arc::from_raw(region);
}

// omg this naming convention might not scale
#[no_mangle]
pub unsafe extern "C" fn lucet_mmap_region_new_instance_with_ctx(
    region: *const MmapRegion,
    module: *const DlModule,
    embed_ctx: *mut c_void,
) -> *const InstanceHandle {
    with_ffi_arcs!([region, module], {
        region
            .new_instance_with_ctx(module.clone() as Arc<dyn Module>, embed_ctx)
            .map(|i| Box::into_raw(Box::new(i)) as *const InstanceHandle)
            .unwrap_or(ptr::null())
    })
}

#[no_mangle]
pub unsafe extern "C" fn lucet_mmap_region_new_instance(
    region: *const MmapRegion,
    module: *const DlModule,
) -> *const InstanceHandle {
    lucet_mmap_region_new_instance_with_ctx(region, module, ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn lucet_dl_module_load(path: *const c_char) -> *const DlModule {
    let path = CStr::from_ptr(path);
    DlModule::load(path.to_string_lossy().into_owned())
        .map(Arc::into_raw)
        .unwrap_or(ptr::null())
}

#[no_mangle]
pub unsafe extern "C" fn lucet_dl_module_release(module: *const DlModule) {
    Arc::from_raw(module);
}
