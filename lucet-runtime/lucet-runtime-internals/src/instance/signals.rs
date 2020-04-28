use crate::alloc::validate_sigstack_size;
use crate::error::Error;
use crate::instance::{
    siginfo_ext::SiginfoExt, FaultDetails, Instance, State, TerminationDetails, CURRENT_INSTANCE,
    HOST_CTX,
};
use crate::sysdeps::UContextPtr;
use lazy_static::lazy_static;
use libc::{c_int, c_void, siginfo_t, SIGBUS, SIGSEGV};
use lucet_module::TrapCode;
use nix::sys::signal::{
    pthread_sigmask, raise, sigaction, SaFlags, SigAction, SigHandler, SigSet, SigmaskHow, Signal,
};
use std::convert::TryFrom;
use std::mem::MaybeUninit;
use std::ops::DerefMut;
use std::panic;
use std::sync::{Arc, Mutex};

lazy_static! {
    // TODO: work out an alternative to this that is signal-safe for `reraise_host_signal_in_handler`
    static ref LUCET_SIGNAL_STATE: Mutex<Option<SignalState>> = Mutex::new(None);

    static ref SIGNAL_HANDLER_MANUALLY_INSTALLED: Mutex<bool> = Mutex::new(false);
}

/// The value returned by
/// [`Instance.signal_handler`](struct.Instance.html#structfield.signal_handler) to determine the
/// outcome of a handled signal.
pub enum SignalBehavior {
    /// Use default behavior, which switches back to the host with `State::Fault` populated.
    Default,
    /// Override default behavior and cause the instance to continue.
    Continue,
    /// Override default behavior and cause the instance to terminate.
    Terminate,
}

pub type SignalHandler = dyn Fn(
    &Instance,
    &Option<TrapCode>,
    libc::c_int,
    *const siginfo_t,
    *const c_void,
) -> SignalBehavior;

pub fn signal_handler_none(
    _inst: &Instance,
    _trapcode: &Option<TrapCode>,
    _signum: libc::c_int,
    _siginfo_ptr: *const siginfo_t,
    _ucontext_ptr: *const c_void,
) -> SignalBehavior {
    SignalBehavior::Default
}

/// Install the Lucet signal handler for the current process.
///
/// This happens automatically by default, but must be run manually before running instances where
/// `instance.ensure_signal_handler_installed(false)` has been set.
///
/// Calling this function more than once without first calling `remove_lucet_signal_handler()` has
/// no additional effect.
pub fn install_lucet_signal_handler() {
    let mut installed = SIGNAL_HANDLER_MANUALLY_INSTALLED.lock().unwrap();
    if !*installed {
        increment_lucet_signal_state();
        *installed = true;
    }
}

/// Increment the count of currently-running instances, and install the signal handler if it is
/// currently missing.
///
/// The count only reflects running instances with `ensure_signal_handler_installed` set to `true`.
fn increment_lucet_signal_state() {
    let mut ostate = LUCET_SIGNAL_STATE.lock().unwrap();
    if let Some(state) = ostate.deref_mut() {
        state.counter += 1;
    } else {
        unsafe {
            setup_guest_signal_state(&mut ostate);
        }
    }
}

/// Remove the Lucet signal handler for the current process, restoring the signal handler that was
/// present when `install_lucet_signal_handler()` was called.
///
/// Calling this function without first calling `install_lucet_signal_handler()` has no effect.
pub fn remove_lucet_signal_handler() {
    let mut installed = SIGNAL_HANDLER_MANUALLY_INSTALLED.lock().unwrap();
    if *installed {
        decrement_lucet_signal_state();
        *installed = false;
    }
}

/// Decrement the count of currently-running instances, and remove the signal handler if the count
/// reaches zero.
///
/// The count only reflects running instances with `ensure_signal_handler_installed` set to `true`.
fn decrement_lucet_signal_state() {
    let mut ostate = LUCET_SIGNAL_STATE.lock().unwrap();
    let counter_zero = if let Some(state) = ostate.deref_mut() {
        state.counter -= 1;
        if state.counter == 0 {
            unsafe {
                restore_host_signal_state(state);
            }
            true
        } else {
            false
        }
    } else {
        panic!("signal handlers weren't installed at decrement");
    };
    if counter_zero {
        *ostate = None;
    }
}

impl Instance {
    pub(crate) fn with_signals_on<F, R>(&mut self, f: F) -> Result<R, Error>
    where
        F: FnOnce(&mut Instance) -> Result<R, Error>,
    {
        let previous_sigstack = if self.ensure_sigstack_installed {
            validate_sigstack_size(self.alloc.slot().limits.signal_stack_size)?;

            // Set up the signal stack for this thread. Note that because signal stacks are per-thread,
            // rather than per-process, we do this for every run, while the signal handler is installed
            // only once per process.
            let guest_sigstack = SigStack::new(
                self.alloc.slot().sigstack,
                SigStackFlags::empty(),
                self.alloc.slot().limits.signal_stack_size,
            );
            let previous_sigstack = unsafe { sigaltstack(Some(guest_sigstack)) }
                .expect("enabling or changing the signal stack succeeds");
            if let Some(previous_sigstack) = previous_sigstack {
                assert!(
                    !previous_sigstack
                        .flags()
                        .contains(SigStackFlags::SS_ONSTACK),
                    "an instance was created with a signal stack"
                );
            }
            previous_sigstack
        } else {
            // in debug mode only, make sure the installed sigstack is of sufficient size
            if cfg!(debug_assertions) {
                unsafe {
                    let mut current_sigstack = MaybeUninit::<libc::stack_t>::uninit();
                    libc::sigaltstack(std::ptr::null(), current_sigstack.as_mut_ptr());
                    let current_sigstack = current_sigstack.assume_init();
                    debug_assert!(
                        validate_sigstack_size(current_sigstack.ss_size).is_ok(),
                        "signal stack must be large enough"
                    );
                }
            }
            None
        };

        if self.ensure_signal_handler_installed {
            increment_lucet_signal_state();
        } else if cfg!(debug_assertions) {
            // in debug mode only, make sure the signal state is already present
            debug_assert!(
                LUCET_SIGNAL_STATE.lock().unwrap().is_some(),
                "signal handler is installed"
            );
        }

        // run the body
        let res = f(self);

        if self.ensure_signal_handler_installed {
            decrement_lucet_signal_state();
        }

        if self.ensure_sigstack_installed {
            unsafe {
                // restore the host signal stack for this thread
                if !altstack_flags()
                    .expect("the current stack flags could be retrieved")
                    .contains(SigStackFlags::SS_ONSTACK)
                {
                    sigaltstack(previous_sigstack).expect("sigaltstack restoration succeeds");
                }
            }
        }

        res
    }
}

/// Signal handler installed during instance execution.
///
/// This function is only designed to handle signals that are the direct result of execution of a
/// hardware instruction from the faulting WASM thread. It thus safely assumes the signal is
/// directed specifically at this thread (i.e. not a different thread or the process as a whole).
extern "C" fn handle_signal(signum: c_int, siginfo_ptr: *mut siginfo_t, ucontext_ptr: *mut c_void) {
    let signal = Signal::try_from(signum).expect("signum is a valid signal");
    if !(signal == Signal::SIGBUS
        || signal == Signal::SIGSEGV
        || signal == Signal::SIGILL
        || signal == Signal::SIGFPE
        || signal == Signal::SIGALRM)
    {
        panic!("unexpected signal in guest signal handler: {:?}", signal);
    }
    assert!(!siginfo_ptr.is_null(), "siginfo must not be null");

    // Safety: when using a SA_SIGINFO sigaction, the third argument can be cast to a `ucontext_t`
    // pointer per the manpage
    assert!(!ucontext_ptr.is_null(), "ucontext_ptr must not be null");
    let ctx = UContextPtr::new(ucontext_ptr);
    let rip = ctx.get_ip();

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

        if signal == Signal::SIGALRM {
            #[cfg(feature = "concurrent_testpoints")]
            inst.lock_testpoints
                .signal_handler_before_checking_alarm
                .check();
            if inst.kill_state.alarm_active() {
                inst.state = State::Terminating {
                    details: TerminationDetails::Remote,
                };
                return true;
            } else {
                // Ignore the alarm - this means we don't even want to change the signal context,
                // just act as if it never occurred.
                return false;
            }
        }

        let trapcode = inst.module.lookup_trapcode(rip);

        let behavior = (inst.signal_handler)(inst, &trapcode, signum, siginfo_ptr, ucontext_ptr);
        let switch_to_host = match behavior {
            SignalBehavior::Continue => {
                // return to the guest context without making any modifications to the instance
                false
            }
            SignalBehavior::Terminate => {
                // set the state before jumping back to the host context
                inst.state = State::Terminating {
                    details: TerminationDetails::Signal,
                };

                true
            }
            SignalBehavior::Default => {
                /*
                 * /!\ WARNING: LOAD-BEARING THUNK /!\
                 *
                 * This thunk, in debug builds, introduces multiple copies of UContext in the local
                 * stack frame. This also includes a local `State`, which is quite large as well.
                 * In total, this thunk accounts for roughly 5kb of stack use, where default signal
                 * stack sizes are typically 8kb total.
                 *
                 * In code paths that do not pass through this (such as immediately reraising as a
                 * host signal), the code in this thunk would force an exhaustion of more than half
                 * the stack, significantly increasing the likelihood the Lucet signal handler may
                 * overflow some other thread with a minimal stack size.
                 */
                let mut thunk = || {
                    // safety: pointer is checked for null at the top of the function, and the
                    // manpage guarantees that a siginfo_t will be passed as the second argument
                    let siginfo = unsafe { *siginfo_ptr };
                    let rip_addr = rip as usize;
                    // If the trap table lookup returned unknown, it is a fatal error
                    let unknown_fault = trapcode.is_none();

                    // If the trap was a segv or bus fault and the addressed memory was in the
                    // signal stack guard page or outside the alloc entirely, the fault is fatal
                    let outside_guard = (siginfo.si_signo == SIGSEGV || siginfo.si_signo == SIGBUS)
                        && inst
                            .alloc
                            .addr_location(siginfo.si_addr_ext())
                            .is_fault_fatal();

                    // record the fault and jump back to the host context
                    inst.state = State::Faulted {
                        details: FaultDetails {
                            fatal: unknown_fault || outside_guard,
                            trapcode,
                            rip_addr,
                            // Details set to `None` here: have to wait until `verify_trap_safety` to
                            // fill in these details, because access may not be signal safe.
                            rip_addr_details: None,
                        },
                        siginfo,
                        context: ctx.into(),
                    };
                };

                thunk();
                true
            }
        };

        if switch_to_host {
            #[cfg(feature = "concurrent_testpoints")]
            inst.lock_testpoints
                .signal_handler_before_disabling_termination
                .check();

            // we must disable termination so no KillSwitch for this execution may fire in host
            // code.
            let can_terminate = inst.kill_state.disable_termination();

            if !can_terminate {
                #[cfg(feature = "concurrent_testpoints")]
                inst.lock_testpoints
                    .signal_handler_after_unable_to_disable_termination
                    .check();

                // A killswitch began firing, but we're already going to switch to the host for a
                // more severe reason. Record that this instance's alarm must now be ignored.
                let ignored = inst.kill_state.silence_alarm();

                // If we'd already decided to ignore this instance's alarm, we must have already
                // signalled in a fatal way, *and* successfully disabled termination more than once
                // (which itself should be impossible).
                assert!(
                    !ignored,
                    "runtime must decide to ignore an instance's alarm at most once"
                );
            } else {
                // We are terminating this instance on account of `switch_to_host`, and we disabled
                // termination. Check in at the appropriate testpoint and continue.
                #[cfg(feature = "concurrent_testpoints")]
                inst.lock_testpoints
                    .signal_handler_after_disabling_termination
                    .check();
            }
        }

        switch_to_host
    });

    if switch_to_host {
        // Switch to host by preparing the context to switch when we return from the signal andler.
        // We must return from the signal handler for POSIX reasons, so instead prepare the context
        // that the signal handler will resume the program as if a call were made. First, by
        // pointing the instruction pointer at `lucet_context_set`, then by preparing the argument
        // that `lucet_context_set` should read from `rdi` - the context to switch to.
        //
        // NOTE: it is absolutely critical that `lucet_context_set` does not use the guest stack!
        // If it did, and the signal being handled were a segfault from reaching the guard page,
        // there would be no stack available for the function we return to. By not using stack
        // space, `lucet_context_set` is safe for use even in handling guard page faults.
        //
        // TODO: `rdi` is only correct for SysV (unixy) calling conventions! For Windows x86_64 this
        // would be `rcx`, with other architectures being their own question.
        ctx.set_ip(crate::context::lucet_context_set as *const c_void);
        HOST_CTX.with(|host_ctx| {
            ctx.set_rdi(host_ctx.get() as u64);
        });

        #[cfg(feature = "concurrent_testpoints")]
        CURRENT_INSTANCE.with(|current_instance| {
            let mut current_instance = current_instance.borrow_mut();

            // If we're switching to the host, there must be an instance, because we are switching away
            // from it.
            let inst = unsafe {
                current_instance
                    .as_mut()
                    .expect("current instance exists")
                    .as_mut()
            };

            // and the entire reason we're grabbing the instance again: lock for any races we're
            // testing with last-stretch signal handling.
            inst.lock_testpoints.signal_handler_before_returning.check();
        });
    }
}

struct SignalState {
    counter: usize,
    saved_sigbus: SigAction,
    saved_sigfpe: SigAction,
    saved_sigill: SigAction,
    saved_sigsegv: SigAction,
    saved_sigalrm: SigAction,
    saved_panic_hook: Option<Arc<Box<dyn Fn(&panic::PanicInfo<'_>) + Sync + Send + 'static>>>,
}

// raw pointers in the saved types
unsafe impl Send for SignalState {}

unsafe fn setup_guest_signal_state(ostate: &mut Option<SignalState>) {
    let mut masked_signals = SigSet::empty();
    masked_signals.add(Signal::SIGBUS);
    masked_signals.add(Signal::SIGFPE);
    masked_signals.add(Signal::SIGILL);
    masked_signals.add(Signal::SIGSEGV);
    masked_signals.add(Signal::SIGALRM);

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
    let saved_sigalrm = sigaction(Signal::SIGALRM, &sa).expect("sigaction succeeds");

    let saved_panic_hook = Some(setup_guest_panic_hook());

    *ostate = Some(SignalState {
        counter: 1,
        saved_sigbus,
        saved_sigfpe,
        saved_sigill,
        saved_sigsegv,
        saved_sigalrm,
        saved_panic_hook,
    });
}

fn setup_guest_panic_hook() -> Arc<Box<dyn Fn(&panic::PanicInfo<'_>) + Sync + Send + 'static>> {
    let saved_panic_hook = Arc::new(panic::take_hook());
    let closure_saved_panic_hook = saved_panic_hook.clone();
    std::panic::set_hook(Box::new(move |panic_info| {
        if panic_info
            .payload()
            .downcast_ref::<TerminationDetails>()
            .is_none()
        {
            closure_saved_panic_hook(panic_info);
        } else {
            // this is a panic used to implement instance termination (such as
            // `lucet_hostcall_terminate!`), so we don't want to print a backtrace; instead, we do
            // nothing
        }
    }));
    saved_panic_hook
}

unsafe fn restore_host_signal_state(state: &mut SignalState) {
    // restore signal handlers
    sigaction(Signal::SIGBUS, &state.saved_sigbus).expect("sigaction succeeds");
    sigaction(Signal::SIGFPE, &state.saved_sigfpe).expect("sigaction succeeds");
    sigaction(Signal::SIGILL, &state.saved_sigill).expect("sigaction succeeds");
    sigaction(Signal::SIGSEGV, &state.saved_sigsegv).expect("sigaction succeeds");
    sigaction(Signal::SIGALRM, &state.saved_sigalrm).expect("sigaction succeeds");

    // restore panic hook
    drop(panic::take_hook());
    state
        .saved_panic_hook
        .take()
        .map(|hook| Arc::try_unwrap(hook).map(|hook| panic::set_hook(hook)));
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
        if let Some(state) = LUCET_SIGNAL_STATE.lock().unwrap().as_ref() {
            match sig {
                Signal::SIGBUS => state.saved_sigbus.clone(),
                Signal::SIGFPE => state.saved_sigfpe.clone(),
                Signal::SIGILL => state.saved_sigill.clone(),
                Signal::SIGSEGV => state.saved_sigsegv.clone(),
                Signal::SIGALRM => state.saved_sigalrm.clone(),
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

////////////////////////////////////////////////////////////////////////////////////////////////////
// A collection of wrappers that will be upstreamed to the `nix` crate eventually.
////////////////////////////////////////////////////////////////////////////////////////////////////

use bitflags::bitflags;

#[derive(Copy, Clone)]
pub struct SigStack {
    stack: libc::stack_t,
}

impl SigStack {
    pub fn new(sp: *mut libc::c_void, flags: SigStackFlags, size: libc::size_t) -> SigStack {
        let stack = libc::stack_t {
            ss_sp: sp,
            ss_flags: flags.bits(),
            ss_size: size,
        };
        SigStack { stack }
    }

    pub fn disabled() -> SigStack {
        let stack = libc::stack_t {
            ss_sp: std::ptr::null_mut(),
            ss_flags: SigStackFlags::SS_DISABLE.bits(),
            ss_size: libc::SIGSTKSZ,
        };
        SigStack { stack }
    }

    pub fn flags(&self) -> SigStackFlags {
        SigStackFlags::from_bits_truncate(self.stack.ss_flags)
    }
}

impl AsRef<libc::stack_t> for SigStack {
    fn as_ref(&self) -> &libc::stack_t {
        &self.stack
    }
}

impl AsMut<libc::stack_t> for SigStack {
    fn as_mut(&mut self) -> &mut libc::stack_t {
        &mut self.stack
    }
}

bitflags! {
    pub struct SigStackFlags: libc::c_int {
        const SS_ONSTACK = libc::SS_ONSTACK;
        const SS_DISABLE = libc::SS_DISABLE;
    }
}

pub unsafe fn sigaltstack(new_sigstack: Option<SigStack>) -> nix::Result<Option<SigStack>> {
    let mut previous_stack = MaybeUninit::<libc::stack_t>::uninit();
    let disabled_sigstack = SigStack::disabled();
    let new_stack = match new_sigstack {
        None => &disabled_sigstack.stack,
        Some(ref new_stack) => &new_stack.stack,
    };
    let res = libc::sigaltstack(
        new_stack as *const libc::stack_t,
        previous_stack.as_mut_ptr(),
    );
    nix::errno::Errno::result(res).map(|_| {
        let sigstack = SigStack {
            stack: previous_stack.assume_init(),
        };
        if sigstack.flags().contains(SigStackFlags::SS_DISABLE) {
            None
        } else {
            Some(sigstack)
        }
    })
}

pub unsafe fn altstack_flags() -> nix::Result<SigStackFlags> {
    let mut current_stack = MaybeUninit::<libc::stack_t>::uninit();
    let res = libc::sigaltstack(std::ptr::null_mut(), current_stack.as_mut_ptr());
    nix::errno::Errno::result(res)
        .map(|_| SigStackFlags::from_bits_truncate(current_stack.assume_init().ss_flags))
}
