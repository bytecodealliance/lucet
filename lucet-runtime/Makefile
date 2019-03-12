# This Makefile exists primarily to allow running just the test suite for lucet-runtime, rather than
# for the whole workspace.

export GUEST_MODULE_PREFIX=$(abspath $(CURDIR)/..)

default: build

.PHONY: lucetc
lucetc:
	cargo build -p lucetc

LUCET_LIBC:=../lucet-libc
.PHONY: $(LUCET_LIBC)
$(LUCET_LIBC):
	make -C $(LUCET_LIBC)

LUCET_TESTS:=../tests
.PHONY: $(LUCET_TESTS)
$(LUCET_TESTS): $(LUCET_LIBC) lucetc
	make -C $(LUCET_TESTS) guests

TEST_DEPS:= $(LUCET_LIB_C) $(LUCET_TESTS)

.PHONY: build
build:
	cargo build -p lucet-runtime

.PHONY: release
release:
	cargo build --release -p lucet-runtime

.PHONY: test
test: build $(TEST_DEPS)
	cargo test --no-fail-fast -p lucet-runtime -p lucet-runtime-internals

.PHONY: audit
audit:
	cargo audit

.PHONY: clean
clean:
	cargo clean -p lucet-runtime lucet-runtime-internals lucet-runtime-tests
