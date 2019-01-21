use crate::errors::{LucetError, TerminationDetails};
use crate::module::Module;
use crate::pool::Pool;
use crate::state::State;
use crate::val::{UntypedRetval, Val};
use libc::c_int;
use lucet_sys::*;
use std::ffi::CString;
use std::os::raw::c_void;
use std::ptr;
use std::u32;
use xfailure::xbail;

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub enum RunStatus {
    Ok = lucet_run_stat_lucet_run_ok as _,
    SymbolNotFound = lucet_run_stat_lucet_run_symbol_not_found as _,
}

#[allow(non_upper_case_globals)]
impl From<u32> for RunStatus {
    fn from(v: u32) -> RunStatus {
        match v {
            lucet_run_stat_lucet_run_ok => RunStatus::Ok,
            lucet_run_stat_lucet_run_symbol_not_found => RunStatus::SymbolNotFound,
            _ => panic!("Unexpected status"),
        }
    }
}

pub struct Instance {
    lucet_instance: *mut lucet_instance,
}

impl Instance {
    pub fn new(pool: &Pool, module: &Module) -> Result<Instance, LucetError> {
        unsafe { Instance::new_with_delegate(pool, module, ptr::null_mut()) }
    }

    pub unsafe fn new_with_delegate(
        pool: &Pool,
        module: &Module,
        delegate: *mut c_void,
    ) -> Result<Instance, LucetError> {
        let lucet_instance = lucet_instance_create(pool.lucet_pool, module.lucet_module, delegate);
        if lucet_instance.is_null() {
            xbail!(LucetError::RuntimeError("Unable to create a new instance"));
        }
        let instance = Instance { lucet_instance };
        Ok(instance)
    }

    pub fn run_start(&mut self) -> Result<(), LucetError> {
        let lucet_run_stat = unsafe { lucet_instance_run_start(self.lucet_instance) };
        let run_status: RunStatus = lucet_run_stat.into();
        match run_status {
            RunStatus::SymbolNotFound => Err(LucetError::SymbolNotFound("guest_start".to_owned()))?,
            RunStatus::Ok => {}
        }
        match self.state() {
            State::Ready { .. } => Ok(()),
            State::Running => panic!(
                "should be impossible to be running after call to lucet_instance_run_start returns"
            ),
            State::Fault { details } => {
                self.reset();
                Err(LucetError::RuntimeFault(details))
            }
            State::Terminated { details } => {
                self.reset();
                Err(LucetError::RuntimeTerminated(TerminationDetails {
                    details,
                }))
            }
        }
    }

    pub fn run(&mut self, entrypoint: &str, args: &[Val]) -> Result<UntypedRetval, LucetError> {
        let entrypoint_c = CString::new(entrypoint)?;
        let args: Vec<_> = args.into_iter().map(|&a| lucet_val::from(a)).collect();
        let argc = args.len();
        let lucet_run_stat = unsafe {
            match argc {
                0 => lucet_instance_run(self.lucet_instance, entrypoint_c.as_ptr(), argc as c_int),
                1 => lucet_instance_run(
                    self.lucet_instance,
                    entrypoint_c.as_ptr(),
                    argc as c_int,
                    args[0],
                ),
                2 => lucet_instance_run(
                    self.lucet_instance,
                    entrypoint_c.as_ptr(),
                    argc as c_int,
                    args[0],
                    args[1],
                ),
                3 => lucet_instance_run(
                    self.lucet_instance,
                    entrypoint_c.as_ptr(),
                    argc as c_int,
                    args[0],
                    args[1],
                    args[2],
                ),
                4 => lucet_instance_run(
                    self.lucet_instance,
                    entrypoint_c.as_ptr(),
                    argc as c_int,
                    args[0],
                    args[1],
                    args[2],
                    args[3],
                ),
                5 => lucet_instance_run(
                    self.lucet_instance,
                    entrypoint_c.as_ptr(),
                    argc as c_int,
                    args[0],
                    args[1],
                    args[2],
                    args[3],
                    args[4],
                ),
                6 => lucet_instance_run(
                    self.lucet_instance,
                    entrypoint_c.as_ptr(),
                    argc as c_int,
                    args[0],
                    args[1],
                    args[2],
                    args[3],
                    args[4],
                    args[5],
                ),
                7 => lucet_instance_run(
                    self.lucet_instance,
                    entrypoint_c.as_ptr(),
                    argc as c_int,
                    args[0],
                    args[1],
                    args[2],
                    args[3],
                    args[4],
                    args[5],
                    args[6],
                ),
                8 => lucet_instance_run(
                    self.lucet_instance,
                    entrypoint_c.as_ptr(),
                    argc as c_int,
                    args[0],
                    args[1],
                    args[2],
                    args[3],
                    args[4],
                    args[5],
                    args[6],
                    args[7],
                ),
                _ => xbail!(LucetError::Unsupported),
            }
        };
        let run_status: RunStatus = lucet_run_stat.into();
        match run_status {
            RunStatus::SymbolNotFound => Err(LucetError::SymbolNotFound(entrypoint.to_owned()))?,
            RunStatus::Ok => {}
        }
        match self.state() {
            State::Ready { untyped_retval } => Ok(untyped_retval),
            State::Running => panic!(
                "should be impossible to be running after call to lucet_instance_run returns"
            ),
            State::Fault { details } => {
                self.reset();
                Err(LucetError::RuntimeFault(details))
            }
            State::Terminated { details } => {
                self.reset();
                Err(LucetError::RuntimeTerminated(TerminationDetails {
                    details,
                }))
            }
        }
    }

    pub fn run_func_id(
        &mut self,
        table_id: u32,
        func_id: u32,
        args: &[Val],
    ) -> Result<UntypedRetval, LucetError> {
        let args: Vec<_> = args.into_iter().map(|&a| lucet_val::from(a)).collect();
        let argc = args.len();
        let lucet_run_stat = unsafe {
            match argc {
                0 => lucet_instance_run_func_id(
                    self.lucet_instance,
                    table_id,
                    func_id,
                    argc as c_int,
                ),
                1 => lucet_instance_run_func_id(
                    self.lucet_instance,
                    table_id,
                    func_id,
                    argc as c_int,
                    args[0],
                ),
                2 => lucet_instance_run_func_id(
                    self.lucet_instance,
                    table_id,
                    func_id,
                    argc as c_int,
                    args[0],
                    args[1],
                ),
                3 => lucet_instance_run_func_id(
                    self.lucet_instance,
                    table_id,
                    func_id,
                    argc as c_int,
                    args[0],
                    args[1],
                    args[2],
                ),
                4 => lucet_instance_run_func_id(
                    self.lucet_instance,
                    table_id,
                    func_id,
                    argc as c_int,
                    args[0],
                    args[1],
                    args[2],
                    args[3],
                ),
                5 => lucet_instance_run_func_id(
                    self.lucet_instance,
                    table_id,
                    func_id,
                    argc as c_int,
                    args[0],
                    args[1],
                    args[2],
                    args[3],
                    args[4],
                ),
                6 => lucet_instance_run_func_id(
                    self.lucet_instance,
                    table_id,
                    func_id,
                    argc as c_int,
                    args[0],
                    args[1],
                    args[2],
                    args[3],
                    args[4],
                    args[5],
                ),
                7 => lucet_instance_run_func_id(
                    self.lucet_instance,
                    table_id,
                    func_id,
                    argc as c_int,
                    args[0],
                    args[1],
                    args[2],
                    args[3],
                    args[4],
                    args[5],
                    args[6],
                ),
                8 => lucet_instance_run_func_id(
                    self.lucet_instance,
                    table_id,
                    func_id,
                    argc as c_int,
                    args[0],
                    args[1],
                    args[2],
                    args[3],
                    args[4],
                    args[5],
                    args[6],
                    args[7],
                ),
                _ => xbail!(LucetError::Unsupported),
            }
        };
        let run_status: RunStatus = lucet_run_stat.into();
        match run_status {
            RunStatus::SymbolNotFound => Err(LucetError::FuncNotFound(table_id, func_id))?,
            RunStatus::Ok => {}
        }
        match self.state() {
            State::Ready { untyped_retval } => Ok(untyped_retval),
            State::Running => panic!(
                "should be impossible to be running after call to lucet_instance_run returns"
            ),
            State::Fault { details } => {
                self.reset();
                Err(LucetError::RuntimeFault(details))
            }
            State::Terminated { details } => {
                self.reset();
                Err(LucetError::RuntimeTerminated(TerminationDetails {
                    details,
                }))
            }
        }
    }

    pub fn reset(&mut self) {
        unsafe { lucet_instance_reset(self.lucet_instance) }
    }

    pub fn state(&self) -> State {
        unsafe { (*lucet_instance_get_state(self.lucet_instance)).into() }
    }

    pub fn grow_memory(&self, additional_size: usize) -> Result<usize, LucetError> {
        if additional_size > u32::MAX as usize {
            xbail!(LucetError::UsageError("Unsupported size"))
        }
        let new_pages_count =
            unsafe { lucet_instance_grow_memory(self.lucet_instance, additional_size as u32) };
        if new_pages_count < 0 {
            Err(LucetError::RuntimeError("Unable to allocate pages"))
        } else {
            Ok(new_pages_count as usize)
        }
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            lucet_instance_release(self.lucet_instance);
            self.lucet_instance = ptr::null_mut();
        };
    }
}
