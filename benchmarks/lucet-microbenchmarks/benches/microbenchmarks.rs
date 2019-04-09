#[macro_use]
extern crate criterion;

use lucet_microbenchmarks::benches;

criterion_main!(benches);
