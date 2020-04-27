use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// A `SyncWaiter` is a handle to coordinate or execute some testing with respect to its
/// corresponding `Syncpoint`.
///
/// A `SyncWaiter` corresponds to one `Syncpoint` and is created by its `wait_at`. A `SyncWaiter`
/// can only be waited at once, which is why both `wait_and_then` and `wait` consume the waiter.
pub struct SyncWaiter {
    arrived: Arc<AtomicBool>,
    proceed: Arc<AtomicBool>,
}

impl SyncWaiter {
    /// Wait for the corresponding `Syncpoint` to be reached, then continue.
    pub fn wait(self) {
        self.wait_and_then(|| {})
    }

    /// Wait for the corresponding `Syncpoint` to be reached, run the provided function, then
    /// continue. This is useful for causing race conditions where a `Syncpoint` guarantees the
    /// program under test has stopped at a location of interest, so the function provided to
    /// `wait_and_then` is free to "race" with complete determinism.
    pub fn wait_and_then<U, F: FnOnce() -> U>(self, f: F) -> U {
        let resumption = self.pause();

        let res = f();

        resumption.resume();

        res
    }

    /// Wait for the corresponding `Syncpoint` to be reached, then return without permitting it to
    /// continue. *If you do not resume from this `SyncWaiter` at some point you will likely
    /// deadlock your test!*
    #[must_use]
    pub fn pause(self) -> Self {
        while !self.arrived.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(10));
        }

        self
    }

    /// Resume this `SyncWaiter`, consuming it as can have no further effect. A `SyncWaiter` may be
    /// resumed before it is reached, in which case this behaves similarly to having never called
    /// `wait_at()` on the corresponding `Syncpoint`.
    pub fn resume(self) {
        self.proceed.store(true, Ordering::SeqCst);
    }
}

/// A `Syncpoint` is a tool to coordinate testing at specific locations in Lucet.
///
/// When `lock_testpoints` are compiled in, lucet-runtime will `check` unconditionally, where by
/// default this is functionally a no-op. For `Syncpoint`s a test has indicated interest in, with
/// `wait_at`, `check` becomes blocking until the test allows continuation through the
/// corresponding `SyncWaiter` that `wait_at` constructed. This allows tests to be written that
/// check race conditions in a deterministic manner: the runtime can execute "enter a guest", be
/// blocked at a Syncpoint just before guest entry, and a test that termination is correct in this
/// otherwise-unlikely circumstance can be performed.
pub struct Syncpoint {
    arrived: Arc<AtomicBool>,
    proceed: Arc<AtomicBool>,
}

impl Syncpoint {
    pub fn new() -> Self {
        Self {
            arrived: Arc::new(AtomicBool::new(false)),
            proceed: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn wait_at(&self) -> SyncWaiter {
        let arrived = Arc::clone(&self.arrived);
        let proceed = Arc::clone(&self.proceed);

        proceed.store(false, Ordering::SeqCst);

        SyncWaiter { arrived, proceed }
    }

    pub fn check(&self) {
        self.arrived.store(true, Ordering::SeqCst);

        while !self.proceed.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

pub struct LockTestpoints {
    pub instance_after_clearing_current_instance: Syncpoint,
    pub instance_entering_guest_after_domain_change: Syncpoint,
    pub instance_entering_guest_before_domain_change: Syncpoint,
    pub instance_entering_hostcall_after_domain_change: Syncpoint,
    pub instance_entering_hostcall_before_domain_change: Syncpoint,
    pub instance_exiting_guest_after_domain_change: Syncpoint,
    pub instance_exiting_guest_before_acquiring_terminable: Syncpoint,
    pub instance_exiting_guest_without_terminable: Syncpoint,
    pub instance_exiting_hostcall_after_domain_change: Syncpoint,
    pub instance_exiting_hostcall_before_domain_change: Syncpoint,
    pub kill_switch_after_acquiring_domain_lock: Syncpoint,
    pub kill_switch_after_acquiring_termination: Syncpoint,
    pub kill_switch_after_forbidden_termination: Syncpoint,
    pub kill_switch_after_guest_alarm: Syncpoint,
    pub kill_switch_after_releasing_domain: Syncpoint,
    pub kill_switch_before_disabling_termination: Syncpoint,
    pub kill_switch_before_guest_alarm: Syncpoint,
    pub kill_switch_before_guest_termination: Syncpoint,
    pub kill_switch_before_hostcall_termination: Syncpoint,
    pub kill_switch_before_releasing_domain: Syncpoint,
    pub kill_switch_before_terminated_termination: Syncpoint,
    pub signal_handler_after_disabling_termination: Syncpoint,
    pub signal_handler_after_unable_to_disable_termination: Syncpoint,
    pub signal_handler_before_checking_alarm: Syncpoint,
    pub signal_handler_before_disabling_termination: Syncpoint,
    pub signal_handler_before_returning: Syncpoint,
}

impl LockTestpoints {
    pub fn new() -> Self {
        LockTestpoints {
            instance_after_clearing_current_instance: Syncpoint::new(),
            instance_entering_guest_after_domain_change: Syncpoint::new(),
            instance_entering_guest_before_domain_change: Syncpoint::new(),
            instance_entering_hostcall_after_domain_change: Syncpoint::new(),
            instance_entering_hostcall_before_domain_change: Syncpoint::new(),
            instance_exiting_guest_after_domain_change: Syncpoint::new(),
            instance_exiting_guest_before_acquiring_terminable: Syncpoint::new(),
            instance_exiting_guest_without_terminable: Syncpoint::new(),
            instance_exiting_hostcall_after_domain_change: Syncpoint::new(),
            instance_exiting_hostcall_before_domain_change: Syncpoint::new(),
            kill_switch_after_acquiring_domain_lock: Syncpoint::new(),
            kill_switch_after_acquiring_termination: Syncpoint::new(),
            kill_switch_after_forbidden_termination: Syncpoint::new(),
            kill_switch_after_guest_alarm: Syncpoint::new(),
            kill_switch_after_releasing_domain: Syncpoint::new(),
            kill_switch_before_disabling_termination: Syncpoint::new(),
            kill_switch_before_guest_alarm: Syncpoint::new(),
            kill_switch_before_guest_termination: Syncpoint::new(),
            kill_switch_before_hostcall_termination: Syncpoint::new(),
            kill_switch_before_releasing_domain: Syncpoint::new(),
            kill_switch_before_terminated_termination: Syncpoint::new(),
            signal_handler_after_disabling_termination: Syncpoint::new(),
            signal_handler_after_unable_to_disable_termination: Syncpoint::new(),
            signal_handler_before_checking_alarm: Syncpoint::new(),
            signal_handler_before_disabling_termination: Syncpoint::new(),
            signal_handler_before_returning: Syncpoint::new(),
        }
    }
}
