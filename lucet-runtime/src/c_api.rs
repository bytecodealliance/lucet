extern crate lucet_runtime_internals;

use crate::{DlModule, Instance, Limits, MmapRegion, Module, Region, TrapCode};
use libc::{c_char, c_int, c_void};
use lucet_runtime_internals::c_api::*;
use lucet_runtime_internals::instance::{
    instance_handle_from_raw, instance_handle_to_raw, InstanceInternal,
};
use lucet_runtime_internals::vmctx::{instance_from_vmctx, lucet_vmctx, Vmctx, VmctxInternal};
use lucet_runtime_internals::WASM_PAGE_SIZE;
use lucet_runtime_internals::{assert_nonnull, with_ffi_arcs};
use num_traits::FromPrimitive;
use std::ffi::CStr;
use std::ptr;
use std::sync::{Arc, Once};

macro_rules! with_instance_ptr {
    ( $name:ident, $body:block ) => {{
        assert_nonnull!($name);
        let $name: &mut Instance = &mut *($name as *mut Instance);
        $body
    }};
}

macro_rules! with_instance_ptr_unchecked {
    ( $name:ident, $body:block ) => {{
        let $name: &mut Instance = &mut *($name as *mut Instance);
        $body
    }};
}

#[no_mangle]
pub extern "C" fn lucet_error_name(e: c_int) -> *const c_char {
    if let Some(e) = lucet_error::from_i32(e) {
        use self::lucet_error::*;
        match e {
            Ok => "lucet_error_ok\0".as_ptr() as _,
            InvalidArgument => "lucet_error_invalid_argument\0".as_ptr() as _,
            RegionFull => "lucet_error_region_full\0".as_ptr() as _,
            Module => "lucet_error_module\0".as_ptr() as _,
            LimitsExceeded => "lucet_error_limits_exceeded\0".as_ptr() as _,
            SymbolNotFound => "lucet_error_symbol_not_found\0".as_ptr() as _,
            FuncNotFound => "lucet_error_func_not_found\0".as_ptr() as _,
            RuntimeFault => "lucet_error_runtime_fault\0".as_ptr() as _,
            RuntimeTerminated => "lucet_error_runtime_terminated\0".as_ptr() as _,
            Dl => "lucet_error_dl\0".as_ptr() as _,
            Internal => "lucet_error_internal\0".as_ptr() as _,
            Unsupported => "lucet_error_unsupported\0".as_ptr() as _,
        }
    } else {
        "!!! error: unknown lucet_error variant\0".as_ptr() as _
    }
}

#[no_mangle]
pub extern "C" fn lucet_state_tag_name(tag: libc::c_int) -> *const c_char {
    if let Some(tag) = lucet_state_tag::from_i32(tag) {
        use self::lucet_state_tag::*;
        match tag {
            Returned => "lucet_state_tag_returned\0".as_ptr() as _,
            Running => "lucet_state_tag_running\0".as_ptr() as _,
            Fault => "lucet_state_tag_fault\0".as_ptr() as _,
            Terminated => "lucet_state_tag_terminated\0".as_ptr() as _,
        }
    } else {
        "!!! unknown lucet_state_tag variant!\0".as_ptr() as _
    }
}

#[no_mangle]
pub unsafe extern "C" fn lucet_mmap_region_create(
    instance_capacity: u64,
    limits: *const lucet_alloc_limits,
    region_out: *mut *mut lucet_region,
) -> lucet_error {
    assert_nonnull!(region_out);
    let limits = limits
        .as_ref()
        .map(|l| l.into())
        .unwrap_or(Limits::default());
    match MmapRegion::create(instance_capacity as usize, &limits) {
        Ok(region) => {
            let region_thin = Arc::into_raw(Arc::new(region as Arc<dyn Region>));
            region_out.write(region_thin as _);
            return lucet_error::Ok;
        }
        Err(e) => return e.into(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn lucet_region_release(region: *const lucet_region) {
    Arc::from_raw(region as *const Arc<dyn Region>);
}

// omg this naming convention might not scale
#[no_mangle]
pub unsafe extern "C" fn lucet_region_new_instance_with_ctx(
    region: *const lucet_region,
    module: *const lucet_dl_module,
    embed_ctx: *mut c_void,
    inst_out: *mut *mut lucet_instance,
) -> lucet_error {
    assert_nonnull!(inst_out);
    with_ffi_arcs!([region: dyn Region, module: DlModule], {
        region
            .new_instance_builder(module.clone() as Arc<dyn Module>)
            .with_embed_ctx(embed_ctx)
            .build()
            .map(|i| {
                inst_out.write(instance_handle_to_raw(i) as _);
                lucet_error::Ok
            })
            .unwrap_or_else(|e| e.into())
    })
}

#[no_mangle]
pub unsafe extern "C" fn lucet_region_new_instance(
    region: *const lucet_region,
    module: *const lucet_dl_module,
    inst_out: *mut *mut lucet_instance,
) -> lucet_error {
    lucet_region_new_instance_with_ctx(region, module, ptr::null_mut(), inst_out)
}

#[no_mangle]
pub unsafe extern "C" fn lucet_dl_module_load(
    path: *const c_char,
    mod_out: *mut *mut lucet_dl_module,
) -> lucet_error {
    assert_nonnull!(mod_out);
    let path = CStr::from_ptr(path);
    DlModule::load(path.to_string_lossy().into_owned())
        .map(|m| {
            mod_out.write(Arc::into_raw(m) as _);
            lucet_error::Ok
        })
        .unwrap_or_else(|e| e.into())
}

#[no_mangle]
pub unsafe extern "C" fn lucet_dl_module_release(module: *const lucet_dl_module) {
    Arc::from_raw(module as *const DlModule);
}

#[no_mangle]
pub unsafe extern "C" fn lucet_instance_run(
    inst: *mut lucet_instance,
    entrypoint: *const c_char,
    argc: usize,
    argv: *const lucet_val::lucet_val,
) -> lucet_error {
    assert_nonnull!(entrypoint);
    if argc != 0 && argv.is_null() {
        return lucet_error::InvalidArgument;
    }
    let args = if argc == 0 {
        vec![]
    } else {
        std::slice::from_raw_parts(argv, argc)
            .into_iter()
            .map(|v| v.into())
            .collect()
    };
    with_instance_ptr!(inst, {
        let entrypoint = CStr::from_ptr(entrypoint);
        inst.run(entrypoint.to_bytes(), args.as_slice())
            .map(|_| lucet_error::Ok)
            .unwrap_or_else(|e| {
                eprintln!("{}", e);
                e.into()
            })
    })
}

#[no_mangle]
pub unsafe extern "C" fn lucet_instance_run_func_idx(
    inst: *mut lucet_instance,
    table_idx: u32,
    func_idx: u32,
    argc: usize,
    argv: *const lucet_val::lucet_val,
) -> lucet_error {
    if argc != 0 && argv.is_null() {
        return lucet_error::InvalidArgument;
    }
    let args = if argc == 0 {
        vec![]
    } else {
        std::slice::from_raw_parts(argv, argc)
            .into_iter()
            .map(|v| v.into())
            .collect()
    };
    with_instance_ptr!(inst, {
        inst.run_func_idx(table_idx, func_idx, args.as_slice())
            .map(|_| lucet_error::Ok)
            .unwrap_or_else(|e| e.into())
    })
}

#[no_mangle]
pub unsafe extern "C" fn lucet_instance_state(
    inst: *const lucet_instance,
    state_out: *mut lucet_state::lucet_state,
) -> lucet_error {
    assert_nonnull!(state_out);
    with_instance_ptr!(inst, {
        state_out.write(inst.state().into());
        lucet_error::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn lucet_state_release(state: *mut lucet_state::lucet_state) {
    use lucet_runtime_internals::c_api::lucet_state::*;
    use std::ffi::CString;

    let state = state.read();
    if let lucet_state_tag::Fault = state.tag {
        let addr_details = state.val.fault.rip_addr_details;
        // free the strings
        CString::from_raw(addr_details.file_name as *mut _);
        CString::from_raw(addr_details.sym_name as *mut _);
    }
}

#[no_mangle]
pub unsafe extern "C" fn lucet_instance_reset(inst: *mut lucet_instance) -> lucet_error {
    with_instance_ptr!(inst, {
        inst.reset()
            .map(|_| lucet_error::Ok)
            .unwrap_or_else(|e| e.into())
    })
}

#[no_mangle]
pub unsafe extern "C" fn lucet_instance_release(inst: *mut lucet_instance) {
    instance_handle_from_raw(inst as *mut Instance);
}

#[no_mangle]
pub unsafe extern "C" fn lucet_instance_heap(inst: *mut lucet_instance) -> *mut u8 {
    with_instance_ptr_unchecked!(inst, { inst.heap_mut().as_mut_ptr() })
}

#[no_mangle]
pub unsafe extern "C" fn lucet_instance_heap_len(inst: *const lucet_instance) -> u32 {
    with_instance_ptr_unchecked!(inst, { inst.heap().len() as u32 })
}

#[no_mangle]
pub unsafe extern "C" fn lucet_instance_check_heap(
    inst: *const lucet_instance,
    ptr: *const c_void,
    len: usize,
) -> bool {
    with_instance_ptr_unchecked!(inst, { inst.check_heap(ptr, len) })
}

#[no_mangle]
pub unsafe extern "C" fn lucet_instance_grow_heap(
    inst: *mut lucet_instance,
    additional_pages: u32,
    previous_pages_out: *mut u32,
) -> lucet_error {
    with_instance_ptr!(inst, {
        match inst.grow_memory(additional_pages) {
            Ok(previous_pages) => {
                if !previous_pages_out.is_null() {
                    previous_pages_out.write(previous_pages);
                }
                lucet_error::Ok
            }
            Err(e) => e.into(),
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn lucet_instance_embed_ctx(inst: *mut lucet_instance) -> *mut c_void {
    with_instance_ptr_unchecked!(inst, {
        inst.get_embed_ctx::<*mut c_void>()
            .map(|p| *p)
            .unwrap_or(ptr::null_mut())
    })
}

/// Release or run* must not be called in the body of this function!
#[no_mangle]
pub unsafe extern "C" fn lucet_instance_set_signal_handler(
    inst: *mut lucet_instance,
    signal_handler: lucet_signal_handler,
) -> lucet_error {
    let handler = move |inst: &Instance, trap: &Option<TrapCode>, signum, siginfo, context| {
        let inst = inst as *const Instance as *mut lucet_instance;
        let trap = trap.into();
        let trap_ptr = &trap as *const lucet_state::lucet_trapcode;
        let res = signal_handler(inst, trap_ptr, signum, siginfo, context).into();
        // make sure `trap_ptr` is live until the signal handler returns
        drop(trap);
        res
    };
    with_instance_ptr!(inst, {
        inst.set_signal_handler(handler);
    });
    lucet_error::Ok
}

#[no_mangle]
pub unsafe extern "C" fn lucet_instance_set_fatal_handler(
    inst: *mut lucet_instance,
    fatal_handler: lucet_fatal_handler,
) -> lucet_error {
    // transmuting is fine here because *mut lucet_instance = *mut Instance
    let fatal_handler: unsafe extern "C" fn(inst: *mut Instance) =
        std::mem::transmute(fatal_handler);
    with_instance_ptr!(inst, {
        inst.set_c_fatal_handler(fatal_handler);
    });
    lucet_error::Ok
}

#[no_mangle]
pub unsafe extern "C" fn lucet_retval_gp(retval: *const lucet_untyped_retval) -> lucet_retval_gp {
    lucet_retval_gp {
        as_untyped: (*retval).gp,
    }
}

#[no_mangle]
pub unsafe extern "C" fn lucet_retval_f32(retval: *const lucet_untyped_retval) -> f32 {
    let mut v = 0.0f32;
    core::arch::x86_64::_mm_storeu_ps(
        &mut v as *mut f32,
        core::arch::x86_64::_mm_loadu_ps((*retval).fp.as_ptr() as *const f32),
    );
    v
}

#[no_mangle]
pub unsafe extern "C" fn lucet_retval_f64(retval: *const lucet_untyped_retval) -> f64 {
    let mut v = 0.0f64;
    core::arch::x86_64::_mm_storeu_pd(
        &mut v as *mut f64,
        core::arch::x86_64::_mm_loadu_pd((*retval).fp.as_ptr() as *const f64),
    );
    v
}

static C_API_INIT: Once = Once::new();

/// Should never actually be called, but should be reachable via a trait method to prevent DCE.
pub fn ensure_linked() {
    use std::ptr::read_volatile;
    C_API_INIT.call_once(|| unsafe {
        read_volatile(lucet_vmctx_get_heap as *const extern "C" fn());
        read_volatile(lucet_vmctx_current_memory as *const extern "C" fn());
        read_volatile(lucet_vmctx_grow_memory as *const extern "C" fn());
    });
}

#[no_mangle]
pub unsafe extern "C" fn lucet_vmctx_get_heap(vmctx: *mut lucet_vmctx) -> *mut u8 {
    Vmctx::from_raw(vmctx).instance().alloc().slot().heap as *mut u8
}

#[no_mangle]
pub unsafe extern "C" fn lucet_vmctx_get_globals(vmctx: *mut lucet_vmctx) -> *mut i64 {
    Vmctx::from_raw(vmctx).instance().alloc().slot().globals as *mut i64
}

/// Get the number of WebAssembly pages currently in the heap.
#[no_mangle]
pub unsafe extern "C" fn lucet_vmctx_current_memory(vmctx: *mut lucet_vmctx) -> libc::uint32_t {
    Vmctx::from_raw(vmctx).instance().alloc().heap_len() as u32 / WASM_PAGE_SIZE
}

#[no_mangle]
/// Grows the guest heap by the given number of WebAssembly pages.
///
/// On success, returns the number of pages that existed before the call. On failure, returns `-1`.
pub unsafe extern "C" fn lucet_vmctx_grow_memory(
    vmctx: *mut lucet_vmctx,
    additional_pages: libc::uint32_t,
) -> libc::int32_t {
    let inst = instance_from_vmctx(vmctx);
    if let Ok(old_pages) = inst.grow_memory(additional_pages) {
        old_pages as libc::int32_t
    } else {
        -1
    }
}

#[no_mangle]
/// Check if a memory region is inside the instance heap.
pub unsafe extern "C" fn lucet_vmctx_check_heap(
    vmctx: *mut lucet_vmctx,
    ptr: *mut c_void,
    len: libc::size_t,
) -> bool {
    let inst = instance_from_vmctx(vmctx);
    inst.check_heap(ptr, len)
}

#[no_mangle]
pub unsafe extern "C" fn lucet_vmctx_get_func_from_idx(
    vmctx: *mut lucet_vmctx,
    table_idx: u32,
    func_idx: u32,
) -> *const c_void {
    let inst = instance_from_vmctx(vmctx);
    inst.module()
        .get_func_from_idx(table_idx, func_idx)
        // the Rust API actually returns a pointer to a function pointer, so we want to dereference
        // one layer of that to make it nicer in C
        .map(|fptr| *(fptr as *const *const c_void))
        .unwrap_or(std::ptr::null())
}

#[no_mangle]
pub unsafe extern "C" fn lucet_vmctx_terminate(vmctx: *mut lucet_vmctx, info: *mut c_void) {
    Vmctx::from_raw(vmctx).terminate(info);
}

#[no_mangle]
/// Get the delegate object for the current instance.
///
/// TODO: rename
pub unsafe extern "C" fn lucet_vmctx_get_delegate(vmctx: *mut lucet_vmctx) -> *mut c_void {
    let inst = instance_from_vmctx(vmctx);
    inst.get_embed_ctx::<*mut c_void>()
        .map(|p| *p)
        .unwrap_or(std::ptr::null_mut())
}
