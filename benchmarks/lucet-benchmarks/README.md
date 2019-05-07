# lucet-benchmarks

This crate defines a suite of microbenchmarks for the Lucet compiler and runtime. It is divided into
several suites to measure different categories of performance.

To run the suite, run `cargo bench -p lucet-benchmarks`. Since some of the benchmarks measure
operations with extremely small runtimes, make sure to close as many competing background
applications as possible. For the most consistent results, disable simultaneous multithreading and
dynamic frequency scaling features such as Hyper-Threading or Turbo Boost before benchmarking.

## Benchmark Suites

### `src/compiler.rs`

The compiler suite measures the performance of `lucetc` when compiling WebAssembly modules. This
suite is still fairly rudimentary; we need to add more example guests, split out the `clang` step,
and add a programmatic interface to `lucetc` that doesn't require an output file.

### `src/context.rs`

The context suite measures the performance of the low-level context switching code used by the
runtime to swap between host and guest execution.

### `src/par.rs`

The parallel execution suite measures the performance of running various runtime operations in
parallel. This suite is still fairly rudimentary.

### `src/seq.rs`

The sequential execution suite measures the performance of single-threaded Lucet runtime use. It
attempts to isolate the main phases of instantiation, running, and dropping a Lucet instance, as
well as some basic guest code benchmarking. The shootout benchmarks in `/benchmarks/shootout` are a
better means of analyzing the performance of guest code.
