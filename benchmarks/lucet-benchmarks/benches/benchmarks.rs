use criterion::Criterion;
use lucet_benchmarks::*;
use lucet_runtime::MmapRegion;

fn main() {
    let mut c = Criterion::default().configure_from_args();

    compile_benches(&mut c);
    context_benches(&mut c);
    seq_benches::<MmapRegion>(&mut c);
    par_benches::<MmapRegion>(&mut c);

    c.final_summary();
}
