use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub struct SyncWaiter {
    arrived: Arc<AtomicBool>,
    proceed: Arc<AtomicBool>,
}

impl SyncWaiter {
    pub fn wait(&self) {
        while !self.arrived.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(10));
        }

        self.proceed.store(true, Ordering::SeqCst);
    }

    pub fn wait_and_then<U, F: FnOnce() -> U>(&self, f: F) -> U {
        while !self.arrived.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(10));
        }

        let res = f();

        self.proceed.store(true, Ordering::SeqCst);

        res
    }
}

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
