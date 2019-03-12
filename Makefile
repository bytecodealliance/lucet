# Even though we don't need LD_LIBRARY_PATH for each target, setting it once and for all prevents
# Cargo from engaging in unnecessary rebuilds
export LD_LIBRARY_PATH:=$(abspath ./lucet-runtime-c/build):$(abspath ./lucet-libc/build/lib):$(LD_LIBRARY_PATH)
export GUEST_MODULE_PREFIX:=$(abspath .)

.PHONY: build
build:
	make -C lucet-runtime-c
	make -C lucet-backtrace
	make -C lucet-libc
	cargo build --all

.PHONY: build-test-deps
build-test-deps:
	cargo build -p lucetc
	make -C lucet-runtime-c/test guests
	make -C lucet-libc
	make -C tests guests

.PHONY: test
test: indent-check build-test-deps
	make -C lucet-runtime-c test
	make -C lucet-backtrace test
	cargo test --no-fail-fast \
            -p lucet-runtime-internals \
            -p lucet-runtime \
            -p lucet \
            -p lucet-sys \
            -p lucet-libc \
            -p lucet-libc-sys \
            -p lucet-module-data \
            -p lucetc \
            -p lucet-idl
	make -C tests

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
	make -C lucet-runtime-c clean
	make -C lucet-backtrace clean
	make -C lucet-libc clean
	make -C tests clean
	cargo clean

.PHONY: indent
indent:
	./indent.sh

.PHONY: indent-check
indent-check:
	./indent.sh check
