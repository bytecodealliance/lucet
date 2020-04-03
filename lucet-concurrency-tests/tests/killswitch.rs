use lucet_runtime::{Error, Limits, Region, KillSuccess, TerminationDetails};
use std::sync::Arc;
use std::thread;

use lucet_module::FunctionPointer;
use lucet_runtime::MmapRegion;
use lucet_runtime_internals::module::Module;
use lucet_runtime_internals::module::{MockExportBuilder, MockModuleBuilder};
use lucet_runtime_internals::vmctx::lucet_vmctx;
use lucet_runtime_internals::lock_testpoints::Syncpoint;

static mut ENTERING_GUEST: Option<Syncpoint> = None;

pub fn mock_traps_module() -> Arc<dyn Module> {
    extern "C" fn onetwothree(_vmctx: *mut lucet_vmctx) -> std::os::raw::c_int {
        123
    }

    extern "C" fn infinite_loop(_vmctx: *mut lucet_vmctx) {
        unsafe {
            ENTERING_GUEST.as_ref().unwrap().check();
        }
        loop {}
    }

    extern "C" fn fatal(vmctx: *mut lucet_vmctx) {
        extern "C" {
            fn lucet_vmctx_get_heap(vmctx: *mut lucet_vmctx) -> *mut u8;
        }

        unsafe {
            let heap_base = lucet_vmctx_get_heap(vmctx);

            // Using the default limits, each instance as of this writing takes up 0x200026000 bytes
            // worth of virtual address space. We want to access a point beyond all the instances,
            // so that memory is unmapped. We assume no more than 16 instances are mapped
            // concurrently. This may change as the library, test configuration, linker, phase of
            // moon, etc change, but for now it works.
            *heap_base.offset(0x0002_0002_6000 * 16) = 0;
        }
    }

    extern "C" fn hit_sigstack_guard_page(vmctx: *mut lucet_vmctx) {
        extern "C" {
            fn lucet_vmctx_get_globals(vmctx: *mut lucet_vmctx) -> *mut u8;
        }

        unsafe {
            let globals_base = lucet_vmctx_get_globals(vmctx);

            // Using the default limits, the globals are a page; try to write just off the end
            *globals_base.offset(0x1000) = 0;
        }
    }

    MockModuleBuilder::new()
        .with_export_func(MockExportBuilder::new(
            "onetwothree",
            FunctionPointer::from_usize(onetwothree as usize),
        ))
        .with_export_func(MockExportBuilder::new(
            "infinite_loop",
            FunctionPointer::from_usize(infinite_loop as usize),
        ))
        .with_export_func(MockExportBuilder::new(
            "fatal",
            FunctionPointer::from_usize(fatal as usize),
        ))
        .with_export_func(MockExportBuilder::new(
            "hit_sigstack_guard_page",
            FunctionPointer::from_usize(hit_sigstack_guard_page as usize),
        ))
        .build()
}

// Test that a timeout that occurs in a signal handler is handled cleanly without signalling the
// Lucet embedder.
#[test]
fn timeout_in_signal_handler() {
    unsafe {
        ENTERING_GUEST = Some(Syncpoint::new());
    }
    let module = mock_traps_module();
    let region = MmapRegion::create(1, &Limits::default()).expect("region can be created");

    let mut inst = region
        .new_instance(module)
        .expect("instance can be created");

    let in_guest = unsafe { ENTERING_GUEST.as_ref().unwrap().wait_at() };

    let kill_switch = inst.kill_switch();

    let t = thread::Builder::new()
        .name("guest".to_owned())
        .spawn(move || {
            match inst.run("infinite_loop", &[]) {
                Err(Error::RuntimeTerminated(TerminationDetails::Remote)) => {
                    // this is what we want!
                }
                res => panic!("unexpected result: {:?}", res),
            }
        })
        .expect("can spawn a thread");

    let terminator = in_guest.wait_and_then(move || {
        thread::spawn(move || {
            assert_eq!(kill_switch.terminate(), Ok(KillSuccess::Signalled));
        })
    });

    t.join().unwrap();
    terminator.join().unwrap();
}
