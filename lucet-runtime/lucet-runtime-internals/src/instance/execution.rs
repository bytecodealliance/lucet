//! The `execution` module contains state for an instance's execution, and exposes functions
//! building that state into something appropriate for safe use externally.
//!
//! So far as state tracked in this module is concerned, the key concept to understand is an
//! "execution domain".
//!
//! This is used to answer the question "is it safe to initiate termination of this instance right
//! now?". An instance can be terminated when it is created, and stops being terminable when it is
//! terminated, when it faults, or when it is dropped. An instance is terminable after it has been
//! reset, but any outstanding kill switches will no longer be valid.
//!
//! ## Execution Domain
//!
//! Termination does not directly map to the idea of guest code currently executing on a processor,
//! because termination can occur either during host code, or while a guest has yielded execution.
//!
//! As a result, termination can only be treated as a best-effort to deschedule a guest. This is
//! typically quick when it occurs during guest code execution, and otherwise happens immediately
//! upon resuming execution of guest code (exiting host code, or resuming a yielded instance).
//!
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
//! And now we can enumerate interleavings of execution and terminability, to see the expected
//! execution domain at possible points of interest in an instance's lifecycle:
//!
//! * `Instance created`
//!   - termination result: `Ok(KillSuccess::Cancelled)`
//!   - execution_domain: `Pending`
//! * `Instance::run executing`
//!   - termination result: `Ok(KillSuccess::Signalled)`, `Ok(KillSuccess::Pending)`, or
//!     `Err(KillError::NotTerminable)`
//!   - execution_domain: `Guest, Hostcall, or Terminated`
//!   - `execution_domain` will only be `Guest` when executing guest code, only be `Hostcall` when
//!     executing a hostcall, but may also be `Terminated` while in a hostcall to indicate that it
//!     should exit when the hostcall completes.
//!   - `KillSwitch::terminate` will succeed with `Signalled` when terminated while executing guest
//!     code, `Pending` when terminateed while executing a hostcall, and will fail with
//!     `NotTerminable` when the instance has already been terminated.
//! * `Instance::run returns`
//!   - execution_domain: `Guest, Hostcall, or Terminated`
//!   - `execution_domain` will be `Pending` when the initial guest function returns, `Hostcall`
//!     when terminated by `lucet_hostcall_terminate!`, and `Terminated` when exiting due to a
//!     termination request.
//! * `Guest function executing`
//!   - termination result: `Ok(KillSuccess::Signalled)`
//!   - execution_domain: `Guest`
//! * `Guest function returns`
//!   - termination result: `Ok(KillSuccess::Pending)`
//!   - execution_domain: `Guest`
//!   - If the guest function has already returned, the instance will be placed into the
//!     `Terminated` domain, and will not run any guest the next time it is run.
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
//! * `Instance::reset` or `Instance dropped`
//!   - termination result: `Err(KillError::Invalid)`
//!   - execution_domain: `Pending`
//!   - If an instance is reset, outstanding `KillSwitch` objects will no longer hold a valid
//!     reference to the instance. In that case, calls to `terminate` will return `Invalid`, and
//!     have no bearing on further execution.
//!
//! ## Further Reading
//!
//! For more information about kill state, execution domains, and instance termination, see
//! [`KillState`](struct.KillState.html), [`Domain`](enum.Domain.html), and
/// [`KillSwitch::terminate`](struct.KillSwitch.html#method.terminate), respectively.
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
/// made. In fact, it is possible for a kill signal to be successfully sent and still never be
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
/// guest code, currently in a hostcall, or as cancelled.  Attempting to enter an instance
/// currently in any of these domains mean that something has gone seriously wrong.
pub unsafe extern "C" fn enter_guest_region(instance: *mut Instance) {
    let mut current_domain = (*instance).kill_state.execution_domain.lock().unwrap();
    match *current_domain {
        Domain::Pending => {
            // All systems go! We are about to start executing a guest. Set the execution domain
            // accordingly, and then return so we can jump to the guest code.
            *current_domain = Domain::Guest
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
            (*instance).terminate(TerminationDetails::Remote);
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
            panic!("Invalid state: Instance marked as terminated while exiting a guest region.");
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
pub struct KillSwitch {
    /// A temporary, non-owning reference to the instance's kill state.
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
    /// signalled to terminate in some other manner, or possibly faulted during execution.
    NotTerminable,
    /// The associated instance is no longer valid.
    ///
    /// This usually means that instance already exited and was discarded, or that it was reset.
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
        // discarded, so we can't terminate.
        let state = self.state.upgrade().ok_or(KillError::Invalid)?;
        // Now check what domain the instance is in.
        //
        // Hold this lock through all signalling logic to prevent the instance from switching
        // domains (and invalidating safety of whichever mechanism we choose here)
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
                    *execution_domain = Domain::Terminated;
                    Ok(KillSuccess::Signalled)
                } else {
                    // If the execution domain is marked as `Guest`, but there is not a thread
                    // running that we wan signal, the guest most likely faulted.
                    Err(KillError::NotTerminable)
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
