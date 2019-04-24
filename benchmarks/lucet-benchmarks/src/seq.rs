use crate::modules::*;
use criterion::Criterion;
use lucet_runtime::{DlModule, InstanceHandle, Limits, Module, Region, RegionCreate};
use lucet_wasi::WasiCtxBuilder;
use std::path::Path;
use std::sync::Arc;
use tempfile::TempDir;

/// End-to-end instance instantiation.
///
/// This is meant to simulate our startup time when we start from scratch, with no module loaded and
/// no region created at all. This would be unusual for a server application, but reflects what
/// one-shot command-line tools like `lucet-wasi` do.
///
/// To minimize the effects of filesystem cache on the `DlModule::load()`, this runs `sync` between
/// each iteration.
fn load_mkregion_and_instantiate<R: RegionCreate + 'static>(c: &mut Criterion) {
    fn body<R: RegionCreate + 'static>(so_file: &Path) -> InstanceHandle {
        let module = DlModule::load(so_file).unwrap();
        let region = R::create(1, &Limits::default()).unwrap();
        region.new_instance(module).unwrap()
    }

    let workdir = TempDir::new().expect("create working directory");

    let so_file = workdir.path().join("out.so");
    compile_hello(&so_file);

    c.bench_function(
        &format!("load_mkregion_and_instantiate ({})", R::TYPE_NAME),
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
fn instantiate<R: RegionCreate + 'static>(c: &mut Criterion) {
    fn body<R: Region>(module: Arc<dyn Module>, region: Arc<R>) -> InstanceHandle {
        region.new_instance(module).unwrap()
    }

    let workdir = TempDir::new().expect("create working directory");

    let so_file = workdir.path().join("out.so");
    compile_hello(&so_file);

    let module = DlModule::load(&so_file).unwrap();
    let region = R::create(1, &Limits::default()).unwrap();

    c.bench_function(&format!("instantiate ({})", R::TYPE_NAME), move |b| {
        b.iter(|| body(module.clone(), region.clone()))
    });

    workdir.close().unwrap();
}

/// Instance instantiation with a large, dense heap.
fn instantiate_large_dense<R: RegionCreate + 'static>(c: &mut Criterion) {
    fn body<R: Region>(module: Arc<dyn Module>, region: Arc<R>) -> InstanceHandle {
        region.new_instance(module).unwrap()
    }

    let module = large_dense_heap_mock();

    let limits = Limits {
        heap_memory_size: 1024 * 1024 * 1024,
        ..Limits::default()
    };

    let region = R::create(1, &limits).unwrap();

    c.bench_function(
        &format!("instantiate_large_dense ({})", R::TYPE_NAME),
        move |b| b.iter(|| body(module.clone(), region.clone())),
    );
}

/// Instance instantiation with a large, sparse heap.
fn instantiate_large_sparse<R: RegionCreate + 'static>(c: &mut Criterion) {
    fn body<R: Region>(module: Arc<dyn Module>, region: Arc<R>) -> InstanceHandle {
        region.new_instance(module).unwrap()
    }

    let module = large_sparse_heap_mock();

    let limits = Limits {
        heap_memory_size: 1024 * 1024 * 1024,
        ..Limits::default()
    };

    let region = R::create(1, &limits).unwrap();

    c.bench_function(
        &format!("instantiate_large_sparse ({})", R::TYPE_NAME),
        move |b| b.iter(|| body(module.clone(), region.clone())),
    );
}

/// Instance destruction.
///
/// Instances have some cleanup to do with memory management and freeing their slot on their region.
fn drop_instance<R: RegionCreate + 'static>(c: &mut Criterion) {
    fn body(_inst: InstanceHandle) {}

    let workdir = TempDir::new().expect("create working directory");

    let so_file = workdir.path().join("out.so");
    compile_hello(&so_file);

    let module = DlModule::load(&so_file).unwrap();
    let region = R::create(1, &Limits::default()).unwrap();

    c.bench_function(&format!("drop_instance ({})", R::TYPE_NAME), move |b| {
        b.iter_batched(
            || region.new_instance(module.clone()).unwrap(),
            |inst| body(inst),
            criterion::BatchSize::PerIteration,
        )
    });

    workdir.close().unwrap();
}

/// Instance destruction with a large, dense heap.
fn drop_instance_large_dense<R: RegionCreate + 'static>(c: &mut Criterion) {
    fn body(_inst: InstanceHandle) {}

    let limits = Limits {
        heap_memory_size: 1024 * 1024 * 1024,
        ..Limits::default()
    };

    let module = large_dense_heap_mock();
    let region = R::create(1, &limits).unwrap();

    c.bench_function(
        &format!("drop_instance_large_dense ({})", R::TYPE_NAME),
        move |b| {
            b.iter_batched(
                || region.new_instance(module.clone()).unwrap(),
                |inst| body(inst),
                criterion::BatchSize::PerIteration,
            )
        },
    );
}

/// Instance destruction with a large, sparse heap.
fn drop_instance_large_sparse<R: RegionCreate + 'static>(c: &mut Criterion) {
    fn body(_inst: InstanceHandle) {}

    let limits = Limits {
        heap_memory_size: 1024 * 1024 * 1024,
        ..Limits::default()
    };

    let module = large_sparse_heap_mock();
    let region = R::create(1, &limits).unwrap();

    c.bench_function(
        &format!("drop_instance_large_sparse ({})", R::TYPE_NAME),
        move |b| {
            b.iter_batched(
                || region.new_instance(module.clone()).unwrap(),
                |inst| body(inst),
                criterion::BatchSize::PerIteration,
            )
        },
    );
}

/// Run a trivial guest function.
///
/// This is primarily a measurement of the signal handler installation and removal, and the context
/// switching overhead.
fn run_null<R: RegionCreate + 'static>(c: &mut Criterion) {
    fn body(inst: &mut InstanceHandle) {
        inst.run(b"f", &[]).unwrap();
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
        inst.run(b"f", &[]).unwrap();
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
        inst.run(b"_start", &[]).unwrap();
    }

    let workdir = TempDir::new().expect("create working directory");

    let so_file = workdir.path().join("out.so");
    compile_hello(&so_file);

    let module = DlModule::load(&so_file).unwrap();
    let region = R::create(1, &Limits::default()).unwrap();

    c.bench_function(&format!("run_hello ({})", R::TYPE_NAME), move |b| {
        b.iter_batched_ref(
            || {
                let null = std::fs::File::open("/dev/null").unwrap();
                let ctx = WasiCtxBuilder::new()
                    .args(&["hello"])
                    .fd(1, null)
                    .build()
                    .unwrap();
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
            b"f",
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

pub fn seq_benches<R: RegionCreate + 'static>(c: &mut Criterion) {
    load_mkregion_and_instantiate::<R>(c);
    instantiate::<R>(c);
    instantiate_large_dense::<R>(c);
    instantiate_large_sparse::<R>(c);
    drop_instance::<R>(c);
    drop_instance_large_dense::<R>(c);
    drop_instance_large_sparse::<R>(c);
    run_null::<R>(c);
    run_fib::<R>(c);
    run_hello::<R>(c);
    run_many_args::<R>(c);
}
