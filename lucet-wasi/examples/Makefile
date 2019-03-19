WASI_CC ?= /opt/wasi-sdk/bin/clang

default: run-hello

.PHONY: run-hello
run-hello: build/hello.so
	cargo run -p lucet-wasi -- ./build/hello.so
	cargo run -p lucet-wasi -- ./build/hello.so -- "makefile user"
	GREETING="goodbye" cargo run -p lucet-wasi -- ./build/hello.so -- "makefile user"

build/hello.so: build/hello.wasm ../bindings.json
	cargo run -p lucetc -- $< --bindings ../bindings.json -o $@

build/hello.wasm: hello.c
	mkdir -p build
	$(WASI_CC) $< -o $@

build/hello.wat: build/hello.wasm
	wasm2wat -f $< > $@

.PHONY: clean
clean:
	@rm -rf build
