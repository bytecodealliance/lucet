
default: build

.PHONY: build
build:
	cargo build


.PHONY: test
test:
	LD_LIBRARY_PATH=$(abspath ../lucet-runtime-c/build) cargo test -- --nocapture

.PHONY: test-%
test-%:
	RUST_BACKTRACE=1 LD_LIBRARY_PATH=$(abspath ../lucet-runtime-c/build) cargo run -- spec/test/core/$*.wast

.PHONY: clean
clean:
	cargo clean
