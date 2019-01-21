
LIBLUCET_RUNTIME_C:=../lucet-runtime-c

BUILD_DEPS:=$(LIBLUCET_RUNTIME_C)/build/liblucet-runtime-c.so
TEST_DEPS:=$(LIBLUCET_RUNTIME_C)/test/build/entrypoint/calculator.so $(LIBLUCET_RUNTIME_C)/test/build/globals/internal.so

default: build

.PHONY: build
build: $(BUILD_DEPS)
	cargo build

.PHONY: release
release: $(BUILD_DEPS)
	cargo build --release

.PHONY: test
test: build $(TEST_DEPS)
	cd lucet-sys && LD_LIBRARY_PATH=$(abspath $(LIBLUCET_RUNTIME_C)/build):$(LD_LIBRARY_PATH) cargo test
	LD_LIBRARY_PATH=$(abspath $(LIBLUCET_RUNTIME_C)/build):$(LD_LIBRARY_PATH) cargo test

$(BUILD_DEPS):
	make -C $(LIBLUCET_RUNTIME_C)

$(TEST_DEPS):
	make -C $(LIBLUCET_RUNTIME_C) test

.PHONY: clean
clean:
	-rm -fr target lucet-sys/target
