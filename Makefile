export GUEST_MODULE_PREFIX:=$(abspath .)

CRATES_NOT_TESTED ?= lucet-spectest lucet-benchmarks lucet-runtime-example

.PHONY: build-dev
build-dev:
	@echo Creating a DEBUG build
	cargo build --workspace

.PHONY: build
build:
	@echo Creating a RELEASE build
	cargo build --workspace --release --bins --lib

.PHONY: install
install: build
	@helpers/install.sh

.PHONY: install-dev
install-dev: build-dev
	@helpers/install.sh --unoptimized

.PHONY: test
test: indent-check test-crates

.PHONY: test-crates
test-crates:
	cargo test --no-fail-fast --all $(CRATES_NOT_TESTED:%=--exclude %)

.PHONY: test-full
test-full: indent-check audit book test-ci test-benchmarks test-fuzz

# The --release option runs the tests on an artifact built in release mode. We
# have found regressions in release mode due to optimizations in the past.
.PHONY: test-release
test-release:
	cargo test --release --no-fail-fast --all --exclude $(CRATES_NOT_TESTED:%=--exclude %)

.PHONY: test-release-executables
test-release-executables:
	cargo build --release -p lucetc -p lucet-wasi -p lucet-objdump
	helpers/lucet-toolchain-tests/signature.sh release
	helpers/lucet-toolchain-tests/objdump.sh release

.PHONY: test-ci
test-ci: test-crates test-objdump test-bitrot test-signature test-objdump

.PHONY: test-bitrot
test-bitrot:
	# build but do *not* run these crates. The tests for these crates are
	# known to fail, but we want to make sure they still build.
	cargo check -p lucet-spectest -p lucet-runtime-example

.PHONY: test-signature
test-signature:
	helpers/lucet-toolchain-tests/signature.sh

.PHONY: test-objdump
test-objdump:
	cargo build -p lucet-objdump
	helpers/lucet-toolchain-tests/objdump.sh

.PHONY: test-benchmarks
test-benchmarks:
	# Smoke test of benchmarks:
	cargo test --benches -p lucet-benchmarks -- --test

# run a single seed through the fuzzer to stave off bitrot
.PHONY: test-fuzz
test-fuzz:
	cargo run -p lucet-wasi-fuzz -- test-seed 410757864950

FUZZ_NUM_TESTS?=1000
.PHONY: fuzz
fuzz:
	cargo run --release -p lucet-wasi-fuzz -- fuzz --num-tests=$(FUZZ_NUM_TESTS)

.PHONY: book
book:
	mdbook build docs

.PHONY: bench
bench:
	cargo bench -p lucet-benchmarks
	make -C benchmarks/shootout clean
	make -C benchmarks/shootout bench

.PHONY: audit
audit:
	cargo audit

.PHONY: clean
clean:
	make -C benchmarks/shootout clean
	cargo clean

.PHONY: indent
indent:
	helpers/indent.sh

.PHONY: indent-check
indent-check:
	helpers/indent.sh check

.PHONY: watch
watch:
	cargo watch --exec "test --all --exclude $(CRATES_NOT_TESTED)"
