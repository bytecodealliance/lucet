use crate::modules::*;
use criterion::Criterion;
use lucetc::OptLevel;
use tempfile::TempDir;

/// Compile Hello World with default optimizations.
fn compile_hello_all(c: &mut Criterion) {
    fn body(workdir: &TempDir, opt_level: OptLevel) {
        let out = workdir.path().join("out.so");
        compile_hello(out, opt_level);
    }

    let bench = criterion::ParameterizedBenchmark::new(
        "compile_hello",
        move |b, &&opt_level| {
            b.iter_batched_ref(
                || TempDir::new().expect("create per-run working directory"),
                |workdir| body(workdir, opt_level),
                criterion::BatchSize::SmallInput,
            )
        },
        &[OptLevel::None, OptLevel::Speed, OptLevel::SpeedAndSize],
    )
    .sample_size(10);

    c.bench("compile", bench);
}

pub fn compile_benches(c: &mut Criterion) {
    compile_hello_all(c);
}
