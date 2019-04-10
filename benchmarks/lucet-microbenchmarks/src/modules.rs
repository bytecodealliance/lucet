use lucet_runtime::vmctx::lucet_vmctx;
use lucet_runtime::Module;
use lucet_runtime_internals::module::MockModuleBuilder;
use lucet_wasi_sdk::{CompileOpts, Lucetc};
use lucetc::{Bindings, LucetcOpts};
use std::path::Path;
use std::sync::Arc;

fn wasi_bindings() -> Bindings {
    Bindings::from_file("../../lucet-wasi/bindings.json").unwrap()
}

pub fn compile_hello<P: AsRef<Path>>(so_file: P) {
    let wasm_build = Lucetc::new(&["guests/hello.c"])
        .print_output(true)
        .with_cflag("-Wall")
        .with_cflag("-Werror")
        .with_bindings(wasi_bindings());

    wasm_build.build(&so_file).unwrap();
}

pub fn null_mock() -> Arc<dyn Module> {
    extern "C" fn f(_vmctx: *mut lucet_vmctx) {}

    MockModuleBuilder::new()
        .with_export_func(b"f", f as *const extern "C" fn())
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
        .with_export_func(b"f", f as *const extern "C" fn())
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
        .with_export_func(b"f", f as *const extern "C" fn())
        .build()
}
