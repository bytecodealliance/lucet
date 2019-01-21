use crate::val::UntypedRetval;
use lucet_sys::*;
use std::ffi::CStr;
use std::os::raw::c_void;

#[derive(Clone, Debug)]
pub enum State {
    Ready { untyped_retval: UntypedRetval },
    Running,
    Fault { details: FaultDetails },
    Terminated { details: *mut c_void },
}

impl From<lucet_state> for State {
    fn from(s: lucet_state) -> State {
        let tag = s.tag;
        #[allow(non_upper_case_globals)]
        match tag {
            lucet_state_tag_lucet_state_ready => {
                let untyped_retval = unsafe { s.u.ready.untyped_retval }.into();
                State::Ready { untyped_retval }
            }
            lucet_state_tag_lucet_state_running => State::Running,
            lucet_state_tag_lucet_state_fault => {
                let fatal = unsafe { s.u.fault.fatal };
                let trapcode = unsafe { s.u.fault.trapcode }.into();
                let rip_addr = unsafe { s.u.fault.rip_addr };
                let rip_details = unsafe { s.u.fault.rip_addr_details }.into();
                let details = FaultDetails {
                    fatal,
                    trapcode,
                    rip_addr,
                    rip_details,
                };
                State::Fault { details }
            }
            lucet_state_tag_lucet_state_terminated => State::Terminated {
                details: unsafe { s.u.terminated.info },
            },
            _ => panic!("lucet_state tag out of range: {}", tag),
        }
    }
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct FaultDetails {
    pub fatal: bool,
    pub trapcode: Trapcode,
    pub rip_addr: usize,
    pub rip_details: AddrDetails,
}

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub enum TrapcodeType {
    StackOverflow,
    HeapOutOfBounds,
    OutOfBounds,
    IndirectCallToNull,
    BadSignature,
    IntegerOverflow,
    IntegerDivByZero,
    BadConversionToInteger,
    Interrupt,
    User,
    Unknown,
}

impl From<lucet_trapcode_type> for TrapcodeType {
    fn from(t: lucet_trapcode_type) -> TrapcodeType {
        #[allow(non_upper_case_globals)]
        match t {
            lucet_trapcode_type_lucet_trapcode_stack_overflow => TrapcodeType::StackOverflow,
            lucet_trapcode_type_lucet_trapcode_heap_oob => TrapcodeType::HeapOutOfBounds,
            lucet_trapcode_type_lucet_trapcode_oob => TrapcodeType::OutOfBounds,
            lucet_trapcode_type_lucet_trapcode_indirect_call_to_null => {
                TrapcodeType::IndirectCallToNull
            }
            lucet_trapcode_type_lucet_trapcode_bad_signature => TrapcodeType::BadSignature,
            lucet_trapcode_type_lucet_trapcode_integer_overflow => TrapcodeType::IntegerOverflow,
            lucet_trapcode_type_lucet_trapcode_integer_div_by_zero => {
                TrapcodeType::IntegerDivByZero
            }
            lucet_trapcode_type_lucet_trapcode_bad_conversion_to_integer => {
                TrapcodeType::BadConversionToInteger
            }
            lucet_trapcode_type_lucet_trapcode_interrupt => TrapcodeType::Interrupt,
            lucet_trapcode_type_lucet_trapcode_user => TrapcodeType::User,
            lucet_trapcode_type_lucet_trapcode_unknown => TrapcodeType::Unknown,
            _ => TrapcodeType::Unknown,
        }
    }
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct Trapcode {
    pub type_: TrapcodeType,
    pub tag: u16,
}

impl From<lucet_trapcode> for Trapcode {
    fn from(t: lucet_trapcode) -> Trapcode {
        let type_ = t.code.into();
        let tag = t.tag;
        Trapcode { type_, tag }
    }
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct AddrDetails {
    module_code_resolvable: bool,
    in_module_code: bool,
    file_name: Option<String>,
    sym_name: Option<String>,
}

impl From<lucet_module_addr_details> for AddrDetails {
    fn from(d: lucet_module_addr_details) -> AddrDetails {
        let module_code_resolvable = d.module_code_resolvable;
        let in_module_code = d.in_module_code;
        let file_name = if d.file_name.is_null() {
            None
        } else {
            let cstr = unsafe { CStr::from_ptr(d.file_name) };
            match cstr.to_owned().into_string() {
                Ok(s) => Some(s),
                Err(_) => None,
            }
        };
        let sym_name = if d.sym_name.is_null() {
            None
        } else {
            let cstr = unsafe { CStr::from_ptr(d.sym_name) };
            match cstr.to_owned().into_string() {
                Ok(s) => Some(s),
                Err(_) => None,
            }
        };
        AddrDetails {
            module_code_resolvable,
            in_module_code,
            file_name,
            sym_name,
        }
    }
}
