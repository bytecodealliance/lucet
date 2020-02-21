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
//!   - terminable: `true`
//!   - execution_domain: `Pending`
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
//!
//! FIXME KTM 2020-02-13: Some details in the documentation above will need to change.

use libc::{pthread_kill, pthread_t, SIGALRM};
use std::mem;
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

/// Enter a guest region.
///
/// This function is called by `lucet_context_activate` before entering a region of guest code.
///
/// This function is responsible for setting the execution domain in the instance's
/// [`KillState`](struct.KillState.html), so that we can appropriately signal the instance to
/// terminate if needed. If a pending instance was signalled before it started executing guest
/// code, we will terminate the instance and swap back to its host context.
///
/// # Safety
///
/// This function will call [Instance::terminate](struct.Instance.html#method.terminate) if the
/// execution domain is [`Domain::Cancelled`](enum.Domain.html). In that case, this function will
/// not return, and we will swap back to the host context without unwinding.
///
/// This function will panic if the `Instance`'s execution domain is marked as currently executing
/// guest code, currently in a hostcall, or as cancelled.  Any of these domains mean that something
/// has gone seriously wrong, and leaving the execution domain mutex in a poisoned state is the
/// least of our concerns.
pub unsafe extern "C" fn enter_guest_region(instance: *mut Instance) {
    let mut current_domain = (*instance).kill_state.execution_domain.lock().unwrap();
    match *current_domain {
        Domain::Pending => {
            // All systems go!  We are about to start executing a guest. Set the execution domain
            // accordingly, and proceed to the guest region.
            *current_domain = Domain::Guest
        }
        Domain::Guest => {
            // This is an error because it suggests we might send a SIGALRM before we've indicated
            // that it safe to do so.
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
        #[allow(unreachable_code)]
        Domain::Cancelled => {
            // Our `Instance` was terminated before it began executing guest code, so we should not
            // enter the guest region. Instead, we will terminate the `Instance` and swap back to
            // the host context. We explicitly explicitly release our lock because
            // `Instance::terminate` will never return.
            mem::drop(current_domain);
            (*instance).terminate(TerminationDetails::Remote);
            unreachable!(); // `Instance::terminate` should never never return.
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
    let mut current_domain = (*instance).kill_state.execution_domain.lock().unwrap();
    match *current_domain {
        Domain::Pending => {
            panic!("Invalid state: Instance marked as pending while exiting a guest region.");
        }
        Domain::Guest => {
            // If we are here, we finished executing the code in our guest region as expected!
            // Mark that we are pending once more, and then exit the guest context.
            *current_domain = Domain::Pending;
        }
        Domain::Hostcall => {
            panic!("Invalid state: Instance marked as in a hostcall while exiting a guest region.");
        }
        Domain::Terminated => {
            // Something else has signalled us to terminate, so it's not safe to actually exit the
            // guest context yet. Because this is called when exiting a guest context, the
            // termination mechanism will be a signal, delivered at some point (hopefully soon!).
            // Further, because the termination mechanism will be a signal, we are constrained to
            // only signal-safe behavior.
            //
            // For now, release our lock on the execution domain, and hang indefinitely, waiting
            // for the sigalrm to arrive. So yes, clippy, we know that this loop is empty :)
            mem::drop(current_domain);
            #[allow(clippy::empty_loop)]
            loop {}
        }
        Domain::Cancelled => {
            panic!("Invalid state: Instance marked as cancelled while exiting a guest region.");
        }
    };
}

impl Default for KillState {
    fn default() -> Self {
        Self {
            tid_change_notifier: Condvar::new(),
            execution_domain: Mutex::new(Domain::Pending),
            thread_id: Mutex::new(None),
        }
    }
}

impl KillState {
    /// Construct a new `KillState`.
    pub fn new() -> Self {
        Default::default()
    }

    /// Set the execution domain to signify that we are currently executing a hostcall.
    ///
    /// This method will panic if the execution domain is currently marked as anything but
    /// `Domain::Guest`, because any other domain means that we have somehow entered an invalid
    /// state.
    ///
    /// This method will also panic if the mutex on the execution domain has been poisoned.
    pub fn begin_hostcall(&self) {
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
        let mut current_domain = self.execution_domain.lock().unwrap();
        match *current_domain {
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
            Domain::Terminated => Some(TerminationDetails::Remote),
            Domain::Cancelled => {
                panic!("Invalid state: Instance marked as cancelled while exiting a hostcall.");
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

    /// Check if the execution domain is currently locked.
    ///
    /// This uses [`Mutex::try_lock`](std/sync/struct.Mutex.html#method.try_lock), to see if
    /// acquiring a lock of the execution domain's mutex, would block the current thread. If
    /// so, return `true`. If the mutex is not locked, or is in a poisoned state, return false.
    ///
    /// This is used by an [`Instance`](struct.KillState.html)'s signal handler to check that a
    /// `SIGALRM` received during execution came from a kill switch, rather than some other source.
    pub(crate) fn domain_is_locked(&self) -> bool {
        use std::sync::TryLockError::WouldBlock;
        if let Err(WouldBlock) = self.execution_domain.try_lock() {
            true
        } else {
            false
        }
    }

    /// Acquire a lock on the execution domain.
    ///
    /// This is used by an [`Instance`](struct.KillState.html)'s signal handler to disable
    /// termination before switching to the host context, for signals _other than_ a `SIGALRM`
    /// that came from a kill switch, rather than some other source.
    pub(crate) fn execution_domain(
        &self,
    ) -> std::sync::LockResult<std::sync::MutexGuard<'_, Domain>> {
        self.execution_domain.lock()
    }
}

/// Instance execution domains.
///
/// This enum allow us to distinguish what an appropriate mechanism to signal termination is.
#[derive(Debug, PartialEq)]
pub enum Domain {
    /// Represents an instance that is not currently running.
    /// FIXME KTM 2020-02-20: Maybe we should call this `Ready` or `Paused`.
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
pub struct KillSwitch {
    state: Weak<KillState>,
}

#[derive(Debug, PartialEq)]
pub enum KillSuccess {
    Signalled,
    Pending,
    Cancelled,
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
                    // If we are here, we probably faulted without updating the execution domain.
                    panic!("logic error: instance is terminable but not actually running.");
                }
            }
            Domain::Hostcall => {
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
                // Something else (another KillSwitch?) has already signalled this instance
                // to exit, either when it has completed its hostcall, or when it starts.
                // In either case, there is nothing to do here.
                Err(KillError::NotTerminable)
            }
        };
        // explicitly drop the lock to be clear about how long we want to hold this lock, which is
        // until all signalling is complete.
        mem::drop(execution_domain);
        result
    }
}
