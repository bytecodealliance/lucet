#![allow(non_camel_case_types)]

use crate::{DlModule, Error, InstanceHandle, Limits, MmapRegion, Module, Region};
use libc;
use std::ffi::CStr;
use std::os::raw::{c_char, c_void};
use std::ptr;
use std::sync::Arc;

macro_rules! assert_nonnull {
    ( $name:ident ) => {
        if $name.is_null() {
            return lucet_error::InvalidArgument;
        }
    };
}

macro_rules! with_ffi_arcs {
    ( [ $($name:ident : $ty:ty),+ ], $body:block ) => {{
        $(
            assert_nonnull!($name);
            let $name = Arc::from_raw($name as *const $ty);
        )+
        let res = $body;
        $(
            Arc::into_raw($name);
        )+
        res
    }}
}

macro_rules! with_ffi_boxes {
    ( [ $($name:ident : $ty:ty),+ ], $body:block ) => {{
        $(
            assert_nonnull!($name);
            #[allow(unused_mut)]
            let mut $name = Box::from_raw($name as *mut $ty);
        )+
        let res = $body;
        $(
            Box::into_raw($name);
        )+
        res
    }}
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub enum lucet_error {
    Ok,
    InvalidArgument,
    RegionFull,
    Module,
    LimitsExceeded,
    SymbolNotFound,
    FuncNotFound,
    RuntimeFault,
    RuntimeTerminated,
    Dl,
    Internal,
    Unsupported,
}

impl From<Error> for lucet_error {
    fn from(e: Error) -> lucet_error {
        match e {
            Error::InvalidArgument(_) => lucet_error::InvalidArgument,
            Error::RegionFull(_) => lucet_error::RegionFull,
            Error::ModuleError(_) => lucet_error::Module,
            Error::LimitsExceeded(_) => lucet_error::LimitsExceeded,
            Error::SymbolNotFound(_) => lucet_error::SymbolNotFound,
            Error::FuncNotFound(_, _) => lucet_error::FuncNotFound,
            Error::RuntimeFault(_) => lucet_error::RuntimeFault,
            Error::RuntimeTerminated(_) => lucet_error::RuntimeTerminated,
            Error::DlError(_) => lucet_error::Dl,
            Error::InternalError(_) => lucet_error::Internal,
            Error::Unsupported(_) => lucet_error::Unsupported,
        }
    }
}

pub struct lucet_instance {
    _unused: [u8; 0],
}

pub struct lucet_mmap_region {
    _unused: [u8; 0],
}

pub struct lucet_dl_module {
    _unused: [u8; 0],
}

/// Runtime limits for the various memories that back a Lucet instance.
///
/// Each value is specified in bytes, and must be evenly divisible by the host page size (4K).
#[derive(Clone, Debug)]
#[repr(C)]
pub struct lucet_alloc_limits {
    /// Max size of the heap, which can be backed by real memory. (default 1M)
    pub heap_memory_size: u64,
    /// Size of total virtual memory. (default 8G)
    pub heap_address_space_size: u64,
    /// Size of the guest stack. (default 128K)
    pub stack_size: u64,
    /// Size of the globals region in bytes; each global uses 8 bytes. (default 4K)
    pub globals_size: u64,
}

impl From<Limits> for lucet_alloc_limits {
    fn from(limits: Limits) -> lucet_alloc_limits {
        limits.into()
    }
}

impl From<&Limits> for lucet_alloc_limits {
    fn from(limits: &Limits) -> lucet_alloc_limits {
        lucet_alloc_limits {
            heap_memory_size: limits.heap_memory_size as u64,
            heap_address_space_size: limits.heap_address_space_size as u64,
            stack_size: limits.stack_size as u64,
            globals_size: limits.globals_size as u64,
        }
    }
}

impl From<lucet_alloc_limits> for Limits {
    fn from(limits: lucet_alloc_limits) -> Limits {
        limits.into()
    }
}

impl From<&lucet_alloc_limits> for Limits {
    fn from(limits: &lucet_alloc_limits) -> Limits {
        Limits {
            heap_memory_size: limits.heap_memory_size as usize,
            heap_address_space_size: limits.heap_address_space_size as usize,
            stack_size: limits.stack_size as usize,
            globals_size: limits.globals_size as usize,
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn lucet_mmap_region_create(
    instance_capacity: u64,
    limits: &lucet_alloc_limits,
    region_out: *mut *const lucet_mmap_region,
) -> lucet_error {
    assert_nonnull!(region_out);
    match MmapRegion::create(instance_capacity as usize, &limits.into()) {
        Ok(region) => {
            region_out.write(Arc::into_raw(region) as _);
            return lucet_error::Ok;
        }
        Err(e) => return e.into(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn lucet_mmap_region_release(region: *const lucet_mmap_region) {
    Arc::from_raw(region as *const MmapRegion);
}

// omg this naming convention might not scale
#[no_mangle]
pub unsafe extern "C" fn lucet_mmap_region_new_instance_with_ctx(
    region: *const lucet_mmap_region,
    module: *const lucet_dl_module,
    embed_ctx: *mut c_void,
    inst_out: *mut *mut lucet_instance,
) -> lucet_error {
    assert_nonnull!(inst_out);
    with_ffi_arcs!([region: MmapRegion, module: DlModule], {
        region
            .new_instance_with_ctx(module.clone() as Arc<dyn Module>, embed_ctx)
            .map(|i| {
                inst_out.write(Box::into_raw(Box::new(i)) as _);
                lucet_error::Ok
            })
            .unwrap_or_else(|e| e.into())
    })
}

#[no_mangle]
pub unsafe extern "C" fn lucet_mmap_region_new_instance(
    region: *const lucet_mmap_region,
    module: *const lucet_dl_module,
    inst_out: *mut *mut lucet_instance,
) -> lucet_error {
    lucet_mmap_region_new_instance_with_ctx(region, module, ptr::null_mut(), inst_out)
}

#[no_mangle]
pub unsafe extern "C" fn lucet_dl_module_load(
    path: *const c_char,
    mod_out: *mut *const lucet_dl_module,
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
    with_ffi_boxes!([inst: InstanceHandle], {
        let entrypoint = CStr::from_ptr(entrypoint);
        inst.run(entrypoint.to_bytes(), args.as_slice())
            .map(|_| lucet_error::Ok)
            .unwrap_or_else(|e| e.into())
    })
}

mod lucet_state {
    use crate::c_api::lucet_val;
    use libc::{c_char, c_void};
    use lucet_runtime_internals::instance::State;
    use lucet_runtime_internals::module::AddrDetails;
    use lucet_runtime_internals::trapcode::{TrapCode, TrapCodeType};
    use std::ffi::CString;
    use std::ptr;

    impl From<&State> for lucet_state {
        fn from(state: &State) -> lucet_state {
            match state {
                State::Ready { retval } => lucet_state {
                    tag: lucet_state_tag::Returned,
                    val: lucet_state_val {
                        returned: retval.into(),
                    },
                },
                State::Running => lucet_state {
                    tag: lucet_state_tag::Running,
                    val: lucet_state_val { running: true },
                },
                State::Fault {
                    details,
                    siginfo,
                    context,
                } => lucet_state {
                    tag: lucet_state_tag::Fault,
                    val: lucet_state_val {
                        fault: lucet_runtime_fault {
                            fatal: details.fatal,
                            trapcode: details.trapcode.into(),
                            rip_addr: details.rip_addr,
                            rip_addr_details: (&details.rip_addr_details).into(),
                            signal_info: *siginfo,
                            context: *context,
                        },
                    },
                },
                State::Terminated { details } => lucet_state {
                    tag: lucet_state_tag::Terminated,
                    val: lucet_state_val {
                        terminated: details.info,
                    },
                },
            }
        }
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct lucet_state {
        pub tag: lucet_state_tag,
        pub val: lucet_state_val,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Debug)]
    pub enum lucet_state_tag {
        Returned,
        Running,
        Fault,
        Terminated,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub union lucet_state_val {
        pub returned: lucet_val::lucet_untyped_retval,
        // no meaning to this boolean, it's just there so the type is FFI-safe
        pub running: bool,
        pub fault: lucet_runtime_fault,
        pub terminated: *mut c_void,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct lucet_runtime_fault {
        pub fatal: bool,
        pub trapcode: lucet_trapcode,
        pub rip_addr: libc::uintptr_t,
        pub rip_addr_details: lucet_module_addr_details,
        pub signal_info: libc::siginfo_t,
        pub context: libc::ucontext_t,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Debug)]
    pub enum lucet_trapcode_type {
        StackOverflow,
        HeapOutOfBounds,
        OutOfBounds,
        IndirectCallToNull,
        BadSignature,
        IntegerOverflow,
        IntegerDivByZero,
        BadConversionToInteger,
        Interrupt,
        TableOutOfBounds,
        User,
        Unknown,
    }

    impl From<TrapCodeType> for lucet_trapcode_type {
        fn from(ty: TrapCodeType) -> lucet_trapcode_type {
            ty.into()
        }
    }

    impl From<&TrapCodeType> for lucet_trapcode_type {
        fn from(ty: &TrapCodeType) -> lucet_trapcode_type {
            match ty {
                TrapCodeType::StackOverflow => lucet_trapcode_type::StackOverflow,
                TrapCodeType::HeapOutOfBounds => lucet_trapcode_type::HeapOutOfBounds,
                TrapCodeType::OutOfBounds => lucet_trapcode_type::OutOfBounds,
                TrapCodeType::IndirectCallToNull => lucet_trapcode_type::IndirectCallToNull,
                TrapCodeType::BadSignature => lucet_trapcode_type::BadSignature,
                TrapCodeType::IntegerOverflow => lucet_trapcode_type::IntegerOverflow,
                TrapCodeType::IntegerDivByZero => lucet_trapcode_type::IntegerDivByZero,
                TrapCodeType::BadConversionToInteger => lucet_trapcode_type::BadConversionToInteger,
                TrapCodeType::Interrupt => lucet_trapcode_type::Interrupt,
                TrapCodeType::TableOutOfBounds => lucet_trapcode_type::TableOutOfBounds,
                TrapCodeType::User => lucet_trapcode_type::User,
                TrapCodeType::Unknown => lucet_trapcode_type::Unknown,
            }
        }
    }

    #[repr(C)]
    #[derive(Clone, Copy, Debug)]
    pub struct lucet_trapcode {
        code: lucet_trapcode_type,
        tag: u16,
    }

    impl From<TrapCode> for lucet_trapcode {
        fn from(trap: TrapCode) -> lucet_trapcode {
            trap.into()
        }
    }

    impl From<&TrapCode> for lucet_trapcode {
        fn from(trap: &TrapCode) -> lucet_trapcode {
            lucet_trapcode {
                code: trap.ty.into(),
                tag: trap.tag,
            }
        }
    }

    #[repr(C)]
    #[derive(Clone, Copy, Debug)]
    pub struct lucet_module_addr_details {
        pub module_code_resolvable: bool,
        pub in_module_code: bool,
        pub file_name: *const c_char,
        pub sym_name: *const c_char,
    }

    impl Default for lucet_module_addr_details {
        fn default() -> Self {
            lucet_module_addr_details {
                module_code_resolvable: false,
                in_module_code: false,
                file_name: ptr::null(),
                sym_name: ptr::null(),
            }
        }
    }

    impl From<Option<AddrDetails>> for lucet_module_addr_details {
        fn from(details: Option<AddrDetails>) -> Self {
            details.into()
        }
    }

    impl From<&Option<AddrDetails>> for lucet_module_addr_details {
        fn from(details: &Option<AddrDetails>) -> Self {
            details
                .as_ref()
                .map(|details| lucet_module_addr_details {
                    module_code_resolvable: true,
                    in_module_code: details.in_module_code,
                    file_name: details
                        .file_name
                        .as_ref()
                        .and_then(|s| {
                            CString::new(s.clone())
                                .ok()
                                .map(|s| s.into_raw() as *const _)
                        })
                        .unwrap_or(ptr::null()),
                    sym_name: details
                        .sym_name
                        .as_ref()
                        .and_then(|s| {
                            CString::new(s.clone())
                                .ok()
                                .map(|s| s.into_raw() as *const _)
                        })
                        .unwrap_or(ptr::null()),
                })
                .unwrap_or_default()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn lucet_instance_state(
    inst: *mut lucet_instance,
    state_out: *mut lucet_state::lucet_state,
) -> lucet_error {
    assert_nonnull!(state_out);
    with_ffi_boxes!([inst: InstanceHandle], {
        use lucet_runtime_internals::instance::InstanceInternal;
        state_out.write(inst.state().into());
        lucet_error::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn lucet_state_release(state: *mut lucet_state::lucet_state) {
    use self::lucet_state::*;
    use std::ffi::CString;

    let state = state.read();
    if let lucet_state_tag::Fault = state.tag {
        let addr_details = state.val.fault.rip_addr_details;
        // free the strings
        CString::from_raw(addr_details.file_name as *mut _);
        CString::from_raw(addr_details.sym_name as *mut _);
    }
}

mod lucet_val {
    #![allow(non_upper_case_globals)]

    use lucet_runtime_internals::val::{UntypedRetVal, UntypedRetValInternal, Val};

    include!(concat!(env!("OUT_DIR"), "/lucet_val.rs"));

    impl From<lucet_val> for Val {
        fn from(val: lucet_val) -> Val {
            val.into()
        }
    }

    impl From<&lucet_val> for Val {
        fn from(val: &lucet_val) -> Val {
            match val.type_ {
                lucet_val_type_lucet_val_c_ptr => Val::CPtr(unsafe { val.inner_val.as_u64 } as _),
                lucet_val_type_lucet_val_guest_ptr => {
                    Val::GuestPtr(unsafe { val.inner_val.as_u64 } as _)
                }
                lucet_val_type_lucet_val_u8 => Val::U8(unsafe { val.inner_val.as_u64 } as _),
                lucet_val_type_lucet_val_u16 => Val::U16(unsafe { val.inner_val.as_u64 } as _),
                lucet_val_type_lucet_val_u32 => Val::U32(unsafe { val.inner_val.as_u64 } as _),
                lucet_val_type_lucet_val_u64 => Val::U64(unsafe { val.inner_val.as_u64 } as _),
                lucet_val_type_lucet_val_i8 => Val::I16(unsafe { val.inner_val.as_i64 } as _),
                lucet_val_type_lucet_val_i16 => Val::I32(unsafe { val.inner_val.as_i64 } as _),
                lucet_val_type_lucet_val_i32 => Val::I32(unsafe { val.inner_val.as_i64 } as _),
                lucet_val_type_lucet_val_i64 => Val::I64(unsafe { val.inner_val.as_i64 } as _),
                lucet_val_type_lucet_val_usize => Val::USize(unsafe { val.inner_val.as_u64 } as _),
                lucet_val_type_lucet_val_isize => Val::ISize(unsafe { val.inner_val.as_i64 } as _),
                lucet_val_type_lucet_val_bool => Val::Bool(unsafe { val.inner_val.as_u64 } != 0),
                lucet_val_type_lucet_val_f32 => Val::F32(unsafe { val.inner_val.as_f32 } as _),
                lucet_val_type_lucet_val_f64 => Val::F64(unsafe { val.inner_val.as_f64 } as _),
                _ => panic!("Unsupported type"),
            }
        }
    }

    impl From<Val> for lucet_val {
        fn from(val: Val) -> Self {
            val.into()
        }
    }

    impl From<&Val> for lucet_val {
        fn from(val: &Val) -> Self {
            match val {
                Val::CPtr(a) => lucet_val {
                    type_: lucet_val_type_lucet_val_c_ptr,
                    inner_val: lucet_val_inner_val { as_u64: *a as _ },
                },
                Val::GuestPtr(a) => lucet_val {
                    type_: lucet_val_type_lucet_val_guest_ptr,
                    inner_val: lucet_val_inner_val { as_u64: *a as _ },
                },
                Val::U8(a) => lucet_val {
                    type_: lucet_val_type_lucet_val_u8,
                    inner_val: lucet_val_inner_val { as_u64: *a as _ },
                },
                Val::U16(a) => lucet_val {
                    type_: lucet_val_type_lucet_val_u16,
                    inner_val: lucet_val_inner_val { as_u64: *a as _ },
                },
                Val::U32(a) => lucet_val {
                    type_: lucet_val_type_lucet_val_u32,
                    inner_val: lucet_val_inner_val { as_u64: *a as _ },
                },
                Val::U64(a) => lucet_val {
                    type_: lucet_val_type_lucet_val_u64,
                    inner_val: lucet_val_inner_val { as_u64: *a as _ },
                },
                Val::I8(a) => lucet_val {
                    type_: lucet_val_type_lucet_val_i8,
                    inner_val: lucet_val_inner_val { as_i64: *a as _ },
                },
                Val::I16(a) => lucet_val {
                    type_: lucet_val_type_lucet_val_i16,
                    inner_val: lucet_val_inner_val { as_i64: *a as _ },
                },
                Val::I32(a) => lucet_val {
                    type_: lucet_val_type_lucet_val_i32,
                    inner_val: lucet_val_inner_val { as_i64: *a as _ },
                },
                Val::I64(a) => lucet_val {
                    type_: lucet_val_type_lucet_val_i64,
                    inner_val: lucet_val_inner_val { as_i64: *a as _ },
                },
                Val::USize(a) => lucet_val {
                    type_: lucet_val_type_lucet_val_usize,
                    inner_val: lucet_val_inner_val { as_u64: *a as _ },
                },
                Val::ISize(a) => lucet_val {
                    type_: lucet_val_type_lucet_val_isize,
                    inner_val: lucet_val_inner_val { as_i64: *a as _ },
                },
                Val::Bool(a) => lucet_val {
                    type_: lucet_val_type_lucet_val_bool,
                    inner_val: lucet_val_inner_val { as_u64: *a as _ },
                },
                Val::F32(a) => lucet_val {
                    type_: lucet_val_type_lucet_val_f32,
                    inner_val: lucet_val_inner_val { as_f32: *a as _ },
                },
                Val::F64(a) => lucet_val {
                    type_: lucet_val_type_lucet_val_f64,
                    inner_val: lucet_val_inner_val { as_f64: *a as _ },
                },
            }
        }
    }

    impl From<&UntypedRetVal> for lucet_untyped_retval {
        fn from(retval: &UntypedRetVal) -> lucet_untyped_retval {
            let mut v = lucet_untyped_retval {
                fp: [0; 16],
                gp: [0; 8],
            };
            unsafe {
                core::arch::x86_64::_mm_storeu_ps(
                    v.fp.as_mut().as_mut_ptr() as *mut f32,
                    retval.fp(),
                );
                *(v.gp.as_mut().as_mut_ptr() as *mut u64) = retval.gp();
            }
            v
        }
    }
}
