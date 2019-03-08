#!/usr/bin/env sh

# generate the wasm file; we check in the text representation to make git happy
wat2wasm hello.wat -o hello.wasm

# make sure lucetc is built, then run it with the wasi bindings
(cd ../../../public/lucetc && cargo build)
../../../public/lucetc/target/debug/lucetc \
    hello.wasm \
    --no-std-bindings \
    --bindings ../wasi/bindings.json \
    -o hello.so

# make sure lucet-wasi is built, then run it on the hello module
(cd .. && cargo build)
../target/debug/lucet-wasi hello.so
