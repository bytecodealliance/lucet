# This Makefile exists primarily to keep all of the test guest sources in one place. They should
# probably be factored out of the C runtime eventually into a standalone test support package.

LD_LIBRARY_PATH:=$(abspath ../lucet-libc/build/lib):$(LD_LIBRARY_PATH)
GUEST_MODULE_PREFIX=$(abspath $(CURDIR)/..)

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
	cd lucet-runtime-internals && cargo build
	cargo build

.PHONY: test
test: build $(TEST_DEPS)
	cd lucet-runtime-internals && LD_LIBRARY_PATH=$(LD_LIBRARY_PATH) GUEST_MODULE_PREFIX=$(GUEST_MODULE_PREFIX) cargo test --no-fail-fast
	LD_LIBRARY_PATH=$(LD_LIBRARY_PATH) GUEST_MODULE_PREFIX=$(GUEST_MODULE_PREFIX) cargo test --no-fail-fast

.PHONY: audit
audit:
	cargo audit

.PHONY: clean
clean:
	cd lucet-runtime-internals && cargo clean
	cargo clean
