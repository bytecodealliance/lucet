use lucet_module::lucet_signature;
use lucet_runtime::lucet_hostcall;
use lucet_runtime::vmctx::{lucet_vmctx, Vmctx};
use lucet_runtime_internals::module::{
    FunctionPointer, HeapSpec, MockExportBuilder, MockModuleBuilder, Module,
};
use lucet_wasi_sdk::{CompileOpts, Lucetc};
use lucetc::{Bindings, LucetcOpts, OptLevel};
use std::path::Path;
use std::sync::Arc;

fn wasi_bindings() -> Bindings {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../lucet-wasi/bindings.json");
    Bindings::from_file(&path).unwrap()
}

pub fn compile_hello<P: AsRef<Path>>(so_file: P, opt_level: OptLevel) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("guests/hello.c");
    let wasm_build = Lucetc::new(&[&path])
        .with_cflag("-Wall")
        .with_cflag("-Werror")
        .with_bindings(wasi_bindings())
        .with_opt_level(opt_level);

    wasm_build.build(&so_file).unwrap();
}

pub fn null_mock() -> Arc<dyn Module> {
    extern "C" fn f(_vmctx: *mut lucet_vmctx) {}

    MockModuleBuilder::new()
        .with_export_func(MockExportBuilder::new(
            "f",
            FunctionPointer::from_usize(f as usize),
        ))
        .build()
}

pub fn large_dense_heap_mock(heap_kb: usize) -> Arc<dyn Module> {
    extern "C" fn f(_vmctx: *mut lucet_vmctx) {}

    let heap_len = heap_kb * 1024;

    let heap_spec = HeapSpec {
        reserved_size: heap_len as u64,
        guard_size: 4 * 1024 * 1024,
        initial_size: heap_len as u64,
        max_size: None,
    };

    let mut heap = vec![0x00; heap_len];
    (0..heap_len).into_iter().for_each(|i| {
        heap[i] = (i % 256) as u8;
    });

    MockModuleBuilder::new()
        .with_export_func(MockExportBuilder::new(
            "f",
            FunctionPointer::from_usize(f as usize),
        ))
        .with_initial_heap(heap.as_slice())
        .with_heap_spec(heap_spec)
        .build()
}

pub fn large_sparse_heap_mock(heap_kb: usize, stride: usize) -> Arc<dyn Module> {
    extern "C" fn f(_vmctx: *mut lucet_vmctx) {}

    let heap_len = heap_kb * 1024;

    let heap_spec = HeapSpec {
        reserved_size: heap_len as u64,
        guard_size: 4 * 1024 * 1024,
        initial_size: heap_len as u64,
        max_size: None,
    };

    let mut heap = vec![0x00; heap_len];

    // fill every `stride`th page with data
    (0..heap_len)
        .into_iter()
        .step_by(4096 * stride)
        .for_each(|base| {
            for i in base..base + 4096 {
                heap[i] = (i % 256) as u8;
            }
        });

    MockModuleBuilder::new()
        .with_export_func(MockExportBuilder::new(
            "f",
            FunctionPointer::from_usize(f as usize),
        ))
        .with_initial_heap(heap.as_slice())
        .with_heap_spec(heap_spec)
        .build()
}

pub fn fib_mock() -> Arc<dyn Module> {
    extern "C" fn f(_vmctx: *mut lucet_vmctx) {
        fn fib(n: u32) -> u32 {
            if n == 0 {
                0
            } else if n == 1 {
                1
            } else {
                fib(n - 1) + fib(n - 2)
            }
        }
        assert_eq!(fib(25), 75025);
    }

    MockModuleBuilder::new()
        .with_export_func(MockExportBuilder::new(
            "f",
            FunctionPointer::from_usize(f as usize),
        ))
        .build()
}

pub fn many_args_mock() -> Arc<dyn Module> {
    extern "C" fn f(
        _vmctx: *mut lucet_vmctx,
        _: u8,
        _: u16,
        _: u32,
        _: u64,
        _: f32,
        _: f64,
        _: u8,
        _: u16,
        _: u32,
        _: u64,
        _: f32,
        _: f64,
        _: u8,
        _: u16,
        _: u32,
        _: u64,
        _: f32,
        _: f64,
        _: u8,
        _: u16,
        _: u32,
        _: u64,
        _: f32,
        _: f64,
        _: u8,
        _: u16,
        _: u32,
        _: u64,
        _: f32,
        _: f64,
        _: u8,
        _: u16,
        _: u32,
        _: u64,
        _: f32,
        _: f64,
        _: u8,
        _: u16,
        _: u32,
        _: u64,
        _: f32,
        _: f64,
        _: u8,
        _: u16,
        _: u32,
        _: u64,
        _: f32,
        _: f64,
        _: u8,
        _: u16,
        _: u32,
        _: u64,
        _: f32,
        _: f64,
        _: u8,
        _: u16,
        _: u32,
        _: u64,
        _: f32,
        _: f64,
        _: u8,
        _: u16,
        _: u32,
        _: u64,
        _: f32,
        _: f64,
    ) {
    }

    MockModuleBuilder::new()
        .with_export_func(
            MockExportBuilder::new("f", FunctionPointer::from_usize(f as usize)).with_sig(
                lucet_signature!(
                    (
                        I32, I32, I32, I64, F32, F64,
                        I32, I32, I32, I64, F32, F64,
                        I32, I32, I32, I64, F32, F64,
                        I32, I32, I32, I64, F32, F64,
                        I32, I32, I32, I64, F32, F64,
                        I32, I32, I32, I64, F32, F64,
                        I32, I32, I32, I64, F32, F64,
                        I32, I32, I32, I64, F32, F64,
                        I32, I32, I32, I64, F32, F64,
                        I32, I32, I32, I64, F32, F64,
                        I32, I32, I32, I64, F32, F64
                    ) -> ()
                ),
            ),
        )
        .build()
}

pub fn hostcalls_mock() -> Arc<dyn Module> {
    #[lucet_hostcall]
    #[inline(never)]
    #[no_mangle]
    pub unsafe extern "C" fn hostcall_wrapped(
        vmctx: &mut Vmctx,
        x1: u64,
        x2: u64,
        x3: u64,
        x4: u64,
        x5: u64,
        x6: u64,
        x7: u64,
        x8: u64,
        x9: u64,
        x10: u64,
        x11: u64,
        x12: u64,
        x13: u64,
        x14: u64,
        x15: u64,
        x16: u64,
    ) -> () {
        vmctx.heap_mut()[0] =
            (x1 + x2 + x3 + x4 + x5 + x6 + x7 + x8 + x9 + x10 + x11 + x12 + x13 + x14 + x15 + x16)
                as u8;
        assert_eq!(vmctx.heap()[0], 136);
    }

    #[inline(never)]
    #[no_mangle]
    pub unsafe extern "C" fn hostcall_raw(
        vmctx: *mut lucet_vmctx,
        x1: u64,
        x2: u64,
        x3: u64,
        x4: u64,
        x5: u64,
        x6: u64,
        x7: u64,
        x8: u64,
        x9: u64,
        x10: u64,
        x11: u64,
        x12: u64,
        x13: u64,
        x14: u64,
        x15: u64,
        x16: u64,
    ) {
        let vmctx = Vmctx::from_raw(vmctx);
        vmctx.heap_mut()[0] =
            (x1 + x2 + x3 + x4 + x5 + x6 + x7 + x8 + x9 + x10 + x11 + x12 + x13 + x14 + x15 + x16)
                as u8;
        assert_eq!(vmctx.heap()[0], 136);
    }

    unsafe extern "C" fn wrapped(vmctx: *mut lucet_vmctx) {
        for _ in 0..1000 {
            hostcall_wrapped(vmctx, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16);
        }
    }

    unsafe extern "C" fn raw(vmctx: *mut lucet_vmctx) {
        for _ in 0..1000 {
            hostcall_raw(vmctx, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16);
        }
    }

    MockModuleBuilder::new()
        .with_export_func(MockExportBuilder::new(
            "wrapped",
            FunctionPointer::from_usize(wrapped as usize),
        ))
        .with_export_func(MockExportBuilder::new(
            "raw",
            FunctionPointer::from_usize(raw as usize),
        ))
        .build()
}
