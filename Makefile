
.PHONY: build
build:
	cd lucetc && cargo build
	make -C lucet-runtime
	make -C lucet-runtime-c
	make -C lucet-backtrace
	make -C lucet-rs
	cd lucet-module-data && cargo build
	cd lucet-spectest && cargo build
	cd lucet-analyze && cargo build
	cd lucet-idl && cargo build

.PHONY: build-test-deps
build-test-deps:
	cd lucetc && cargo build

.PHONY: test
test: build-test-deps
	make -C lucet-runtime test
	make -C lucet-runtime-c test
	make -C lucet-backtrace test
	make -C lucet-rs test
	cd lucet-module-data && cargo test
	cd lucetc && cargo test
	cd lucet-idl && cargo test
	make -C tests

.PHONY: bench
bench:
	make -C benchmarks/shootout clean
	make -C benchmarks/shootout bench

.PHONY: audit
audit:
	make -C lucet-runtime audit
	make -C lucet-rs audit
	cd lucet-module-data && cargo audit
	cd lucetc && cargo audit
	cd lucet-idl && cargo audit

.PHONY: clean
clean:
	rm -rf lucetc/target
	rm -rf lucet-idl/target
	make -C benchmarks/shootout clean
	make -C builtins clean
	make -C lucet-runtime clean
	make -C lucet-runtime-c clean
	make -C lucet-backtrace clean
	make -C lucet-rs clean
	make -C tests clean
	cd lucetc && cargo clean
	cd lucet-idl && cargo clean
	cd lucet-analyze && cargo clean
	cd lucet-spectest && cargo clean
	cd lucet-module-data && cargo clean
	cd sightglass && cargo clean

.PHONY: indent
indent:
	./indent.sh

.PHONY: indent-check
indent-check:
	./indent.sh check
