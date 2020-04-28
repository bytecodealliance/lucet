//! The `execution` module contains state for an instance's execution, and exposes functions
//! building that state into something appropriate for safe use externally.
//!
//! So far as state tracked in this module is concerned, there are two key items:
//! "execution domain" and "terminability".
//!
//! ## Execution Domain
//!
//! Execution domains allow us to distinguish what an appropriate mechanism to signal termination
//! is. This means that changing of an execution domain must be atomic - it would be an error to
//! read the current execution domain, continue with that domain to determine temination, and
//! simultaneously for execution to continue possibly into a different execution domain. For
//! example, beginning termination directly at the start of a hostcall, where sending `SIGALRM` may
//! be appropriate, while the domain switches to `Hostcall` and is no longer appropriate for
//! signalling, would be an error.
//!
//! ## Terminability
//!
//! This is used to answer the question "is it safe to initiate termination of this instance right
//! now?".  An instance is terminable when it is created or reset. An instance stops being
//! terminable when it is terminated, when it faults, or when it is dropped. When an instance
//! finishes running, it will once again be terminable, but existing [`KillSwitch`] objects will no
//! longer be valid.
//!
//! Termination does not directly map to the idea of guest code currently executing on a processor,
//! because termination can occur before the guest has started, during host code, or while a guest
//! has yielded execution.
//!
//! As a result, termination of a running instance can only be treated as a best-effort to
//! deschedule a guest. This is typically quick when it occurs during guest code execution, and
//! otherwise happens immediately upon resuming execution of guest code (exiting host code, or
//! resuming a yielded instance).
//!
//! ## Instance Lifecycle and `KillState`
//!
//! And now we can enumerate interleavings of execution and terminability, to see the expected
//! state at possible points of interest in an instance's lifecycle:
//!
//! * `Instance created`
//!   - terminable: `true`
//!   - execution_domain: `Pending`
//!   - termination result: `Ok(KillSuccess::Cancelled)`
//! * `Instance::run executing`
//!   - terminable: `true` or `false`
//!   - termination result: `Ok(KillSuccess::Signalled)`, `Ok(KillSuccess::Pending)`, or
//!     `Err(KillError::NotTerminable)`
//!   - execution_domain: `Guest, Hostcall, or Terminated`
//!   - `execution_domain` will only be `Guest` when executing guest code, only be `Hostcall` when
//!     executing a hostcall, but may also be `Terminated` while in a hostcall to indicate that it
//!     should exit when the hostcall completes.
//!   - `terminable` will be false if and only if `execution_domain` is `Terminated`.
//!   - `KillSwitch::terminate` will succeed with `Signalled` when terminated while executing guest
//!     code, `Pending` when terminated while executing a hostcall, and will fail with
//!     `NotTerminable` when the instance has already been terminated.
//! * `Instance::run returns`
//!   - terminable: `true`
//!   - execution_domain: `Pending, Hostcall, or Terminated`
//!   - termination result: `Err(KillError::Invalid)`
//!   - `execution_domain` will be `Pending` when the initial guest function returns, `Hostcall`
//!     when terminated by `lucet_hostcall_terminate!`, and `Terminated` when exiting due to a
//!     termination request.
//!   - While `terminable` *is* true, it should be noted that [`KillState`] is reset here, and
//!     existing `KillSwitch` objects will no longer hold a reference to the instance kill state.
//! * `Guest function executing`
//!   - terminable: `true`
//!   - termination result: `Ok(KillSuccess::Signalled)`
//!   - execution_domain: `Guest`
//! * `Hostcall called`
//!   - termination result: `Ok(KillSuccess::Pending)`
//!   - execution_domain: `Hostcall`
//! * `Hostcall executing`
//!   - termination result: `Ok(KillSuccess::Pending)` or `Err(KillError::NotTerminable)`
//!   - execution_domain: `Hostcall, or Terminated`
//!   - `execution_domain` will typically be `Hostcall`, but may be `Terminated` if termination of
//!     the instance is requested during the hostcall.
//!   - `KillSwitch::terminate` will return `NotTerminable` if and only if the instance has already
//!     been terminated.
//! * `Hostcall yields`, or `Hostcall resumes`
//!   - These are specific points in "Hostcall executing" and has no further semantics.
//! * `Hostcall returns`
//!   - termination result: `Ok(KillSuccess::Signalled)`
//!   - execution_domain: `Guest`
//!   - `execution_domain` may be `Terminated` before returning, in which case `terminate` will
//!     return `NotTerminable`, but the hostcall would then exit. If a hostcall successfully
//!     returns to its caller it was not terminated, so the only state an instance will have after
//!     returning from a hostcall will be that it is executing terminable guest code.
//! * `Guest faults`, `Hostcall faults`, `Guest returns`, `Instance::reset` or `Instance dropped`
//!   - termination result: `Err(KillError::Invalid)`
//!
//!   - If an instance is reset or finishes running (through fault or normal return), outstanding
//!   `KillSwitch` objects will no longer hold a valid reference to the instance. In that case,
//!   calls to `terminate` will return `Invalid`, and have no bearing on further execution.
//!
//! ## Further Reading
//!
//! For more information about kill state, execution domains, and instance termination, see
//! [`KillState`](struct.KillState.html), [`Domain`](enum.Domain.html), and
//! [`KillSwitch::terminate`](struct.KillSwitch.html#method.terminate), respectively.
//!
//! For more information about signal-safe behavior, see `signal-safety(7)`.
use libc::{pthread_kill, pthread_t, SIGALRM};
use std::mem;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, Weak};

use crate::instance::{Instance, TerminationDetails};
#[cfg(feature = "concurrent_testpoints")]
use crate::lock_testpoints::LockTestpoints;

/// All instance state a remote kill switch needs to determine if and how to signal that execution
/// should stop.
///
/// Some definitions for reference in this struct's documentation:
/// * "stopped" means "stop executing at some point before reaching the end of the entrypoint
/// wasm function".
/// * "critical section" means what it typically means - an uninterruptable region of code. The
/// detail here is that currently "critical section" and "hostcall" are interchangeable, but in
/// the future this may change. Hostcalls may one day be able to opt out of criticalness, or
/// perhaps guest code may include critical sections.
///
/// "Stopped" is a particularly loose word here because it encompasses the worst case: trying to
/// stop a guest that is currently in a critical section. Because the signal will only be checked
/// when exiting the critical section, the latency is bounded by whatever embedder guarantees are
/// made. In fact, it is possible for a kill signal to be successfully sent and still never be
/// impactful, if a hostcall itself invokes `lucet_hostcall_terminate!`. In this circumstance, the
/// hostcall would terminate the instance if it returned, but `lucet_hostcall_terminate!` will
/// terminate the guest before the termination request would even be checked.
pub struct KillState {
    /// Can the instance be terminated? This must be `true` only when the instance can be stopped.
    /// This may be false while the instance can safely be stopped, such as immediately after
    /// completing a host->guest context swap. Regions such as this should be minimized, but are
    /// not a problem of correctness.
    ///
    /// Typically, this is true while in any guest code, or hostcalls made from guest code.
    terminable: AtomicBool,
    /// The kind of code is currently executing in the instance this `KillState` describes.
    ///
    /// This allows a `KillSwitch` to determine what the appropriate signalling mechanism is in
    /// `terminate`. Locks on `execution_domain` prohibit modification while signalling, ensuring
    /// both that:
    /// * we don't enter a hostcall while someone may decide it is safe to signal, and
    /// * no one may try to signal in a hostcall-safe manner after exiting a hostcall, where it
    ///   may never again be checked by the guest.
    execution_domain: Mutex<Domain>,
    /// The current `thread_id` the associated instance is running on. This is the TID where
    /// `SIGALRM` will be sent if the instance is killed via `KillSwitch::terminate` and a signal
    /// is an appropriate mechanism.
    thread_id: Mutex<Option<pthread_t>>,
    /// `tid_change_notifier` allows functions that may cause a change in `thread_id` to wait,
    /// without spinning, for the signal to be processed.
    tid_change_notifier: Condvar,
    /// `ignore_alarm` indicates if a SIGALRM directed at this KillState's instance must be
    /// ignored. This is necessary for a specific race where a termination occurs right around when
    /// a Lucet guest, or hostcall the guest made, handles some other signal: if the termination
    /// occurs during handling of a signal that arose from guest code, a SIGALRM will either be
    /// pending, masked by Lucet's sigaction's signal mask, OR a SIGLARM will be imminent after
    /// handling the signal.
    ignore_alarm: AtomicBool,
    #[cfg(feature = "concurrent_testpoints")]
    /// When testing race permutations, `KillState` keeps a reference to the `LockTestpoints` its
    /// associated instance holds.
    lock_testpoints: Arc<LockTestpoints>,
}

/// Enter a guest region.
///
/// This is the entry callback function installed on the [`Instance`](struct.Instance.html) for a
/// guest. This is called by `lucet_context_activate` after a context switch, just before we begin
/// execution in a guest region.
///
/// This function is responsible for setting the execution domain in the instance's
/// [`KillState`](struct.KillState.html), so that we can appropriately signal the instance to
/// terminate if needed. If the instance was already terminated, swap back to the host context
/// rather than returning.
///
/// # Safety
///
/// This function will call [Instance::terminate](struct.Instance.html#method.terminate) if the
/// execution domain is [`Domain::Cancelled`](enum.Domain.html). In that case, this function will
/// not return, and we will swap back to the host context without unwinding.
///
/// This function will panic if the `Instance`'s execution domain is marked as currently executing
/// guest code, currently in a hostcall, or as cancelled.  Attempting to enter an instance from any
/// of these domains means that something has gone seriously wrong.
pub unsafe extern "C" fn enter_guest_region(instance: *mut Instance) {
    let instance = instance.as_mut().expect("instance pointer cannot be null");

    #[cfg(feature = "concurrent_testpoints")]
    instance
        .lock_testpoints
        .instance_entering_guest_before_domain_change
        .check();

    let mut current_domain = instance.kill_state.execution_domain.lock().unwrap();
    match *current_domain {
        Domain::Pending => {
            // All systems go! We are about to start executing a guest. Set the execution domain
            // accordingly, and then return so we can jump to the guest code.
            *current_domain = Domain::Guest;

            // explicitly drop `current_domain` to release the lock before reaching testpoint
            mem::drop(current_domain);

            #[cfg(feature = "concurrent_testpoints")]
            instance
                .lock_testpoints
                .instance_entering_guest_after_domain_change
                .check();
        }
        Domain::Guest => {
            // This is an error because it suggests a KillSwitch could send a SIGALRM before
            // we have indicated that it safe to do so.
            panic!(
                "Invalid state: Instance marked as already in guest while entering a guest region."
            );
        }
        Domain::Hostcall => {
            // We will not pass through `enter_guest_region` again when returning from a hostcall,
            // so this should never happen.
            panic!(
                "Invalid state: Instance marked as in a hostcall while entering a guest region."
            );
        }
        Domain::Terminated => {
            panic!("Invalid state: Instance marked as terminated while entering a guest region.");
        }
        Domain::Cancelled => {
            // A KillSwitch terminated our Instance before it began executing guest code. We should
            // not enter the guest region. We will instead terminate the Instance, and then swap
            // back to the host context.
            //
            // Note that we manually drop the domain because`Instance::terminate` never returns.
            mem::drop(current_domain);
            instance.terminate(TerminationDetails::Remote);
        }
    }
}

/// Exit a guest region.
///
/// This is the backstop callback function installed on the [`Instance`](struct.Instance.html) for a
/// guest. This is called by `lucet_context_backstop` after we have finishing execution in
/// a guest region.
///
/// # Safety
///
/// For more information about the safety constraints of the backstop callback, see
/// [`Instance::init`](struct.Instance.html#method.init).
///
/// This function will panic if the `Instance`'s execution domain is marked as pending, currently
/// in a hostcall, or as cancelled.  Any of these domains mean that something has gone seriously
/// wrong, and leaving the execution domain mutex in a poisoned state is the least of our concerns.
pub unsafe extern "C" fn exit_guest_region(instance: *mut Instance) {
    let instance = instance.as_mut().expect("instance pointer cannot be null");

    #[cfg(feature = "concurrent_testpoints")]
    instance
        .lock_testpoints
        .instance_exiting_guest_before_acquiring_terminable
        .check();

    let terminable = instance.kill_state.terminable.swap(false, Ordering::SeqCst);
    if !terminable {
        // If we are here, something else has taken the terminable flag, so it's not safe to
        // actually exit a guest context yet. Because this is called when exiting a guest context,
        // the termination mechanism will be a signal, delivered at some point (hopefully soon!).
        // Further, because the termination mechanism will be a signal, we are constrained to only
        // signal-safe behavior. So, we will hang indefinitely waiting for the sigalrm to arrive.

        #[cfg(feature = "concurrent_testpoints")]
        instance
            .lock_testpoints
            .instance_exiting_guest_without_terminable
            .check();

        #[allow(clippy::empty_loop)]
        loop {}
    } else {
        let current_domain = instance.kill_state.execution_domain.lock().unwrap();
        match *current_domain {
            Domain::Guest => {
                // We finished executing the code in our guest region normally! We should reset
                // the kill state, invalidating any existing killswitches' weak references.
                //
                // There should be only one strong reference to `kill_state`, since acquiring
                // `terminable` prevents a `KillSwitch` from firing and serves as a witness that
                // none are in the process of firing. As a consistency check, ensure that's still
                // true. This is necessary so that when we drop this `Arc`, weak refs are no longer
                // valid. If this assert fails, something cloned `KillState`, or a `KillSwitch` has
                // upgraded its ref - both of these are errors!
                assert_eq!(Arc::strong_count(&instance.kill_state), 1);
            }
            ref domain @ Domain::Pending
            | ref domain @ Domain::Cancelled
            | ref domain @ Domain::Terminated
            | ref domain @ Domain::Hostcall => {
                // If we are exiting a guest that is currently marked as pending, cancelled,
                // terminated, or in a hostcall, something has gone very wrong.
                panic!(
                    "Invalid state: Instance marked as {:?} while exiting a guest region.",
                    domain
                );
            }
        };

        #[cfg(feature = "concurrent_testpoints")]
        instance
            .lock_testpoints
            .instance_exiting_guest_after_domain_change
            .check();
    }
}

#[cfg(not(feature = "concurrent_testpoints"))]
impl Default for KillState {
    fn default() -> Self {
        Self {
            terminable: AtomicBool::new(true),
            tid_change_notifier: Condvar::new(),
            execution_domain: Mutex::new(Domain::Pending),
            thread_id: Mutex::new(None),
            ignore_alarm: AtomicBool::new(false),
        }
    }
}

impl KillState {
    #[cfg(not(feature = "concurrent_testpoints"))]
    /// Construct a new `KillState`.
    pub fn new() -> Self {
        Default::default()
    }

    #[cfg(feature = "concurrent_testpoints")]
    pub fn new(lock_testpoints: Arc<LockTestpoints>) -> KillState {
        KillState {
            terminable: AtomicBool::new(true),
            tid_change_notifier: Condvar::new(),
            execution_domain: Mutex::new(Domain::Pending),
            thread_id: Mutex::new(None),
            ignore_alarm: AtomicBool::new(false),
            lock_testpoints,
        }
    }

    pub fn is_terminable(&self) -> bool {
        self.terminable.load(Ordering::SeqCst)
    }

    pub fn enable_termination(&self) {
        self.terminable.store(true, Ordering::SeqCst);
    }

    pub fn disable_termination(&self) -> bool {
        self.terminable.swap(false, Ordering::SeqCst)
    }

    pub fn terminable_ptr(&self) -> *const AtomicBool {
        &self.terminable as *const AtomicBool
    }

    pub fn silence_alarm(&self) -> bool {
        self.ignore_alarm.swap(true, Ordering::SeqCst)
    }

    pub fn alarm_active(&self) -> bool {
        !self.ignore_alarm.load(Ordering::SeqCst)
    }

    /// Set the execution domain to signify that we are currently executing a hostcall.
    ///
    /// This method will panic if the execution domain is currently marked as anything but
    /// `Domain::Guest`, because any other domain means that we have somehow entered an invalid
    /// state.
    ///
    /// This method will also panic if the mutex on the execution domain has been poisoned.
    pub fn begin_hostcall(&self) {
        #[cfg(feature = "concurrent_testpoints")]
        self.lock_testpoints
            .instance_entering_hostcall_before_domain_change
            .check();

        let mut current_domain = self.execution_domain.lock().unwrap();
        match *current_domain {
            Domain::Pending => {
                panic!("Invalid state: Instance marked as pending while in guest code. This should be an error.");
            }
            Domain::Guest => {
                // Guest is the expected domain until this point. Switch to the Hostcall
                // domain so we know to not interrupt this instance.
                *current_domain = Domain::Hostcall;
            }
            Domain::Hostcall => {
                panic!(
                    "Invalid state: Instance marked as in a hostcall while entering a hostcall."
                );
            }
            Domain::Terminated => {
                panic!("Invalid state: Instance marked as terminated while in guest code. This should be an error.");
            }
            Domain::Cancelled => {
                panic!("Invalid state: Instance marked as cancelled while in guest code. This should be an error.");
            }
        }

        // explicitly drop `current_domain` to release the lock before reaching testpoint
        mem::drop(current_domain);

        #[cfg(feature = "concurrent_testpoints")]
        self.lock_testpoints
            .instance_entering_hostcall_after_domain_change
            .check();
    }

    /// Set the execution domain to signify that we are finished executing a hostcall.
    ///
    /// If the instance was terminated during the hostcall, then we will return termination details
    /// to the caller signifying that we were remotely terminated. This method will panic if the
    /// execution domain is currently marked as pending, in guest code, or as cancelled, because
    /// each of these mean we have somehow entered an invalid state.
    ///
    /// This method will also panic if the mutex on the execution domain has been poisoned.
    pub fn end_hostcall(&self) -> Option<TerminationDetails> {
        #[cfg(feature = "concurrent_testpoints")]
        self.lock_testpoints
            .instance_exiting_hostcall_before_domain_change
            .check();

        let mut current_domain = self.execution_domain.lock().unwrap();
        let res = match *current_domain {
            Domain::Pending => {
                panic!("Invalid state: Instance marked as pending while exiting a hostcall.");
            }
            Domain::Guest => {
                panic!("Invalid state: Instance marked as in guest code while exiting a hostcall.");
            }
            Domain::Hostcall => {
                *current_domain = Domain::Guest;
                None
            }
            Domain::Terminated => {
                // The instance was stopped in the hostcall we were executing.
                debug_assert!(!self.terminable.load(Ordering::SeqCst));
                Some(TerminationDetails::Remote)
            }
            Domain::Cancelled => {
                panic!("Invalid state: Instance marked as cancelled while exiting a hostcall.");
            }
        };

        // explicitly drop `current_domain` to release the lock before reaching testpoint
        std::mem::drop(current_domain);

        #[cfg(feature = "concurrent_testpoints")]
        self.lock_testpoints
            .instance_exiting_hostcall_after_domain_change
            .check();

        res
    }

    pub fn schedule(&self, tid: pthread_t) {
        *self.thread_id.lock().unwrap() = Some(tid);
        self.tid_change_notifier.notify_all();
    }

    pub fn deschedule(&self) {
        *self.thread_id.lock().unwrap() = None;
        self.tid_change_notifier.notify_all();

        // If a guest is being descheduled, this lock is load-bearing in two ways:
        // * If a KillSwitch is in flight and already holds `execution_domain`, we must wait for
        // it to complete. This prevents a SIGALRM from being sent at some point later in host
        // execution.
        // * If a KillSwitch has aqcuired `terminable`, but not `execution_domain`, we may win the
        // race for this lock. We don't know when the KillSwitch will try to check
        // `execution_domain`. Holding the lock, we can update it to `Terminated` - this reflects
        // that the instance has exited, but also signals that the KillSwitch should take no
        // effect.
        //
        // This must occur *after* notifing `tid_change_notifier` so that we indicate to a
        // `KillSwitch` that the instance was actually descheduled, if it was terminating a guest.
        //
        // If any other state is being descheduled, either the instance faulted in another domain,
        // or a hostcall called `yield`, and we must preserve the `Hostcall` domain, so don't
        // change it.
        let mut execution_domain = self.execution_domain.lock().unwrap();
        if let Domain::Guest = *execution_domain {
            *execution_domain = Domain::Terminated;
        }
    }
}

/// Instance execution domains.
///
/// This enum allow us to distinguish how to appropriately terminate an instance.
///
/// We can signal in guest code, but must avoid signalling in host code lest we interrupt some
/// function operating on guest/host shared memory, and invalidate invariants. For example,
/// interrupting in the middle of a resize operation on a `Vec` could be extremely dangerous.
#[derive(Debug, PartialEq)]
pub enum Domain {
    /// Represents an instance that is not currently running.
    Pending,
    /// Represents an instance that is executing guest code.
    Guest,
    /// Represents an instance that is executing host code.
    Hostcall,
    /// Represents an instance that has been signalled to terminate while running code.
    Terminated,
    /// Represents an instance that has been cancelled before it began running code.
    Cancelled,
}

/// An object that can be used to terminate an instance's execution from a separate thread.
///
/// A weak reference to the instance's kill state is used so that a `KillSwitch` can have an
/// arbitrary lifetime unrelated to the [`Instance`](struct.Instance.html) it will terminate.
pub struct KillSwitch {
    state: Weak<KillState>,
}

/// A successful attempt to terminate an [`Instance`](struct.Instance.html).
///
/// See [`KillSwitch::terminate`](struct.KillSwitch.html#method.terminate) for more information.
#[derive(Debug, PartialEq)]
pub enum KillSuccess {
    /// A `SIGALRM` was sent to the instance.
    Signalled,
    /// The guest is in a hostcall and cannot currently be signalled to terminate safely.
    ///
    /// We have indicated the instance should terminate when it completes its hostcall.
    Pending,
    /// The instance was terminated before it started running.
    Cancelled,
}

/// A failed attempt to terminate an [`Instance`](struct.Instance.html).
///
/// See [`KillSwitch::terminate`](struct.KillSwitch.html#method.terminate) for more information.
#[derive(Debug, PartialEq)]
pub enum KillError {
    /// The instance cannot be terminated.
    ///
    /// This means that the instance has already been terminated by another killswitch, or was
    /// signalled to terminate in some other manner.
    NotTerminable,
    /// The associated instance is no longer valid.
    ///
    /// This usually means the instance exited by fault or by normal return.
    Invalid,
}

type KillResult = Result<KillSuccess, KillError>;

impl KillSwitch {
    pub(crate) fn new(state: Weak<KillState>) -> Self {
        KillSwitch { state }
    }

    /// Signal the instance associated with this `KillSwitch` to stop, if possible.
    ///
    /// The returned `Result` only describes the behavior taken by this function, not necessarily
    /// what caused the associated instance to stop.
    ///
    /// As an example, if a `KillSwitch` fires, sending a SIGALRM to an instance at the same
    /// moment it begins handling a SIGSEGV which is determined to be fatal, the instance may
    /// stop with `State::Faulted` before actually _handling_ the SIGALRM we'd send here. So the
    /// host code will see `State::Faulted` as an instance state, where `KillSwitch::terminate`
    /// would return `Ok(KillSuccess::Signalled)`.
    pub fn terminate(&self) -> KillResult {
        // Get the underlying kill state. If this fails, it means the instance exited and was
        // discarded, so we can not terminate.
        let state = self.state.upgrade().ok_or(KillError::Invalid)?;

        #[cfg(feature = "concurrent_testpoints")]
        state
            .lock_testpoints
            .kill_switch_before_disabling_termination
            .check();

        // Attempt to take the flag indicating the instance may terminate
        let terminable = state.terminable.swap(false, Ordering::SeqCst);
        if !terminable {
            #[cfg(feature = "concurrent_testpoints")]
            state
                .lock_testpoints
                .kill_switch_after_forbidden_termination
                .check();

            return Err(KillError::NotTerminable);
        }

        #[cfg(feature = "concurrent_testpoints")]
        state
            .lock_testpoints
            .kill_switch_after_acquiring_termination
            .check();

        // we got it! we can signal the instance.
        //
        // Now check what domain the instance is in. We can signal in guest code, but want
        // to avoid signalling in host code lest we interrupt some function operating on
        // guest/host shared memory, and invalidate invariants. For example, interrupting
        // in the middle of a resize operation on a `Vec` could be extremely dangerous.
        //
        // Hold this lock through all signalling logic to prevent the instance from
        // switching domains (and invalidating safety of whichever mechanism we choose here)
        let mut execution_domain = state.execution_domain.lock().unwrap();

        #[cfg(feature = "concurrent_testpoints")]
        state
            .lock_testpoints
            .kill_switch_after_acquiring_domain_lock
            .check();

        let result = match *execution_domain {
            Domain::Guest => {
                #[cfg(feature = "concurrent_testpoints")]
                state
                    .lock_testpoints
                    .kill_switch_before_guest_termination
                    .check();

                let mut curr_tid = state.thread_id.lock().unwrap();
                // we're in guest code, so we can just send a signal.
                if let Some(thread_id) = *curr_tid {
                    #[cfg(feature = "concurrent_testpoints")]
                    state.lock_testpoints.kill_switch_before_guest_alarm.check();

                    unsafe {
                        pthread_kill(thread_id, SIGALRM);
                    }

                    #[cfg(feature = "concurrent_testpoints")]
                    state.lock_testpoints.kill_switch_after_guest_alarm.check();

                    // wait for the SIGALRM handler to deschedule the instance
                    //
                    // this should never actually loop, which would indicate the instance
                    // was moved to another thread, or we got spuriously notified.
                    while curr_tid.is_some() {
                        curr_tid = state.tid_change_notifier.wait(curr_tid).unwrap();
                    }
                    *execution_domain = Domain::Terminated;
                    Ok(KillSuccess::Signalled)
                } else {
                    panic!("logic error: instance is terminable but not actually running.");
                }
            }
            Domain::Hostcall => {
                #[cfg(feature = "concurrent_testpoints")]
                state
                    .lock_testpoints
                    .kill_switch_before_hostcall_termination
                    .check();

                // the guest is in a hostcall, so the only thing we can do is indicate it
                // should terminate and wait.
                *execution_domain = Domain::Terminated;
                Ok(KillSuccess::Pending)
            }
            Domain::Pending => {
                // the guest has not started, so we indicate that it has been cancelled.
                *execution_domain = Domain::Cancelled;
                Ok(KillSuccess::Cancelled)
            }
            Domain::Cancelled | Domain::Terminated => {
                #[cfg(feature = "concurrent_testpoints")]
                state
                    .lock_testpoints
                    .kill_switch_before_terminated_termination
                    .check();

                // Something else (another KillSwitch?) has already signalled this instance
                // to exit, either when it has completed its hostcall, or when it starts.
                // In either case, there is nothing to do here.
                Err(KillError::NotTerminable)
            }
        };
        #[cfg(feature = "concurrent_testpoints")]
        state
            .lock_testpoints
            .kill_switch_before_releasing_domain
            .check();

        // explicitly drop the lock to be clear about how long we want to hold this lock, which is
        // until all signalling is complete.
        mem::drop(execution_domain);

        #[cfg(feature = "concurrent_testpoints")]
        state
            .lock_testpoints
            .kill_switch_after_releasing_domain
            .check();

        result
    }
}
