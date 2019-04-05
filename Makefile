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
            -p lucet-wasi

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
	make -C lucet-builtins clean
	cargo clean

.PHONY: indent
indent:
	helpers/indent.sh

.PHONY: indent-check
indent-check:
	helpers/indent.sh check
