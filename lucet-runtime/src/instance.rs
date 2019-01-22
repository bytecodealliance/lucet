// temporary until the exported API is fleshed out
#![allow(dead_code)]

mod siginfo_ext;
mod signals;

use crate::alloc::{instance_heap_offset, Alloc};
use crate::context::Context;
use crate::instance::siginfo_ext::SiginfoExt;
use crate::instance::signals::{sigaltstack, SigStack, SigStackFlags};
use crate::module::{self, Module};
use crate::val::{UntypedRetVal, Val};
use failure::{bail, ensure, format_err, Error, ResultExt};
use lazy_static::lazy_static;
use libc::{c_int, c_void, siginfo_t, uintptr_t, SIGBUS, SIGSEGV};
use nix::sys::signal::{
    pthread_sigmask, raise, sigaction, SaFlags, SigAction, SigHandler, SigSet, SigmaskHow, Signal,
};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use std::cell::{RefCell, UnsafeCell};
use std::ffi::{CStr, CString};
use std::mem;
use std::ptr::{self, NonNull};
use std::sync::Mutex;

pub(crate) const LUCET_INSTANCE_MAGIC: u64 = 746932922;
pub(crate) const INSTANCE_PADDING: usize = 2296;

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
    static HOST_CTX: UnsafeCell<Context> = UnsafeCell::new(Context::new());
    static CURRENT_INSTANCE: RefCell<Option<NonNull<Instance>>> = RefCell::new(None);
}

lazy_static! {
    // TODO: work out an alternative to this that is signal-safe for `reraise_host_signal_in_handler`
    static ref LUCET_SIGNAL_STATE: Mutex<Option<SignalState>> = Mutex::new(None);
}

pub struct InstanceHandle {
    inst: NonNull<Instance>,
}

impl InstanceHandle {
    pub(crate) fn new(
        instance: *mut Instance,
        module: Box<dyn Module>,
        alloc: Alloc,
        embed_ctx: *mut c_void,
    ) -> Result<Self, Error> {
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
}

// Safety argument for these deref impls: the instance's `Alloc` field contains an `Arc` to the
// region that backs this memory, keeping the page containing the `Instance` alive as long as the
// region exists

impl std::ops::Deref for InstanceHandle {
    type Target = Instance;
    fn deref(&self) -> &Self::Target {
        unsafe { self.inst.as_ref() }
    }
}

impl std::ops::DerefMut for InstanceHandle {
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
    pub(crate) magic: u64,

    /// The embedding context is a pointer from the embedder that is used to implement hostcalls
    pub embed_ctx: *mut c_void,

    /// The program (WebAssembly module) that is the entrypoint for the instance.
    pub(crate) module: Box<dyn Module>,

    /// The `Context` in which the guest program runs
    ctx: Context,

    /// Instance state and error information
    pub state: State,

    /// The memory allocated for this instance
    pub(crate) alloc: Alloc,

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
    pub(crate) globals_ptr: *const i64,
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

    /// Get an Instance from the `vmctx` pointer.
    ///
    /// Only safe to call from within the guest context.
    pub(crate) unsafe fn from_vmctx<'a>(vmctx: *const c_void) -> &'a mut Instance {
        assert!(!vmctx.is_null(), "vmctx is not null");

        let inst_ptr = (vmctx as usize - instance_heap_offset()) as *mut Instance;

        // We shouldn't actually need to access the thread local, only the exception handler should
        // need to. But, as long as the thread local exists, we should make sure that the guest
        // hasn't pulled any shenanigans and passed a bad vmctx. (Codegen should ensure the guest
        // cant pull any shenanigans but there have been bugs before.)
        CURRENT_INSTANCE.with(|current_instance| {
            if let Some(current_inst_ptr) = current_instance.borrow().map(|nn| nn.as_ptr()) {
                assert!(
                    inst_ptr == current_inst_ptr,
                    "vmctx corresponds to current instance"
                );
            } else {
                panic!(
                    "current instance is not set; thread local storage failure can indicate \
                     dynamic linking issues"
                );
            }
        });

        let inst = inst_ptr.as_mut().unwrap();
        assert!(inst.valid_magic());
        inst
    }

    pub(crate) fn valid_magic(&self) -> bool {
        self.magic == LUCET_INSTANCE_MAGIC
    }

    // TODO: richer error types for this whole family of functions
    pub fn run(&mut self, entrypoint: &[u8], args: &[Val]) -> Result<&State, Error> {
        let func = self.module.get_export_func(entrypoint)?;
        self.run_func(func, &args)
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

    fn with_signals_on<F, R>(&mut self, f: F) -> Result<R, Error>
    where
        F: FnOnce(&mut Instance) -> Result<R, Error>,
    {
        // setup signal stack for this thread
        let guest_sigstack = SigStack::new(
            self.alloc.slot().sigstack,
            SigStackFlags::empty(),
            libc::SIGSTKSZ,
        );
        let saved_sigstack = unsafe { sigaltstack(&guest_sigstack).expect("sigaltstack succeeds") };

        let mut ostate = LUCET_SIGNAL_STATE.lock().unwrap();
        if let Some(ref mut state) = *ostate {
            state.counter += 1;
        } else {
            unsafe {
                setup_guest_signal_state(&mut ostate);
            }
        }
        drop(ostate);

        // run the body
        let res = f(self);

        let mut ostate = LUCET_SIGNAL_STATE.lock().unwrap();
        let counter_zero = if let Some(ref mut state) = *ostate {
            state.counter -= 1;
            if state.counter == 0 {
                unsafe {
                    // restore the host signal stack
                    sigaltstack(&saved_sigstack).expect("sigaltstack succeeds");
                    restore_host_signal_state(state);
                }
                true
            } else {
                false
            }
        } else {
            panic!("signal handlers weren't installed at instance exit");
        };
        if counter_zero {
            *ostate = None;
        }

        res
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

    fn run_start(&mut self) -> Result<(), Error> {
        if let Some(start) = self.module.get_start_func()? {
            self.run_func(start, &[]).context("module start")?;
            if !self.is_ready() {
                bail!("unexpected state after module start: {}", self.state);
            }
        }
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

    // must only be called from within the guest context
    pub(crate) unsafe fn terminate(&mut self, info: *mut c_void) -> ! {
        self.state = State::Terminated { info };
        HOST_CTX.with(|host_ctx| Context::set(&*host_ctx.get()))
    }

    // TODO: replace this with a richer `run` interface?
    pub fn is_ready(&self) -> bool {
        self.state.is_ready()
    }

    pub fn is_terminated(&self) -> bool {
        self.state.is_terminated()
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

struct SignalState {
    counter: usize,
    saved_sigbus: SigAction,
    saved_sigfpe: SigAction,
    saved_sigill: SigAction,
    saved_sigsegv: SigAction,
}

// raw pointers in the saved types
unsafe impl Send for SignalState {}

unsafe fn setup_guest_signal_state(ostate: &mut Option<SignalState>) {
    let mut masked_signals = SigSet::empty();
    masked_signals.add(Signal::SIGBUS);
    masked_signals.add(Signal::SIGFPE);
    masked_signals.add(Signal::SIGILL);
    masked_signals.add(Signal::SIGSEGV);

    // setup signal handlers
    let sa = SigAction::new(
        SigHandler::SigAction(handle_signal),
        SaFlags::SA_RESTART | SaFlags::SA_SIGINFO | SaFlags::SA_ONSTACK,
        masked_signals,
    );
    let saved_sigbus = sigaction(Signal::SIGBUS, &sa).expect("sigaction succeeds");
    let saved_sigfpe = sigaction(Signal::SIGFPE, &sa).expect("sigaction succeeds");
    let saved_sigill = sigaction(Signal::SIGILL, &sa).expect("sigaction succeeds");
    let saved_sigsegv = sigaction(Signal::SIGSEGV, &sa).expect("sigaction succeeds");

    *ostate = Some(SignalState {
        counter: 1,
        saved_sigbus,
        saved_sigfpe,
        saved_sigill,
        saved_sigsegv,
    });
}

unsafe fn restore_host_signal_state(state: &mut SignalState) {
    // restore signal handlers
    sigaction(Signal::SIGBUS, &state.saved_sigbus).expect("sigaction succeeds");
    sigaction(Signal::SIGFPE, &state.saved_sigfpe).expect("sigaction succeeds");
    sigaction(Signal::SIGILL, &state.saved_sigill).expect("sigaction succeeds");
    sigaction(Signal::SIGSEGV, &state.saved_sigsegv).expect("sigaction succeeds");
}

unsafe fn reraise_host_signal_in_handler(
    sig: Signal,
    signum: libc::c_int,
    siginfo_ptr: *mut libc::siginfo_t,
    ucontext_ptr: *mut c_void,
) {
    let saved_handler = {
        // TODO: avoid taking a mutex here, probably by having some static muts just for this
        // function
        if let Some(ref state) = *LUCET_SIGNAL_STATE.lock().unwrap() {
            match sig {
                Signal::SIGBUS => state.saved_sigbus.clone(),
                Signal::SIGFPE => state.saved_sigfpe.clone(),
                Signal::SIGILL => state.saved_sigill.clone(),
                Signal::SIGSEGV => state.saved_sigsegv.clone(),
                sig => panic!(
                    "unexpected signal in reraise_host_signal_in_handler: {:?}",
                    sig
                ),
            }
        } else {
            // this case is very fishy; it can arise when the last lucet instance spins down and
            // uninstalls the lucet handlers while a signal handler is running on this thread, but
            // before taking the mutex above. The theory is that if this has happened, the host
            // handler has been reinstalled, so we shouldn't end up back here if we reraise

            // unmask the signal to reraise; we don't have to restore it because the handler will return
            // after this. If it signals again between here and now, that's a double fault and the
            // process is going to die anyway
            let mut unmask = SigSet::empty();
            unmask.add(sig);
            pthread_sigmask(SigmaskHow::SIG_UNBLOCK, Some(&unmask), None)
                .expect("pthread_sigmask succeeds");
            // if there's no current signal state, just re-raise and hope for the best
            raise(sig).expect("raise succeeds");
            return;
        }
    };

    match saved_handler.handler() {
        SigHandler::SigDfl => {
            // reinstall default signal handler and reraise the signal; this should terminate the
            // program
            sigaction(sig, &saved_handler).expect("sigaction succeeds");
            let mut unmask = SigSet::empty();
            unmask.add(sig);
            pthread_sigmask(SigmaskHow::SIG_UNBLOCK, Some(&unmask), None)
                .expect("pthread_sigmask succeeds");
            raise(sig).expect("raise succeeds");
        }
        SigHandler::SigIgn => {
            // don't do anything; if we hit this case, whatever program is hosting us is almost
            // certainly doing something wrong, because our set of signals requires intervention to
            // proceed
            return;
        }
        SigHandler::Handler(f) => {
            // call the saved handler directly so there is no altstack confusion
            f(signum)
        }
        SigHandler::SigAction(f) => {
            // call the saved handler directly so there is no altstack confusion
            f(signum, siginfo_ptr, ucontext_ptr)
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

#[derive(Copy, Clone, Debug)]
pub struct TrapCode {
    pub ty: TrapCodeType,
    pub tag: u16,
}

impl std::fmt::Display for TrapCode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.ty == TrapCodeType::User {
            write!(f, "{:?}({})", self.ty, self.tag)
        } else {
            write!(f, "{:?}", self.ty)
        }
    }
}

impl TrapCode {
    pub(crate) fn try_from_u32(trapcode_bin: u32) -> Option<TrapCode> {
        let trapcode_type = (trapcode_bin & 0x0000FFFF) as u16;
        TrapCodeType::from_u16(trapcode_type).map(|ty| {
            let tag = (trapcode_bin >> 16) as u16;
            TrapCode { ty, tag }
        })
    }
}

#[repr(u16)]
#[derive(Copy, Clone, Debug, FromPrimitive, PartialEq)]
pub enum TrapCodeType {
    StackOverflow = 0,
    HeapOutOfBounds = 1,
    OutOfBounds = 2,
    IndirectCallToNull = 3,
    BadSignature = 4,
    IntegerOverflow = 5,
    IntegerDivByZero = 6,
    BadConversionToInteger = 7,
    Interrupt = 8,
    TableOutOfBounds = 9,
    User = 0xFFFF,
    Unknown = 0xFFFE,
}

pub enum SignalBehavior {
    /// Use default behavior
    None,
    /// Override default behavior and cause the instance to continue
    Continue,
    /// Override default behavior and cause the instance to terminate
    Terminate,
}

fn default_fatal_handler(inst: &Instance) -> ! {
    panic!("> instance {:p} had fatal error: {}", inst, inst.state);
}

fn signal_handler_none(
    _inst: &Instance,
    _trapcode: &TrapCode,
    _signum: libc::c_int,
    _siginfo_ptr: *const siginfo_t,
    _ucontext_ptr: *const c_void,
) -> SignalBehavior {
    SignalBehavior::None
}

/// Signal handler installed during instance execution.
///
/// This function is only designed to handle signals that are the direct result of execution of a
/// hardware instruction from the faulting WASM thread. It thus safely assumes the signal is
/// directed specifically at this thread (i.e. not a different thread or the process as a whole).
extern "C" fn handle_signal(signum: c_int, siginfo_ptr: *mut siginfo_t, ucontext_ptr: *mut c_void) {
    let signal = Signal::from_c_int(signum).expect("signum is a valid signal");
    if !(signal == Signal::SIGBUS
        || signal == Signal::SIGSEGV
        || signal == Signal::SIGILL
        || signal == Signal::SIGFPE)
    {
        panic!("unexpected signal in guest signal handler: {:?}", signal);
    }
    assert!(!siginfo_ptr.is_null(), "siginfo must not be null");

    // Safety: when using a SA_SIGINFO sigaction, the third argument can be cast to a `ucontext_t`
    // pointer per the manpage
    let ctx = unsafe {
        (ucontext_ptr as *mut libc::ucontext_t)
            .as_ref()
            .expect("ucontext must not be null")
    };
    let rip = ctx.uc_mcontext.gregs[libc::REG_RIP as usize] as *const c_void;

    let switch_to_host = CURRENT_INSTANCE.with(|current_instance| {
        let mut current_instance = current_instance.borrow_mut();

        if current_instance.is_none() {
            // If there is no current instance, we've caught a signal raised by a thread that's not
            // running a lucet instance. Restore the host signal handler and reraise the signal,
            // then return if the host handler returns
            unsafe {
                reraise_host_signal_in_handler(signal, signum, siginfo_ptr, ucontext_ptr);
            }
            // don't try context-switching
            return false;
        }

        // Safety: the memory pointed to by CURRENT_INSTANCE should be a valid instance. This is not
        // a trivial property, but relies on the compiler not emitting guest programs that can
        // overwrite the instance.
        let inst = unsafe {
            current_instance
                .as_mut()
                .expect("current instance exists")
                .as_mut()
        };

        let trapcode = inst
            .module
            .lookup_trapcode(rip)
            // if we couldn't find a code in the manifest, return an unknown trapcode that will be
            // converted to a fatal trap when we switch back to the host
            .unwrap_or(TrapCode {
                ty: TrapCodeType::Unknown,
                tag: 0,
            });

        let behavior = (inst.signal_handler)(inst, &trapcode, signum, siginfo_ptr, ucontext_ptr);
        match behavior {
            SignalBehavior::Continue => {
                // return to the guest context without making any modifications to the instance
                false
            }
            SignalBehavior::Terminate => {
                // set the state before jumping back to the host context
                inst.state = State::Terminated {
                    info: ptr::null_mut(),
                };
                true
            }
            SignalBehavior::None => {
                // record the fault and jump back to the host context
                inst.state = State::Fault {
                    // fatal field is set false here by default - have to wait until
                    // `verify_trap_safety` to have enough information to determine whether trap was
                    // fatal. It is not signal safe to access some of the required information.
                    fatal: false,
                    trapcode: trapcode,
                    rip_addr: rip as usize,
                    rip_addr_details: None,
                    // safety: pointer is checked for null at the top of the function, and the
                    // manpage guarantees that a siginfo_t will be passed as the second argument
                    siginfo: unsafe { *siginfo_ptr },
                    context: *ctx,
                };
                true
            }
        }
    });

    if switch_to_host {
        HOST_CTX.with(|host_ctx| unsafe {
            Context::set_from_signal(&*host_ctx.get())
                .expect("can successfully switch back to the host context");
        });
        unreachable!()
    }
}

// TODO: figure out where to put all of these

#[no_mangle]
pub unsafe extern "C" fn lucet_vmctx_get_heap(vmctx: *const c_void) -> *mut u8 {
    let inst = Instance::from_vmctx(vmctx);
    inst.alloc.slot().heap as *mut u8
}

/// Get the number of WebAssembly pages currently in the heap.
#[no_mangle]
pub unsafe extern "C" fn lucet_vmctx_current_memory(vmctx: *const c_void) -> libc::uint32_t {
    let inst = Instance::from_vmctx(vmctx);
    inst.alloc.heap_len() as u32 / WASM_PAGE_SIZE
}

#[no_mangle]
/// Grows the guest heap by the given number of WebAssembly pages.
///
/// On success, returns the number of pages that existed before the call. On failure, returns `-1`.
pub unsafe extern "C" fn lucet_vmctx_grow_memory(
    vmctx: *const c_void,
    additional_pages: libc::uint32_t,
) -> libc::int32_t {
    let inst = Instance::from_vmctx(vmctx);
    if let Ok(old_pages) = inst.grow_memory(additional_pages) {
        old_pages as libc::int32_t
    } else {
        -1
    }
}

#[no_mangle]
/// Check if a memory region is inside the instance heap.
pub unsafe extern "C" fn lucet_vmctx_check_heap(
    vmctx: *const c_void,
    ptr: *mut c_void,
    len: libc::size_t,
) -> bool {
    let inst = Instance::from_vmctx(vmctx);
    inst.check_heap(ptr, len)
}

#[no_mangle]
pub unsafe extern "C" fn lucet_vmctx_terminate(vmctx: *const c_void, info: *mut c_void) {
    let inst = Instance::from_vmctx(vmctx);
    inst.terminate(info);
}

#[no_mangle]
/// Get the delegate object for the current instance.
pub unsafe extern "C" fn lucet_vmctx_get_delegate(vmctx: *const c_void) -> *mut c_void {
    let inst = Instance::from_vmctx(vmctx);
    inst.embed_ctx
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
