
default: build

.PHONY: build
build:
	cargo build

.PHONY: test
test:
	cargo test -p lucet-spectest -- --nocapture --test-threads 1

.PHONY: test-%
test-%:
	RUST_BACKTRACE=1 cargo run -p lucet-spectest -- spec/test/core/$*.wast

.PHONY: clean
clean:
	cargo clean
