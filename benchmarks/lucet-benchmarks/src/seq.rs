use crate::modules::*;
use criterion::Criterion;
use lucet_runtime::{DlModule, InstanceHandle, Limits, Module, Region, RegionCreate};
use lucet_wasi::WasiCtxBuilder;
use lucetc::OptLevel;
use std::path::Path;
use std::sync::Arc;
use tempfile::TempDir;

/// Common definiton of OptLevel
const BENCHMARK_OPT_LEVEL: OptLevel = OptLevel::SpeedAndSize;

const DENSE_HEAP_SIZES_KB: &'static [usize] =
    &[0, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2 * 1024, 4 * 1024];

const SPARSE_HEAP_SIZES_KB: &'static [usize] = &[0, 256, 512, 1024, 2 * 1024, 4 * 1024];

/// End-to-end instance instantiation.
///
/// This is meant to simulate our startup time when we start from scratch, with no module loaded and
/// no region created at all. This would be unusual for a server application, but reflects what
/// one-shot command-line tools like `lucet-wasi` do.
///
/// To minimize the effects of filesystem cache on the `DlModule::load()`, this runs `sync` between
/// each iteration.
fn hello_load_mkregion_and_instantiate<R: RegionCreate + 'static>(c: &mut Criterion) {
    lucet_wasi::export_wasi_funcs();
    fn body<R: RegionCreate + 'static>(so_file: &Path) -> InstanceHandle {
        let module = DlModule::load(so_file).unwrap();
        let region = R::create(1, &Limits::default()).unwrap();
        region.new_instance(module).unwrap()
    }

    let workdir = TempDir::new().expect("create working directory");

    let so_file = workdir.path().join("out.so");
    compile_hello(&so_file, BENCHMARK_OPT_LEVEL);

    c.bench_function(
        &format!("hello_load_mkregion_and_instantiate ({})", R::TYPE_NAME),
        move |b| {
            b.iter_batched(
                || unsafe { nix::libc::sync() },
                |_| body::<R>(&so_file),
                criterion::BatchSize::PerIteration,
            )
        },
    );

    workdir.close().unwrap();
}

/// Instance instantiation.
///
/// This simulates a typical case for a server process like Terrarium: the region and module stay
/// initialized, but a new instance is created for each request.
fn hello_instantiate<R: RegionCreate + 'static>(c: &mut Criterion) {
    fn body<R: Region>(module: Arc<dyn Module>, region: Arc<R>) -> InstanceHandle {
        region.new_instance(module).unwrap()
    }

    let workdir = TempDir::new().expect("create working directory");

    let so_file = workdir.path().join("out.so");
    compile_hello(&so_file, BENCHMARK_OPT_LEVEL);

    let module = DlModule::load(&so_file).unwrap();
    let region = R::create(1, &Limits::default()).unwrap();

    c.bench_function(&format!("hello_instantiate ({})", R::TYPE_NAME), move |b| {
        b.iter(|| body(module.clone(), region.clone()))
    });

    workdir.close().unwrap();
}

/// Instance instantiation with a large, dense heap.
fn instantiate_with_dense_heap<R: RegionCreate + 'static>(c: &mut Criterion) {
    fn body<R: Region>(module: Arc<dyn Module>, region: Arc<R>) -> InstanceHandle {
        region.new_instance(module).unwrap()
    }

    let limits = Limits {
        heap_memory_size: 1024 * 1024 * 1024,
        ..Limits::default()
    };

    let region = R::create(1, &limits).unwrap();

    c.bench_function_over_inputs(
        &format!("instantiate_with_dense_heap ({})", R::TYPE_NAME),
        move |b, &&heap_kb| {
            let module = large_dense_heap_mock(heap_kb);
            b.iter(|| body(module.clone(), region.clone()))
        },
        DENSE_HEAP_SIZES_KB,
    );
}

/// Instance instantiation with a large, sparse heap.
fn instantiate_with_sparse_heap<R: RegionCreate + 'static>(c: &mut Criterion) {
    fn body<R: Region>(module: Arc<dyn Module>, region: Arc<R>) -> InstanceHandle {
        region.new_instance(module).unwrap()
    }

    let limits = Limits {
        heap_memory_size: 1024 * 1024 * 1024,
        ..Limits::default()
    };

    let region = R::create(1, &limits).unwrap();

    c.bench_function_over_inputs(
        &format!("instantiate_with_sparse_heap ({})", R::TYPE_NAME),
        move |b, &&heap_kb| {
            // 8 means that only every eighth page has non-zero data
            let module = large_sparse_heap_mock(heap_kb, 8);
            b.iter(|| body(module.clone(), region.clone()))
        },
        SPARSE_HEAP_SIZES_KB,
    );
}

/// Instance destruction.
///
/// Instances have some cleanup to do with memory management and freeing their slot on their region.
fn hello_drop_instance<R: RegionCreate + 'static>(c: &mut Criterion) {
    fn body(_inst: InstanceHandle) {}

    let workdir = TempDir::new().expect("create working directory");

    let so_file = workdir.path().join("out.so");
    compile_hello(&so_file, BENCHMARK_OPT_LEVEL);

    let module = DlModule::load(&so_file).unwrap();
    let region = R::create(1, &Limits::default()).unwrap();

    c.bench_function(
        &format!("hello_drop_instance ({})", R::TYPE_NAME),
        move |b| {
            b.iter_batched(
                || region.new_instance(module.clone()).unwrap(),
                |inst| body(inst),
                criterion::BatchSize::PerIteration,
            )
        },
    );

    workdir.close().unwrap();
}

/// Instance destruction with a large, dense heap.
fn drop_instance_with_dense_heap<R: RegionCreate + 'static>(c: &mut Criterion) {
    fn body(_inst: InstanceHandle) {}

    let limits = Limits {
        heap_memory_size: 1024 * 1024 * 1024,
        ..Limits::default()
    };

    let region = R::create(1, &limits).unwrap();

    c.bench_function_over_inputs(
        &format!("drop_instance_with_dense_heap ({})", R::TYPE_NAME),
        move |b, &&heap_kb| {
            let module = large_dense_heap_mock(heap_kb);
            b.iter_batched(
                || region.clone().new_instance(module.clone()).unwrap(),
                |inst| body(inst),
                criterion::BatchSize::PerIteration,
            )
        },
        DENSE_HEAP_SIZES_KB,
    );
}

/// Instance destruction with a large, sparse heap.
fn drop_instance_with_sparse_heap<R: RegionCreate + 'static>(c: &mut Criterion) {
    fn body(_inst: InstanceHandle) {}

    let limits = Limits {
        heap_memory_size: 1024 * 1024 * 1024,
        ..Limits::default()
    };

    let region = R::create(1, &limits).unwrap();

    c.bench_function_over_inputs(
        &format!("drop_instance_with_sparse_heap ({})", R::TYPE_NAME),
        move |b, &&heap_kb| {
            // 8 means that only every eighth page has non-zero data
            let module = large_sparse_heap_mock(heap_kb, 8);
            b.iter_batched(
                || region.clone().new_instance(module.clone()).unwrap(),
                |inst| body(inst),
                criterion::BatchSize::PerIteration,
            )
        },
        SPARSE_HEAP_SIZES_KB,
    );
}

/// Run a trivial guest function.
///
/// This is primarily a measurement of the signal handler installation and removal, and the context
/// switching overhead.
fn run_null<R: RegionCreate + 'static>(c: &mut Criterion) {
    fn body(inst: &mut InstanceHandle) {
        inst.run("f", &[]).unwrap();
    }

    let module = null_mock();
    let region = R::create(1, &Limits::default()).unwrap();

    c.bench_function(&format!("run_null ({})", R::TYPE_NAME), move |b| {
        b.iter_batched_ref(
            || region.new_instance(module.clone()).unwrap(),
            |inst| body(inst),
            criterion::BatchSize::PerIteration,
        )
    });
}

/// Run a computation-heavy guest function from a mock module.
///
/// Since this is running code in a mock module, the cost of the computation should overwhelm the
/// cost of the Lucet runtime.
fn run_fib<R: RegionCreate + 'static>(c: &mut Criterion) {
    fn body(inst: &mut InstanceHandle) {
        inst.run("f", &[]).unwrap();
    }

    let module = fib_mock();
    let region = R::create(1, &Limits::default()).unwrap();

    c.bench_function(&format!("run_fib ({})", R::TYPE_NAME), move |b| {
        b.iter_batched_ref(
            || region.new_instance(module.clone()).unwrap(),
            |inst| body(inst),
            criterion::BatchSize::PerIteration,
        )
    });
}

/// Run a trivial WASI program.
fn run_hello<R: RegionCreate + 'static>(c: &mut Criterion) {
    fn body(inst: &mut InstanceHandle) {
        inst.run("_start", &[]).unwrap();
    }

    let workdir = TempDir::new().expect("create working directory");

    let so_file = workdir.path().join("out.so");
    compile_hello(&so_file, BENCHMARK_OPT_LEVEL);

    let module = DlModule::load(&so_file).unwrap();
    let region = R::create(1, &Limits::default()).unwrap();

    c.bench_function(&format!("run_hello ({})", R::TYPE_NAME), move |b| {
        b.iter_batched_ref(
            || {
                let ctx = WasiCtxBuilder::new()
                    .args(["hello"].iter())
                    .build()
                    .expect("build WasiCtx");
                region
                    .new_instance_builder(module.clone())
                    .with_embed_ctx(ctx)
                    .build()
                    .unwrap()
            },
            |inst| body(inst),
            criterion::BatchSize::PerIteration,
        )
    });
}

/// Run a trivial guest function that takes a bunch of arguments.
///
/// This is primarily interesting as a comparison to `run_null`; the difference is the overhead of
/// installing the arguments into the guest registers and stack.
///
/// `rustfmt` hates this function.
fn run_many_args<R: RegionCreate + 'static>(c: &mut Criterion) {
    fn body(inst: &mut InstanceHandle) {
        inst.run(
            "f",
            &[
                0xAFu8.into(),
                0xAFu16.into(),
                0xAFu32.into(),
                0xAFu64.into(),
                175.0f32.into(),
                175.0f64.into(),
                0xAFu8.into(),
                0xAFu16.into(),
                0xAFu32.into(),
                0xAFu64.into(),
                175.0f32.into(),
                175.0f64.into(),
                0xAFu8.into(),
                0xAFu16.into(),
                0xAFu32.into(),
                0xAFu64.into(),
                175.0f32.into(),
                175.0f64.into(),
                0xAFu8.into(),
                0xAFu16.into(),
                0xAFu32.into(),
                0xAFu64.into(),
                175.0f32.into(),
                175.0f64.into(),
                0xAFu8.into(),
                0xAFu16.into(),
                0xAFu32.into(),
                0xAFu64.into(),
                175.0f32.into(),
                175.0f64.into(),
                0xAFu8.into(),
                0xAFu16.into(),
                0xAFu32.into(),
                0xAFu64.into(),
                175.0f32.into(),
                175.0f64.into(),
                0xAFu8.into(),
                0xAFu16.into(),
                0xAFu32.into(),
                0xAFu64.into(),
                175.0f32.into(),
                175.0f64.into(),
                0xAFu8.into(),
                0xAFu16.into(),
                0xAFu32.into(),
                0xAFu64.into(),
                175.0f32.into(),
                175.0f64.into(),
                0xAFu8.into(),
                0xAFu16.into(),
                0xAFu32.into(),
                0xAFu64.into(),
                175.0f32.into(),
                175.0f64.into(),
                0xAFu8.into(),
                0xAFu16.into(),
                0xAFu32.into(),
                0xAFu64.into(),
                175.0f32.into(),
                175.0f64.into(),
                0xAFu8.into(),
                0xAFu16.into(),
                0xAFu32.into(),
                0xAFu64.into(),
                175.0f32.into(),
                175.0f64.into(),
            ],
        )
        .unwrap();
    }

    let module = many_args_mock();
    let region = R::create(1, &Limits::default()).unwrap();

    c.bench_function(&format!("run_many_args ({})", R::TYPE_NAME), move |b| {
        b.iter_batched_ref(
            || region.new_instance(module.clone()).unwrap(),
            |inst| body(inst),
            criterion::BatchSize::PerIteration,
        )
    });
}

fn run_hostcall_wrapped<R: RegionCreate + 'static>(c: &mut Criterion) {
    fn body(inst: &mut InstanceHandle) {
        inst.run("wrapped", &[]).unwrap();
    }

    let module = hostcalls_mock();
    let region = R::create(1, &Limits::default()).unwrap();

    c.bench_function(
        &format!("run_hostcall_wrapped ({})", R::TYPE_NAME),
        move |b| {
            b.iter_batched_ref(
                || region.new_instance(module.clone()).unwrap(),
                |inst| body(inst),
                criterion::BatchSize::PerIteration,
            )
        },
    );
}

fn run_hostcall_raw<R: RegionCreate + 'static>(c: &mut Criterion) {
    fn body(inst: &mut InstanceHandle) {
        inst.run("raw", &[]).unwrap();
    }

    let module = hostcalls_mock();
    let region = R::create(1, &Limits::default()).unwrap();

    c.bench_function(&format!("run_hostcall_raw ({})", R::TYPE_NAME), move |b| {
        b.iter_batched_ref(
            || region.new_instance(module.clone()).unwrap(),
            |inst| body(inst),
            criterion::BatchSize::PerIteration,
        )
    });
}

pub fn seq_benches<R: RegionCreate + 'static>(c: &mut Criterion) {
    hello_load_mkregion_and_instantiate::<R>(c);
    hello_instantiate::<R>(c);
    instantiate_with_dense_heap::<R>(c);
    instantiate_with_sparse_heap::<R>(c);
    hello_drop_instance::<R>(c);
    drop_instance_with_dense_heap::<R>(c);
    drop_instance_with_sparse_heap::<R>(c);
    run_null::<R>(c);
    run_fib::<R>(c);
    run_hello::<R>(c);
    run_many_args::<R>(c);
    run_hostcall_wrapped::<R>(c);
    run_hostcall_raw::<R>(c);
}
