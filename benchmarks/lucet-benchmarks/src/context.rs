use criterion::Criterion;
use lucet_runtime_internals::context::{Context, ContextHandle};

/// Time the initialization of a context.
fn context_init(c: &mut Criterion) {
    extern "C" fn f() {}

    let mut stack = vec![0u64; 1024].into_boxed_slice();

    c.bench_function("context_init", move |b| {
        b.iter(|| {
            ContextHandle::create_and_init(&mut *stack, f as usize, &[]).unwrap();
        })
    });
}

/// Time the swap from an already-initialized context to a guest function and back.
fn context_swap_return(c: &mut Criterion) {
    extern "C" fn f() {}

    c.bench_function("context_swap_return", move |b| {
        b.iter_batched(
            || {
                let mut stack = vec![0u64; 1024].into_boxed_slice();
                let child = ContextHandle::create_and_init(&mut *stack, f as usize, &[]).unwrap();
                (stack, child)
            },
            |(stack, mut child)| unsafe {
                let mut parent = ContextHandle::new();
                Context::swap(&mut parent, &mut child);
                stack
            },
            criterion::BatchSize::PerIteration,
        )
    });
}

/// Time initialization plus swap and return.
fn context_init_swap_return(c: &mut Criterion) {
    extern "C" fn f() {}

    c.bench_function("context_init_swap_return", move |b| {
        b.iter_batched(
            || vec![0u64; 1024].into_boxed_slice(),
            |mut stack| {
                let mut parent = ContextHandle::new();
                let mut child =
                    ContextHandle::create_and_init(&mut *stack, f as usize, &[]).unwrap();
                unsafe { Context::swap(&mut parent, &mut child) };
                stack
            },
            criterion::BatchSize::PerIteration,
        )
    });
}

/// Swap to a trivial guest function that takes a bunch of arguments.
fn context_init_swap_return_many_args(c: &mut Criterion) {
    extern "C" fn f(
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

    let args = vec![
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
    ];

    c.bench_function("context_init_swap_return_many_args", move |b| {
        b.iter_batched(
            || vec![0u64; 1024].into_boxed_slice(),
            |mut stack| {
                let mut parent = ContextHandle::new();
                let mut child =
                    ContextHandle::create_and_init(&mut *stack, f as usize, &args).unwrap();
                unsafe { Context::swap(&mut parent, &mut child) };
                stack
            },
            criterion::BatchSize::PerIteration,
        )
    });
}

/// Time the call to pthread_sigmask as used in `Context::init()`.
fn context_pthread_sigmask(c: &mut Criterion) {
    use nix::sys::signal;
    c.bench_function("context_pthread_sigmask", |b| {
        b.iter_batched(
            || signal::SigSet::empty(),
            |mut sigset| {
                signal::pthread_sigmask(signal::SigmaskHow::SIG_SETMASK, None, Some(&mut sigset))
                    .unwrap()
            },
            criterion::BatchSize::PerIteration,
        )
    });
}

pub fn context_benches(c: &mut Criterion) {
    context_init(c);
    context_swap_return(c);
    context_init_swap_return(c);
    context_init_swap_return_many_args(c);
    context_pthread_sigmask(c);
}
