export GUEST_MODULE_PREFIX:=$(abspath .)

.PHONY: build-dev
build-dev:
	@echo Creating a DEBUG build
	cargo build --all
	make -C lucet-builtins

.PHONY: build
build:
	@echo Creating a RELEASE build
	cargo build --all --release --bins --lib
	make -C lucet-builtins

.PHONY: install
install: build
	@helpers/install.sh

.PHONY: install-dev
install-dev: build-dev
	@helpers/install.sh --unoptimized

.PHONY: test
test: indent-check
	cargo test --no-fail-fast \
            -p lucet-runtime-internals \
            -p lucet-runtime \
            -p lucet-module-data \
            -p lucetc \
            -p lucet-idl \
            -p lucet-wasi-sdk \
            -p lucet-wasi \
            -p lucet-benchmarks
    # run a single seed through the fuzzer to stave off bitrot
	cargo run -p lucet-wasi-fuzz -- test-seed 410757864950

.PHONY: fuzz
fuzz:
	cargo run --release -p lucet-wasi-fuzz -- fuzz --num-tests=1000

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
	make -C lucet-builtins clean
	cargo clean

.PHONY: indent
indent:
	helpers/indent.sh

.PHONY: indent-check
indent-check:
	helpers/indent.sh check

.PHONY: watch
watch:
	cargo watch --exec "test \
            -p lucet-runtime-internals \
            -p lucet-runtime \
            -p lucet-module-data \
            -p lucetc \
            -p lucet-idl \
            -p lucet-wasi-sdk \
            -p lucet-wasi \
            -p lucet-benchmarks"
