# This Makefile exists primarily to keep all of the test guest sources in one place. They should
# probably be factored out of the C runtime eventually into a standalone test support package.

default: build

LIBLUCET_RUNTIME_C:=../lucet-runtime-c
.PHONY: $(LIBLUCET_RUNTIME_C)
$(LIBLUCET_RUNTIME_C):
	make -C $(LIBLUCET_RUNTIME_C)
	make -C $(LIBLUCET_RUNTIME_C)/test guests

LUCET_TESTS:=../tests
.PHONY: $(LUCET_TESTS)
$(LUCET_TESTS):
	make -C $(LUCET_TESTS) guests

TEST_DEPS:= $(LIBLUCET_RUNTIME_C) $(LUCET_TESTS)

.PHONY: build
build:
	cargo build
	cd lucet-runtime-internals && cargo build
	cd lucet-runtime-tests && cargo build

.PHONY: test
test: build $(TEST_DEPS)
	cargo test
	cd lucet-runtime-internals && cargo test
	cd lucet-runtime-tests && cargo test

.PHONY: audit
audit:
	cargo audit

.PHONY: clean
clean:
	cargo clean
	cd lucet-runtime-internals && cargo clean
	cd lucet-runtime-tests && cargo clean
