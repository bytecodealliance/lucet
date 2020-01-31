# Tests and benchmarks

Most of the crates in this repository have some form of unit tests. In addition,
`lucet-runtime/lucet-runtime-tests` defines a number of integration tests for the runtime, and
`lucet-wasi` has a number of integration tests using WASI C programs.

We also created the [Sight Glass](./sightglass.md) benchmarking tool to measure the runtime of C
code compiled through a standard native toolchain against the Lucet toolchain. It is provided as a
submodule at `/sightglass`.

Sightglass ships with a set of microbenchmarks called `shootout`. The scripts to build the shootout
tests with native and various versions of the Lucet toolchain are in `/benchmarks/shootout`.

Furthermore, there is a suite of benchmarks of various Lucet runtime functions, such as instance
creation and teardown, in `/benchmarks/lucet-benchmarks`.

## Adding new tests for crates other than `lucet-runtime` or `lucet-runtime-internals`

Most of the crates in this repository are tested in the usual way, with a mix of unit tests defined
alongside the modules they test, and integration tests defined in separate test targets.

Note that `lucetc` and `lucet-wasi-sdk` provide library interfaces for the Wasm compiler and the
C-to-Wasm cross compiler, respectively. You should prefer using these interfaces in a test to using
the command-line tools directly with `std::process:Command`, a Makefile, or a shell script.

## Adding new tests for `lucet-runtime` or `lucet-runtime-internals`

While these crates have a similar mix of unit and integration tests, there are some additional
complications to make sure the public interface is well-tested, and to allow reuse of tests across
backends.

### Public interface tests

The tests defined in `lucet-runtime` and `lucet-runtime-tests` are meant to exclusively test the
public interface exported from `lucet-runtime`. This is to ensure that the public interface is
sufficiently expressive to use all of the features we want to expose to users, and to catch linking
problems as early as possible.

While some tests in these crates use APIs exported only from `lucet-runtime-internals`, this is only
for test scaffolding or inspection of results. The parts of the test that are "what the user might
do" should only be defined in terms of `lucet-runtime` APIs.

### Test reuse, regions, and macros

Many of the tests in the runtime require the use of a `lucet_runtime::Region` in order to create
Lucet instances. To enable reuse of these tests across multiple `Region` implementations, many tests
are defined in macros. For example, the unit tests defined in
`/lucet-runtime/lucet-runtime-internals/src/alloc/tests.rs` are parameterized by a `TestRegion`
argument, which is then applied using `crate::region::mmap::MmapRegion`. Likewise, many of the
integration tests in `/lucet-runtime/lucet-runtime-tests` are defined using macros that take a
`TestRegion`.

Most tests that use a `Region` should be defined within a macro like this. The exception is for
tests that are specific to a particular region implementation, likely using an API that is not part
of the `Region` trait. These tests should be defined alongside the implementation of the region (for
unit tests) or directly in a `lucet-runtime` test target (for integration tests).
