//! The `execution` module contains state for an instance's execution, and exposes functions
//! building that state into something appropriate for safe use externally.
//!
//! So far as state tracked in this module is concerned, there are two key items: "terminability"
//! and "execution domain".
//!
//! ## Terminability
//! This specifically answers the question "is it safe to initiate termination of this instance
//! right now?". An instance becomes terminable when it begins executing, and stops being
//! terminable when it is terminated, or when it stops executing. Termination does not directly map
//! to the idea of guest code currently executing on a processor, because termination can occur
//! during host code, or while a guest has yielded execution. As a result, termination can only be
//! treated as a best-effort to deschedule a guest, and is typically quick when it occurs during
//! guest code execution, or immediately upon resuming execution of guest code (exiting host code,
//! or resuming a yielded instance).
//!
//! ## Execution Domain
//! Execution domains allow us to distinguish what an appropriate mechanism to signal termination
//! is. This means that changing of an execution domain must be atomic - it would be an error to
//! read the current execution domain, continue with that domain to determine temination, and
//! simultaneously for execution to continue possibly into a different execution domain. For
//! example, beginning termination directly at the start of a hostcall, where sending `SIGALRM` may
//! be appropriate, while the domain switches to `Hostcall` and is no longer appropriate for
//! signalling, would be an error.
//!
//! ## Instance Lifecycle and `KillState`
//!
//! And now we can enumerate interleavings of execution and timeout, to see the expected state at
//! possible points of interest in an instance's lifecycle:
//!
//! * `Instance created`
//!   - terminable: `false`
//!   - execution_domain: `Guest`
//! * `Instance::run called`
//!   - terminable: `true`
//!   - execution_domain: `Guest`
//! * `Instance::run executing`
//!   - terminable: `true, or false`
//!   - execution_domain: `Guest, Hostcall, or Terminated`
//!   - `execution_domain` will only be `Guest` when executing guest code, only be `Hostcall` when
//!   executing a hostcall, but may also be `Terminated` while in a hostcall to indicate that it
//!   should exit when the hostcall completes.
//!   - `terminable` will be false if and only if `execution_domain` is `Terminated`.
//! * `Instance::run returns`
//!   - terminable: `false`
//!   - execution_domain: `Guest, Hostcall, or Terminated`
//!   - `execution_domain` will be `Guest` when the initial guest function returns, `Hostcall` when
//!   terminated by `lucet_hostcall_terminate!`, and `Terminated` when exiting due to a termination
//!   request.
//! * `Guest function executing`
//!   - terminable: `true`
//!   - execution_domain: `Guest`
//! * `Guest function returns`
//!   - terminable: `true`
//!   - execution_domain: `Guest`
//! * `Hostcall called`
//!   - terminable: `true`
//!   - execution_domain: `Hostcall`
//! * `Hostcall executing`
//!   - terminable: `true`
//!   - execution_domain: `Hostcall, or Terminated`
//!   - `execution_domain` will typically be `Hostcall`, but may be `Terminated` if termination of
//!   the instance is requested during the hostcall.
//!   - `terminable` will be false if and only if `execution_domain` is `Terminated`.
//! * `Hostcall yields`
//!   - This is a specific point in "Hostcall executing" and has no further semantics.
//! * `Hostcall resumes`
//!   - This is a specific point in "Hostcall executing" and has no further semantics.
//! * `Hostcall returns`
//!   - terminable: `true`
//!   - execution_domain: `Guest`
//!   - `execution_domain` may be `Terminated` before returning, in which case `terminable` will be
//!   false, but the hostcall would then exit. If a hostcall successfully returns to its caller it
//!   was not terminated, so the only state an instance will have after returning from a hostcall
//!   will be that it's executing terminable guest code.

use libc::{pthread_kill, pthread_t, SIGALRM};
use std::mem;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Condvar, Mutex, Weak};

use crate::instance::{Instance, TerminationDetails};

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
/// made. In fact, it is possible for a kill signal to be successfully sent and still never
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
}

pub unsafe extern "C" fn exit_guest_region(instance: *mut Instance) {
    let terminable = (*instance)
        .kill_state
        .terminable
        .swap(false, Ordering::SeqCst);
    if !terminable {
        // Something else has taken the terminable flag, so it's not safe to actually exit a
        // guest context yet. Because this is called when exiting a guest context, the
        // termination mechanism will be a signal, delivered at some point (hopefully soon!).
        // Further, because the termination mechanism will be a signal, we are constrained to
        // only signal-safe behavior.
        //
        // For now, hang indefinitely, waiting for the sigalrm to arrive.

        loop {}
    }
}

impl KillState {
    pub fn new() -> KillState {
        KillState {
            terminable: AtomicBool::new(false),
            tid_change_notifier: Condvar::new(),
            execution_domain: Mutex::new(Domain::Guest),
            thread_id: Mutex::new(None),
        }
    }

    pub fn is_terminable(&self) -> bool {
        self.terminable.load(Ordering::SeqCst)
    }

    pub fn enable_termination(&self) {
        self.terminable.store(true, Ordering::SeqCst);
    }

    pub fn disable_termination(&self) {
        self.terminable.store(false, Ordering::SeqCst);
    }

    pub fn terminable_ptr(&self) -> *const AtomicBool {
        &self.terminable as *const AtomicBool
    }

    pub fn begin_hostcall(&self) {
        // Lock the current execution domain, so we can update to `Hostcall`.
        let mut current_domain = self.execution_domain.lock().unwrap();
        match *current_domain {
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
        }
    }

    pub fn end_hostcall(&self) -> Option<TerminationDetails> {
        let mut current_domain = self.execution_domain.lock().unwrap();
        match *current_domain {
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
                std::mem::drop(current_domain);
                Some(TerminationDetails::Remote)
            }
        }
    }

    pub fn schedule(&self, tid: pthread_t) {
        *self.thread_id.lock().unwrap() = Some(tid);
        self.tid_change_notifier.notify_all();
    }

    pub fn deschedule(&self) {
        *self.thread_id.lock().unwrap() = None;
        self.tid_change_notifier.notify_all();
    }
}

pub enum Domain {
    Guest,
    Hostcall,
    Terminated,
}

/// An object that can be used to terminate an instance's execution from a separate thread.
pub struct KillSwitch {
    state: Weak<KillState>,
}

#[derive(Debug, PartialEq)]
pub enum KillSuccess {
    Signalled,
    Pending,
}

#[derive(Debug, PartialEq)]
pub enum KillError {
    NotTerminable,
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
        // discarded, so we can't terminate.
        let state = self.state.upgrade().ok_or(KillError::NotTerminable)?;

        // Attempt to take the flag indicating the instance may terminate
        let terminable = state.terminable.swap(false, Ordering::SeqCst);
        if !terminable {
            return Err(KillError::NotTerminable);
        }

        // we got it! we can signal the instance.

        // Now check what domain the instance is in. We can signal in guest code, but want
        // to avoid signalling in host code lest we interrupt some function operating on
        // guest/host shared memory, and invalidate invariants. For example, interrupting
        // in the middle of a resize operation on a `Vec` could be extremely dangerous.
        //
        // Hold this lock through all signalling logic to prevent the instance from
        // switching domains (and invalidating safety of whichever mechanism we choose here)
        let mut execution_domain = state.execution_domain.lock().unwrap();

        let result = match *execution_domain {
            Domain::Guest => {
                let mut curr_tid = state.thread_id.lock().unwrap();
                // we're in guest code, so we can just send a signal.
                if let Some(thread_id) = *curr_tid {
                    unsafe {
                        pthread_kill(thread_id, SIGALRM);
                    }

                    // wait for the SIGALRM handler to deschedule the instance
                    //
                    // this should never actually loop, which would indicate the instance
                    // was moved to another thread, or we got spuriously notified.
                    while curr_tid.is_some() {
                        curr_tid = state.tid_change_notifier.wait(curr_tid).unwrap();
                    }
                    Ok(KillSuccess::Signalled)
                } else {
                    panic!("logic error: instance is terminable but not actually running.");
                }
            }
            Domain::Hostcall => {
                // the guest is in a hostcall, so the only thing we can do is indicate it
                // should terminate and wait.
                *execution_domain = Domain::Terminated;
                Ok(KillSuccess::Pending)
            }
            Domain::Terminated => {
                // Something else (another KillSwitch?) has already signalled this instance
                // to exit when it has completed its hostcall. Nothing to do here.
                Err(KillError::NotTerminable)
            }
        };
        // explicitly drop the lock to be clear about how long we want to hold this lock, which is
        // until all signalling is complete.
        mem::drop(execution_domain);
        result
    }
}
