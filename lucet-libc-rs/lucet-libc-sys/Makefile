
LIBLUCET_RUNTIME_C:=../../lucet-runtime-c
LUCET_LIBC:=../../lucet-libc

LUCET_LIBRARY_PATH=$(abspath $(LIBLUCET_RUNTIME_C)/build):$(abspath $(LUCET_LIBC)/build/lib)

.PHONY: build
build:
	LD_LIBRARY_PATH=$(LUCET_LIBRARY_PATH):$(LD_LIBRARY_PATH) cargo build

.PHONY: test
test:
	LD_LIBRARY_PATH=$(LUCET_LIBRARY_PATH):$(LD_LIBRARY_PATH) cargo test

.PHONY: clean
clean:
	rm -rf target

