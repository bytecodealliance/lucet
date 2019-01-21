use lucet_libc_sys::*;
use std::ffi::CStr;

#[repr(C)]
pub struct LucetLibc {
    libc: lucet_libc,
}

/// the stdio_handler is not supported right now, because nothing will burn to the ground without
/// it. support will have to be added at some point.
impl LucetLibc {
    pub fn new() -> LucetLibc {
        let mut libc = lucet_libc {
            magic: 0,
            term_info: lucet_libc__bindgen_ty_1 { exit: 0 },
            term_reason: lucet_libc_term_reason_lucet_libc_term_none,
            stdio_handler: None,
        };

        unsafe { lucet_libc_init(&mut libc as *mut lucet_libc) };

        LucetLibc { libc }
    }

    pub fn termination_reason(&self) -> Option<TerminationReason> {
        #![allow(non_upper_case_globals)]
        match self.libc.term_reason {
            lucet_libc_term_reason_lucet_libc_term_none => None,
            lucet_libc_term_reason_lucet_libc_term_exit => {
                let exit_code = unsafe { self.libc.term_info.exit };
                Some(TerminationReason::Exit(exit_code))
            }
            lucet_libc_term_reason_lucet_libc_term_abort => Some(TerminationReason::Abort),
            lucet_libc_term_reason_lucet_libc_term_check_heap => {
                let cstr = unsafe { CStr::from_ptr(self.libc.term_info.check_heap) };
                let rstr = cstr.to_str().unwrap_or("(invalid!)");
                Some(TerminationReason::InvalidAddress(rstr.to_owned()))
            }
            _ => panic!("invalid lucet_libc.term_reason"),
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum TerminationReason {
    Exit(i32),
    Abort,
    InvalidAddress(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn create() {
        let libc = LucetLibc::new();
        assert_eq!(libc.termination_reason(), None);
    }
}
