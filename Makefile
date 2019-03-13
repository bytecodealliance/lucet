export GUEST_MODULE_PREFIX:=$(abspath .)

.PHONY: build
build:
	cargo build --all

.PHONY: test
test: indent-check
	cargo test --no-fail-fast \
            -p lucet-runtime-internals \
            -p lucet-runtime \
            -p lucet-module-data \
            -p lucetc \
            -p lucet-idl \
            -p lucet-wasi-sdk

.PHONY: bench
bench:
	make -C benchmarks/shootout clean
	make -C benchmarks/shootout bench

.PHONY: audit
audit:
	cargo audit

.PHONY: clean
clean:
	make -C benchmarks/shootout clean
	make -C builtins clean
	cargo clean

.PHONY: indent
indent:
	./indent.sh

.PHONY: indent-check
indent-check:
	./indent.sh check
