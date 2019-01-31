mod siginfo_ext;
pub mod signals;

pub use crate::instance::signals::{signal_handler_none, SignalBehavior};

use crate::alloc::Alloc;
use crate::context::Context;
use crate::instance::siginfo_ext::SiginfoExt;
use crate::module::{self, Module};
use crate::trapcode::{TrapCode, TrapCodeType};
use crate::val::{UntypedRetVal, Val};
use failure::{bail, ensure, format_err, Error, ResultExt};
use libc::{c_void, siginfo_t, uintptr_t, SIGBUS, SIGSEGV};
use std::cell::{RefCell, UnsafeCell};
use std::ffi::{CStr, CString};
use std::mem;
use std::ops::{Deref, DerefMut};
use std::ptr::{self, NonNull};

pub const LUCET_INSTANCE_MAGIC: u64 = 746932922;
pub const INSTANCE_PADDING: usize = 2296;

pub const WASM_PAGE_SIZE: u32 = 64 * 1024;

thread_local! {
    /// The host context.
    ///
    /// Control returns here implicitly due to the setup in `Context::init()` when guest functions
    /// return normally. Control can return here explicitly from signal handlers when the guest
    /// program needs to be terminated.
    ///
    /// This is an `UnsafeCell` due to nested borrows. The context must be borrowed mutably when
    /// swapping to the guest context, which means that borrow exists for the entire time the guest
    /// function runs even though the mutation to the host context is done only at the beginning of
    /// the swap. Meanwhile, the signal handler can run at any point during the guest function, and
    /// so it also must be able to immutably borrow the host context if it needs to swap back. The
    /// runtime borrowing constraints for a `RefCell` are therefore too strict for this variable.
    pub(crate) static HOST_CTX: UnsafeCell<Context> = UnsafeCell::new(Context::new());

    /// The currently-running `Instance`, if one exists.
    pub(crate) static CURRENT_INSTANCE: RefCell<Option<NonNull<Instance>>> = RefCell::new(None);
}

/// A smart pointer to an [`Instance`](struct.Instance.html) that properly manages cleanup when dropped.
///
/// Instances are always stored in memory backed by a `Region`; we never want to create one directly
/// with the Rust allocator. This type allows us to abide by that rule while also having an owned
/// type that cleans up the instance when we are done with it.
///
/// Since this type implements `Deref` and `DerefMut` to `Instance`, it can usually be treated as
/// though it were a `&mut Instance`.
pub struct InstanceHandle {
    inst: NonNull<Instance>,
}

/// Create a new `InstanceHandle`.
///
/// This is not meant for public consumption, but rather is used to make implementations of
/// `Region`.
pub fn new_instance_handle(
    instance: *mut Instance,
    module: Box<dyn Module>,
    alloc: Alloc,
    embed_ctx: *mut c_void,
) -> Result<InstanceHandle, Error> {
    let inst = NonNull::new(instance).ok_or(format_err!("instance pointer is null"))?;

    // do this check first so we don't run `InstanceHandle::drop()` for a failure
    ensure!(
        unsafe { inst.as_ref().magic } != LUCET_INSTANCE_MAGIC,
        "created a new instance handle in memory with existing instance magic"
    );

    let mut handle = InstanceHandle { inst };

    let inst = Instance::new(alloc, module, embed_ctx);

    unsafe {
        // this is wildly unsafe! you must be very careful to not let the drop impls run on the
        // uninitialized fields; see
        // <https://doc.rust-lang.org/std/mem/fn.forget.html#use-case-1>

        // write the whole struct into place over the uninitialized page
        ptr::write(&mut *handle, inst);
    };

    handle.reset()?;

    Ok(handle)
}

// Safety argument for these deref impls: the instance's `Alloc` field contains an `Arc` to the
// region that backs this memory, keeping the page containing the `Instance` alive as long as the
// region exists

impl Deref for InstanceHandle {
    type Target = Instance;
    fn deref(&self) -> &Self::Target {
        unsafe { self.inst.as_ref() }
    }
}

impl DerefMut for InstanceHandle {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.inst.as_mut() }
    }
}

impl Drop for InstanceHandle {
    fn drop(&mut self) {
        // eprintln!("InstanceHandle::drop()");
        // zero out magic, then run the destructor by taking and dropping the inner `Instance`
        self.magic = 0;
        unsafe {
            mem::replace(self.inst.as_mut(), mem::uninitialized());
        }
    }
}

#[repr(C)]
pub struct Instance {
    /// Used to catch bugs in pointer math used to find the address of the instance
    magic: u64,

    /// The embedding context is a pointer from the embedder that is used to implement hostcalls
    pub(crate) embed_ctx: *mut c_void,

    /// The program (WebAssembly module) that is the entrypoint for the instance.
    module: Box<dyn Module>,

    /// The `Context` in which the guest program runs
    ctx: Context,

    /// Instance state and error information
    pub state: State,

    /// The memory allocated for this instance
    alloc: Alloc,

    /// Handler for when the instance exits in a fatal state
    pub fatal_handler: fn(&Instance) -> !,

    /// Signal handler used to interpret signals that aren't otherwise handled by the WebAssembly trap table
    pub signal_handler: fn(
        &Instance,
        &TrapCode,
        signum: libc::c_int,
        siginfo_ptr: *const siginfo_t,
        ucontext_ptr: *const c_void,
    ) -> SignalBehavior,

    /// Pointer to the function used as the entrypoint (for use in backtraces)
    entrypoint: *const extern "C" fn(),

    /// Padding to ensure the pointer to globals at the end of the page occupied by the `Instance`
    _reserved: [u8; INSTANCE_PADDING],

    /// Pointer to the globals
    ///
    /// This is accessed through the `vmctx` pointer, which points to the heap that begins
    /// immediately after this struct, so it has to come at the very end.
    globals_ptr: *const i64,
}

/// APIs that are internal, but useful to implementors of extension modules; you probably don't want
/// this trait!
///
/// This is a trait rather than inherent `impl`s in order to keep the `lucet-runtime` API clean and
/// safe.
pub trait InstanceInternal {
    fn alloc(&self) -> &Alloc;
    fn alloc_mut(&mut self) -> &mut Alloc;
    fn module(&self) -> &dyn Module;
    fn valid_magic(&self) -> bool;
}

impl InstanceInternal for Instance {
    /// Get a reference to the instance's `Alloc`.
    fn alloc(&self) -> &Alloc {
        &self.alloc
    }

    /// Get a mutable reference to the instance's `Alloc`.
    fn alloc_mut(&mut self) -> &mut Alloc {
        &mut self.alloc
    }

    /// Get a reference to the instance's `Module`.
    fn module(&self) -> &dyn Module {
        self.module.deref()
    }

    /// Check whether the instance magic is valid.
    fn valid_magic(&self) -> bool {
        self.magic == LUCET_INSTANCE_MAGIC
    }
}

// Public API
impl Instance {
    // TODO: richer error types for this whole family of functions
    pub fn run(&mut self, entrypoint: &[u8], args: &[Val]) -> Result<&State, Error> {
        let func = self.module.get_export_func(entrypoint)?;
        self.run_func(func, &args)
    }

    pub fn reset(&mut self) -> Result<(), Error> {
        self.alloc.reset_heap(self.module.as_ref())?;
        let globals = unsafe { self.alloc.globals_mut() };
        let mod_globals = self.module.globals();
        for (i, v) in mod_globals.iter().enumerate() {
            globals[i] = *v;
        }

        self.state = State::Ready {
            retval: UntypedRetVal::default(),
        };

        self.run_start()?;

        Ok(())
    }

    /// Grow the guest memory by the given number of WebAssembly pages.
    ///
    /// On success, returns the number of pages that existed before the call.
    pub fn grow_memory(&mut self, additional_pages: u32) -> Result<u32, Error> {
        let orig_len = self.alloc.expand_heap(additional_pages * WASM_PAGE_SIZE)?;
        Ok(orig_len / WASM_PAGE_SIZE)
    }

    pub fn heap(&self) -> &[u8] {
        unsafe { self.alloc.heap() }
    }

    pub fn heap_mut(&mut self) -> &mut [u8] {
        unsafe { self.alloc.heap_mut() }
    }

    pub fn heap_u32(&self) -> &[u32] {
        unsafe { self.alloc.heap_u32() }
    }

    pub fn heap_u32_mut(&mut self) -> &mut [u32] {
        unsafe { self.alloc.heap_u32_mut() }
    }

    /// Check if a memory region is inside the instance heap.
    pub fn check_heap(&self, ptr: *const c_void, len: usize) -> bool {
        self.alloc.mem_in_heap(ptr, len)
    }

    // TODO: replace this with a richer `run` interface?
    pub fn is_ready(&self) -> bool {
        self.state.is_ready()
    }

    pub fn is_terminated(&self) -> bool {
        self.state.is_terminated()
    }
}

impl Instance {
    fn new(alloc: Alloc, module: Box<dyn Module>, embed_ctx: *mut c_void) -> Self {
        let globals_ptr = alloc.slot().globals as *mut i64;
        Instance {
            magic: LUCET_INSTANCE_MAGIC,
            embed_ctx,
            module,
            ctx: Context::new(),
            state: State::Ready {
                retval: UntypedRetVal::default(),
            },
            alloc,
            fatal_handler: default_fatal_handler,
            signal_handler: signal_handler_none,
            entrypoint: ptr::null(),
            _reserved: [0; INSTANCE_PADDING],
            globals_ptr,
        }
    }

    fn run_func(&mut self, func: *const extern "C" fn(), args: &[Val]) -> Result<&State, Error> {
        ensure!(!func.is_null(), "func cannot be null");
        self.entrypoint = func;

        let mut args_with_vmctx = vec![Val::from(self.alloc.slot().heap)];
        args_with_vmctx.extend_from_slice(args);

        HOST_CTX.with(|host_ctx| {
            Context::init(
                unsafe { self.alloc.stack_u64_mut() },
                unsafe { &mut *host_ctx.get() },
                &mut self.ctx,
                func,
                &args_with_vmctx,
            )
        })?;

        self.state = State::Running;

        // there should never be another instance running on this thread when we enter this function
        CURRENT_INSTANCE.with(|current_instance| {
            let mut current_instance = current_instance.borrow_mut();
            assert!(
                current_instance.is_none(),
                "no other instance is running on this thread"
            );
            // safety: `self` is not null if we are in this function
            *current_instance = Some(unsafe { NonNull::new_unchecked(self) });
        });

        self.with_signals_on(|i| {
            HOST_CTX.with(|host_ctx| {
                // Save the current context into `host_ctx`, and jump to the guest context. The
                // lucet context is linked to host_ctx, so it will return here after it finishes,
                // successfully or otherwise.
                unsafe { Context::swap(&mut *host_ctx.get(), &mut i.ctx) };
                Ok(())
            })
        })?;

        CURRENT_INSTANCE.with(|current_instance| {
            *current_instance.borrow_mut() = None;
        });

        // Sandbox has jumped back to the host process, indicating it has either:
        //
        // * trapped, or called hostcall_error: state tag changed to something other than `Running`
        // * function body returned: set state back to `Ready` with return value
        if self.state.is_running() {
            let retval = self.ctx.get_untyped_retval();
            self.state = State::Ready { retval };
        }

        // Sandbox is no longer runnable. It's unsafe to determine all error details in the signal
        // handler, so we fill in extra details here.
        if self.state.is_fault() {
            self.state = self.populate_fault_detail()?;
        }

        // Some errors indicate that the guest is not functioning correctly or that the loaded code
        // violated some assumption, so bail out via the fatal handler.
        if self.state.is_fatal() {
            (self.fatal_handler)(self);
        }

        Ok(&self.state)
    }

    fn run_start(&mut self) -> Result<(), Error> {
        if let Some(start) = self.module.get_start_func()? {
            self.run_func(start, &[]).context("module start")?;
            if !self.is_ready() {
                bail!("unexpected state after module start: {}", self.state);
            }
        }
        Ok(())
    }

    fn populate_fault_detail(&mut self) -> Result<State, Error> {
        match &self.state {
            State::Fault {
                rip_addr,
                trapcode,
                siginfo,
                context,
                ..
            } => {
                // We do this after returning from the signal handler because it requires `dladdr`
                // calls, which are not signal safe
                let rip_addr_details = self.module.addr_details(*rip_addr as *const c_void)?;

                // If the trap table lookup returned unknown, it is a fatal error
                let unknown_fault = trapcode.ty == TrapCodeType::Unknown;

                // If the trap was a segv or bus fault and the addressed memory was outside the
                // guard pages, it is also a fatal error
                let outside_guard = (siginfo.si_signo == SIGSEGV || siginfo.si_signo == SIGBUS)
                    && !self.alloc.addr_in_heap_guard(siginfo.si_addr());

                Ok(State::Fault {
                    fatal: unknown_fault || outside_guard,
                    trapcode: *trapcode,
                    rip_addr: *rip_addr,
                    rip_addr_details,
                    siginfo: *siginfo,
                    context: *context,
                })
            }
            st => Ok(st.clone()),
        }
    }
}

#[derive(Clone)]
pub enum State {
    Ready {
        retval: UntypedRetVal,
    },
    Running,
    Fault {
        fatal: bool,
        trapcode: TrapCode,
        rip_addr: uintptr_t,
        rip_addr_details: Option<module::AddrDetails>,
        siginfo: libc::siginfo_t,
        context: libc::ucontext_t,
    },
    Terminated {
        info: *mut c_void,
    },
}

impl std::fmt::Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            State::Ready { .. } => write!(f, "ready"),
            State::Running => write!(f, "running"),
            State::Fault {
                fatal,
                rip_addr,
                rip_addr_details,
                siginfo,
                trapcode,
                ..
            } => {
                // TODO: finish implementing this
                if *fatal {
                    write!(f, "fault FATAL ")?;
                } else {
                    write!(f, "fault ")?;
                }

                trapcode.fmt(f)?;

                write!(
                    f,
                    " triggered by {}: ",
                    strsignal_wrapper(siginfo.si_signo)
                        .into_string()
                        .expect("strsignal returns valid UTF-8")
                )?;

                write!(f, "code at address {:p}", *rip_addr as *const c_void)?;

                if let Some(addr_details) = rip_addr_details {
                    if let Some(ref fname) = addr_details.file_name {
                        let sname = addr_details
                            .sym_name
                            .as_ref()
                            .map(String::as_str)
                            .unwrap_or("<unknown>");
                        write!(f, " (symbol {}:{})", fname, sname)?;
                    }
                    if addr_details.in_module_code {
                        write!(f, " (inside module code)")?;
                    } else {
                        write!(f, " (not inside module code)")?;
                    }
                } else {
                    write!(f, " (unknown whether in module)")?;
                }

                if siginfo.si_signo == SIGSEGV || siginfo.si_signo == SIGBUS {
                    // We know this is inside the heap guard, because by the time we get here,
                    // `lucet_error_verify_trap_safety` will have run and validated it.
                    write!(
                        f,
                        " accessed memory at {:p} (inside heap guard)",
                        siginfo.si_addr()
                    )?;
                }
                Ok(())
            }
            State::Terminated { .. } => write!(f, "terminated"),
        }
    }
}

impl State {
    pub fn is_ready(&self) -> bool {
        if let State::Ready { .. } = self {
            true
        } else {
            false
        }
    }

    pub fn is_running(&self) -> bool {
        if let State::Running = self {
            true
        } else {
            false
        }
    }

    pub fn is_fault(&self) -> bool {
        if let State::Fault { .. } = self {
            true
        } else {
            false
        }
    }

    pub fn is_fatal(&self) -> bool {
        if let State::Fault { fatal, .. } = self {
            *fatal
        } else {
            false
        }
    }

    pub fn is_terminated(&self) -> bool {
        if let State::Terminated { .. } = self {
            true
        } else {
            false
        }
    }
}

fn default_fatal_handler(inst: &Instance) -> ! {
    panic!("> instance {:p} had fatal error: {}", inst, inst.state);
}

// TODO: PR into `libc`
extern "C" {
    #[no_mangle]
    fn strsignal(sig: libc::c_int) -> *mut libc::c_char;
}

// TODO: PR into `nix`
fn strsignal_wrapper(sig: libc::c_int) -> CString {
    unsafe { CStr::from_ptr(strsignal(sig)).to_owned() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use memoffset::offset_of;

    #[test]
    fn instance_size_correct() {
        assert_eq!(mem::size_of::<Instance>(), 4096);
    }

    #[test]
    fn instance_globals_offset_correct() {
        let offset = offset_of!(Instance, globals_ptr) as isize;
        if offset != 4096 - 8 {
            let diff = 4096 - 8 - offset;
            let new_padding = INSTANCE_PADDING as isize + diff;
            panic!("new padding should be: {:?}", new_padding);
        }
        assert_eq!(offset_of!(Instance, globals_ptr), 4096 - 8);
    }
}
