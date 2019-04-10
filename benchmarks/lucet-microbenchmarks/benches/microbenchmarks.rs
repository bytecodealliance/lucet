#[macro_use]
extern crate criterion;

use lucet_microbenchmarks::{benches, par};

criterion_main!(benches, par);
