use criterion::Criterion;
use lucet_microbenchmarks::{par_benches, seq_benches};
use lucet_runtime::MmapRegion;

fn main() {
    let mut c = Criterion::default().configure_from_args();

    seq_benches::<MmapRegion>(&mut c);
    par_benches::<MmapRegion>(&mut c);

    c.final_summary();
}
