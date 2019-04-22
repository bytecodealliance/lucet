use criterion::Criterion;
use lucet_runtime_internals::context::{Context, ContextHandle};

/// Time the initialization of a context.
fn context_init(c: &mut Criterion) {
    extern "C" fn f() {}

    let mut stack = vec![0u64; 1024].into_boxed_slice();

    c.bench_function("context_init", move |b| {
        b.iter(|| {
            let mut parent = ContextHandle::new();
            ContextHandle::create_and_init(
                &mut *stack,
                &mut parent,
                f as *const extern "C" fn(),
                &[],
            )
            .unwrap();
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
                let mut parent = ContextHandle::new();
                let child = ContextHandle::create_and_init(
                    &mut *stack,
                    &mut parent,
                    f as *const extern "C" fn(),
                    &[],
                )
                .unwrap();
                (stack, parent, child)
            },
            |(stack, mut parent, child)| unsafe {
                Context::swap(&mut parent, &child);
                stack
            },
            criterion::BatchSize::PerIteration,
        )
    });
}

/// Time the call to sigprocmask as used in `Context::init()`.
fn context_sigprocmask(c: &mut Criterion) {
    use nix::sys::signal;
    c.bench_function("context_sigprocmask", |b| {
        b.iter_batched(
            || signal::SigSet::empty(),
            |mut sigset| {
                signal::sigprocmask(signal::SigmaskHow::SIG_SETMASK, None, Some(&mut sigset))
                    .unwrap()
            },
            criterion::BatchSize::PerIteration,
        )
    });
}

pub fn context_benches(c: &mut Criterion) {
    context_init(c);
    context_swap_return(c);
    context_sigprocmask(c);
}
