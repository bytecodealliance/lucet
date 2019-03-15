default: run-hello

.PHONY: run-hello
run-hello: build/hello.so
	cargo run -p lucet-wasi -- ./build/hello.so

build/hello.so: build/hello.wasm ../wasi/bindings.json
	cargo run -p lucetc -- $< --bindings ../wasi/bindings.json -o $@

build/hello.wasm: hello.c
	mkdir -p build
	/opt/wasi-sdk/bin/clang $< -o $@

.PHONY: clean
clean:
	@rm -rf build
