# Sight Glass

Sight Glass is a benchmark suite and tool to compare different implementations of the same primitives.

## Usage

Sight Glass loads multiple shared libraries implementing the same test suite, runs all tests from all suites, and produces reports to evaluate how implementations compare to each other.

Functions from each library are evaluated as follows:

```c
tests_config.global_setup(&global_ctx);

  test1_setup(global_ctx, &test1_ctx);
  test1_body(test1_ctx);
  test1_teardown(test1_ctx);

  test2_setup(global_ctx, &test2_ctx);
  test2_body(test2_ctx);
  test2_teardown(test2_ctx);

  // ...

  testN_setup(global_ctx, &testN_ctx);
  testN_body(testN_ctx);
  testN_teardown(testN_ctx);

tests_config.global_teardown(global_ctx);
```

Each shared library must export a `tests_config` symbol:

```c
typedef struct TestsConfig {
    void     (*global_setup)(void **global_ctx_p);
    void     (*global_teardown)(void *global_ctx);
    uint64_t version;
} TestsConfig;

TestsConfig tests_config;
```

`global_setup` and `global_teardown` are optional, and can be set to `NULL` if not required.

A test must at least export a function named `<testname>_body`:

```c
void testname_body(void *ctx);
```

This function contains the actual code to be benchmarked.

By default, `ctx` will be set to the `global_ctx`. However, optional `setup` and `teardown` functions can also be provided for individual tests:

```c
void testname_setup(void *global_ctx, void **ctx_p);

void testname_teardown(void *ctx);
```

See `example/example.c` for an example test suite.

Sightglass extracts all symbols matching the above convention to define and run the test suite.

## Running multiple functions for a single test

A single test can evaluate multiple body functions sharing the same context.

These functions have to be named `<testname>_body_<bodyname>`.

`<bodyname>` can be anything; a numeric ID or a short description of the purpose of the function.

```c
void testname_body_2(void *ctx);
void testname_body_randomized(void *ctx);
```

These functions are guaranteed to be evaluated according to their names sorted in lexical order.

## Configuration

The global configuration is loaded from `sightglass.toml` file. This can be changed using the `-c` command-line flag.

The configuration lists implementations to be benchmarked:

```toml
test_suites = [
  { name = "test1", library_path = "implementation1.so" },
  { name = "test2", library_path = "implementation2.so" }
]
```

Individual test suites can also run a command in order to be optionally skipped if that command returns a non-zero exit code:

```toml
test_suites = [
  { name = "test1", library_path = "implementation1.so" },
  { name = "test2", library_path = "implementation2.so", guard = ["/opt/sg/guard-scripts/check", "arg1", "arg2"] }
]
```

Additional properties that the file can include:

- `single_core = <bool>`: set to `true` in order to run the tests on a single CPU core, in order to get more accurate results. This only works on Linux.

- `output = [ { format = "Text|CSV|JSON" [, file = <file>] [, breakdown = <bool>] } ... ]`: how to store or display the results.

By defaut, the `Text` and `CSV` output do not include a breakdown of the time spent in individual functions for tests made of multiple functions.
This can be changed with the optional `breakdown` property being set to `true`.

The `JSON` output always includes this information.
