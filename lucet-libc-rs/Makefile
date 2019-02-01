
LIBLUCET_RUNTIME_C:=../lucet-runtime-c
LUCET_LIBC:=../lucet-libc

LUCET_LIBRARY_PATH=$(abspath $(LIBLUCET_RUNTIME_C)/build):$(abspath $(LUCET_LIBC)/build/lib)

.PHONY: build
build: $(LUCET_LIBC)/build/lib/liblucet-libc.so
	LD_LIBRARY_PATH=$(LUCET_LIBRARY_PATH):$(LD_LIBRARY_PATH) cargo build

.PHONY: test
test: sys-test $(LUCET_LIBC)/build/lib/liblucet-libc.so
	LD_LIBRARY_PATH=$(LUCET_LIBRARY_PATH):$(LD_LIBRARY_PATH) cargo test

.PHONY: sys-test
sys-test:
	make -C lucet-libc-sys test

.PHONY: clean
clean: sys-clean
	rm -rf target

.PHONY: sys-clean
sys-clean:
	make -C lucet-libc-sys clean

.PHONY: audit
audit:
	cargo audit

$(LUCET_LIBC)/build/lib/liblucet-libc.so:
	make -C $(LUCET_LIBC)
