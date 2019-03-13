export GUEST_MODULE_PREFIX:=$(abspath .)

.PHONY: build
build:
	make -C lucet-libc
	cargo build --all

.PHONY: build-test-deps
build-test-deps:
	cargo build -p lucetc
	make -C lucet-libc

.PHONY: test
test: indent-check build-test-deps
	cargo test --no-fail-fast \
            -p lucet-runtime-internals \
            -p lucet-runtime \
            -p lucet \
            -p lucet-sys \
            -p lucet-libc \
            -p lucet-libc-sys \
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
	make -C lucet-libc clean
	cargo clean

.PHONY: indent
indent:
	./indent.sh

.PHONY: indent-check
indent-check:
	./indent.sh check
