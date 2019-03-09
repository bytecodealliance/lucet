#![allow(non_camel_case_types)]

use crate::alloc::Limits;
use crate::error::Error;
use crate::instance::signals::SignalBehavior;
use crate::instance::{
    instance_handle_from_raw, instance_handle_to_raw, Instance, InstanceInternal,
};
use crate::module::{DlModule, Module};
use crate::region::mmap::MmapRegion;
use crate::region::Region;
use crate::trapcode::TrapCode;
use libc::{c_char, c_int, c_void};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use std::ffi::CStr;
use std::ptr;
use std::sync::Arc;

#[no_mangle]
pub static LUCET_WASM_PAGE_SIZE: u32 = crate::WASM_PAGE_SIZE;

#[macro_export]
macro_rules! assert_nonnull {
    ( $name:ident ) => {
        if $name.is_null() {
            return lucet_error::InvalidArgument;
        }
    };
}

#[macro_export]
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

#[repr(C)]
#[derive(Clone, Copy, Debug, FromPrimitive)]
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
        (&limits).into()
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
        (&limits).into()
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
    limits: *const lucet_alloc_limits,
    region_out: *mut *mut lucet_mmap_region,
) -> lucet_error {
    assert_nonnull!(region_out);
    let limits = limits
        .as_ref()
        .map(|l| l.into())
        .unwrap_or(Limits::default());
    match MmapRegion::create(instance_capacity as usize, &limits) {
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
            .new_instance(module.clone() as Arc<dyn Module>)
            .map(|mut i| {
                i.insert_embed_ctx(embed_ctx);
                inst_out.write(instance_handle_to_raw(i) as _);
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
    mod_out: *mut *mut lucet_dl_module,
) -> lucet_error {
    assert_nonnull!(mod_out);
    let path = CStr::from_ptr(path);
    DlModule::load(path.to_string_lossy().into_owned())
        .map(|m| {
            mod_out.write(Arc::into_raw(m) as _);
            lucet_error::Ok
        })
        .unwrap_or_else(|e| {
            // eprintln!("lucet_dl_module_load error: {}", e);
            e.into()
        })
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
            .unwrap_or_else(|e| e.into())
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

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub enum lucet_signal_behavior {
    Default,
    Continue,
    Terminate,
}

impl From<lucet_signal_behavior> for SignalBehavior {
    fn from(sb: lucet_signal_behavior) -> SignalBehavior {
        sb.into()
    }
}

impl From<&lucet_signal_behavior> for SignalBehavior {
    fn from(sb: &lucet_signal_behavior) -> SignalBehavior {
        match sb {
            lucet_signal_behavior::Default => SignalBehavior::Default,
            lucet_signal_behavior::Continue => SignalBehavior::Continue,
            lucet_signal_behavior::Terminate => SignalBehavior::Terminate,
        }
    }
}

type lucet_signal_handler = unsafe extern "C" fn(
    inst: *mut lucet_instance,
    trap: *const lucet_state::lucet_trapcode,
    signum: c_int,
    siginfo: *const libc::siginfo_t,
    context: *const c_void,
) -> lucet_signal_behavior;

/// Release or run* must not be called in the body of this function!
#[no_mangle]
pub unsafe extern "C" fn lucet_instance_set_signal_handler(
    inst: *mut lucet_instance,
    signal_handler: lucet_signal_handler,
) -> lucet_error {
    let handler = move |inst: &Instance, trap: &TrapCode, signum, siginfo, context| {
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

type lucet_fatal_handler = unsafe extern "C" fn(inst: *mut lucet_instance);

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

pub mod lucet_state {
    use crate::c_api::lucet_val;
    use crate::instance::{State, TerminationDetails};
    use crate::module::AddrDetails;
    use crate::trapcode::{TrapCode, TrapCodeType};
    use libc::{c_char, c_void};
    use num_derive::FromPrimitive;
    use num_traits::FromPrimitive;
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
                        terminated: match details {
                            TerminationDetails::Signal => lucet_terminated {
                                reason: lucet_terminated_reason::Signal,
                                other: std::ptr::null_mut(),
                            },
                            TerminationDetails::GetEmbedCtx => lucet_terminated {
                                reason: lucet_terminated_reason::GetEmbedCtx,
                                other: std::ptr::null_mut(),
                            },
                            TerminationDetails::Other(other) => lucet_terminated {
                                reason: lucet_terminated_reason::Other,
                                other: other
                                    .downcast_ref()
                                    .map(|v| *v)
                                    .unwrap_or(std::ptr::null_mut()),
                            },
                        },
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
    #[derive(Clone, Copy, Debug, FromPrimitive)]
    pub enum lucet_state_tag {
        Returned,
        Running,
        Fault,
        Terminated,
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

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub union lucet_state_val {
        pub returned: lucet_val::lucet_untyped_retval,
        // no meaning to this boolean, it's just there so the type is FFI-safe
        pub running: bool,
        pub fault: lucet_runtime_fault,
        pub terminated: lucet_terminated,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct lucet_terminated {
        pub reason: lucet_terminated_reason,
        pub other: *mut c_void,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub enum lucet_terminated_reason {
        Signal,
        GetEmbedCtx,
        Other,
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
            (&ty).into()
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
            (&trap).into()
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
            (&details).into()
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

pub mod lucet_val {
    use crate::val::{UntypedRetVal, UntypedRetValInternal, Val};
    use libc::{c_char, c_void};

    // Note on the value associated with each type: the most significant bits represent the "class"
    // of the type (1: a C pointer, 2: something unsigned that fits in 64 bits, 3: something signed
    // that fits in 64 bits, 4: f32, 5: f64). The remain bits can be anything as long as it is
    // unique.
    #[repr(C)]
    #[derive(Clone, Copy, Debug)]
    pub enum lucet_val_type {
        C_Ptr,    // = (1 << 16) | 0x0100,
        GuestPtr, // = (2 << 16) | 0x0101,
        U8,       // = (2 << 16) | 0x0201,
        U16,      // = (2 << 16) | 0x0202,
        U32,      // = (2 << 16) | 0x0203,
        U64,      // = (2 << 16) | 0x0204,
        I8,       // = (3 << 16) | 0x0300,
        I16,      // = (3 << 16) | 0x0301,
        I32,      // = (3 << 16) | 0x0302,
        I64,      // = (3 << 16) | 0x0303,
        USize,    // = (2 << 16) | 0x0400,
        ISize,    // = (3 << 16) | 0x0401,
        Bool,     // = (2 << 16) | 0x0700,
        F32,      // = (4 << 16) | 0x0800,
        F64,      // = (5 << 16) | 0x0801,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub union lucet_val_inner_val {
        as_c_ptr: *mut c_void, // (1 << 16)
        as_u64: u64,           // (2 << 16)
        as_i64: i64,           // (3 << 16)
        as_f32: f32,           // (4 << 16)
        as_f64: f64,           // (5 << 16)
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct lucet_val {
        ty: lucet_val_type,
        inner_val: lucet_val_inner_val,
    }

    impl From<lucet_val> for Val {
        fn from(val: lucet_val) -> Val {
            (&val).into()
        }
    }

    impl From<&lucet_val> for Val {
        fn from(val: &lucet_val) -> Val {
            match val.ty {
                lucet_val_type::C_Ptr => Val::CPtr(unsafe { val.inner_val.as_u64 } as _),
                lucet_val_type::GuestPtr => Val::GuestPtr(unsafe { val.inner_val.as_u64 } as _),
                lucet_val_type::U8 => Val::U8(unsafe { val.inner_val.as_u64 } as _),
                lucet_val_type::U16 => Val::U16(unsafe { val.inner_val.as_u64 } as _),
                lucet_val_type::U32 => Val::U32(unsafe { val.inner_val.as_u64 } as _),
                lucet_val_type::U64 => Val::U64(unsafe { val.inner_val.as_u64 } as _),
                lucet_val_type::I8 => Val::I16(unsafe { val.inner_val.as_i64 } as _),
                lucet_val_type::I16 => Val::I32(unsafe { val.inner_val.as_i64 } as _),
                lucet_val_type::I32 => Val::I32(unsafe { val.inner_val.as_i64 } as _),
                lucet_val_type::I64 => Val::I64(unsafe { val.inner_val.as_i64 } as _),
                lucet_val_type::USize => Val::USize(unsafe { val.inner_val.as_u64 } as _),
                lucet_val_type::ISize => Val::ISize(unsafe { val.inner_val.as_i64 } as _),
                lucet_val_type::Bool => Val::Bool(unsafe { val.inner_val.as_u64 } != 0),
                lucet_val_type::F32 => Val::F32(unsafe { val.inner_val.as_f32 } as _),
                lucet_val_type::F64 => Val::F64(unsafe { val.inner_val.as_f64 } as _),
            }
        }
    }

    impl From<Val> for lucet_val {
        fn from(val: Val) -> Self {
            (&val).into()
        }
    }

    impl From<&Val> for lucet_val {
        fn from(val: &Val) -> Self {
            match val {
                Val::CPtr(a) => lucet_val {
                    ty: lucet_val_type::C_Ptr,
                    inner_val: lucet_val_inner_val { as_u64: *a as _ },
                },
                Val::GuestPtr(a) => lucet_val {
                    ty: lucet_val_type::GuestPtr,
                    inner_val: lucet_val_inner_val { as_u64: *a as _ },
                },
                Val::U8(a) => lucet_val {
                    ty: lucet_val_type::U8,
                    inner_val: lucet_val_inner_val { as_u64: *a as _ },
                },
                Val::U16(a) => lucet_val {
                    ty: lucet_val_type::U16,
                    inner_val: lucet_val_inner_val { as_u64: *a as _ },
                },
                Val::U32(a) => lucet_val {
                    ty: lucet_val_type::U32,
                    inner_val: lucet_val_inner_val { as_u64: *a as _ },
                },
                Val::U64(a) => lucet_val {
                    ty: lucet_val_type::U64,
                    inner_val: lucet_val_inner_val { as_u64: *a as _ },
                },
                Val::I8(a) => lucet_val {
                    ty: lucet_val_type::I8,
                    inner_val: lucet_val_inner_val { as_i64: *a as _ },
                },
                Val::I16(a) => lucet_val {
                    ty: lucet_val_type::I16,
                    inner_val: lucet_val_inner_val { as_i64: *a as _ },
                },
                Val::I32(a) => lucet_val {
                    ty: lucet_val_type::I32,
                    inner_val: lucet_val_inner_val { as_i64: *a as _ },
                },
                Val::I64(a) => lucet_val {
                    ty: lucet_val_type::I64,
                    inner_val: lucet_val_inner_val { as_i64: *a as _ },
                },
                Val::USize(a) => lucet_val {
                    ty: lucet_val_type::USize,
                    inner_val: lucet_val_inner_val { as_u64: *a as _ },
                },
                Val::ISize(a) => lucet_val {
                    ty: lucet_val_type::ISize,
                    inner_val: lucet_val_inner_val { as_i64: *a as _ },
                },
                Val::Bool(a) => lucet_val {
                    ty: lucet_val_type::Bool,
                    inner_val: lucet_val_inner_val { as_u64: *a as _ },
                },
                Val::F32(a) => lucet_val {
                    ty: lucet_val_type::F32,
                    inner_val: lucet_val_inner_val { as_f32: *a as _ },
                },
                Val::F64(a) => lucet_val {
                    ty: lucet_val_type::F64,
                    inner_val: lucet_val_inner_val { as_f64: *a as _ },
                },
            }
        }
    }

    #[repr(C)]
    #[derive(Clone, Copy, Debug)]
    pub struct lucet_untyped_retval {
        fp: [c_char; 16],
        gp: [c_char; 8],
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub union lucet_retval_gp {
        as_untyped: [c_char; 8],
        as_c_ptr: *mut c_void,
        as_u64: u64,
        as_i64: i64,
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

    #[no_mangle]
    pub unsafe extern "C" fn lucet_retval_gp(
        retval: *const lucet_untyped_retval,
    ) -> lucet_retval_gp {
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
}
