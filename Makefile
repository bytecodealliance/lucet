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
            -p lucet-microbenchmarks
	cargo run -p lucet-wasi-fuzz -- --num-tests=3

.PHONY: fuzz
fuzz:
	cargo run --release -p lucet-wasi-fuzz -- --num-tests=1000

.PHONY: bench
bench:
	cargo bench -p lucet-microbenchmarks
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
