# Compiling on macOS

Install `llvm`, `rust` and `cmake` using [Homebrew](https://brew.sh):

```sh
brew install llvm rust cmake
```

In order to compile applications to WebAssembly, builtins need to be installed
as well:

```sh
curl -sL https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-10/libclang_rt.builtins-wasm32-wasi-10.0.tar.gz | \
  sudo tar x -zf - -C /usr/local/opt/llvm/lib/clang/10*
```

Install the WASI sysroot:

```sh
RUN curl -sS -L https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-10/wasi-sysroot-10.0.tar.gz | \
  tar x -zf - -C /opt
```

Enter the Lucet git repository clone, and fetch/update the submodules:

```sh
cd lucet

git submodule update --init
```

Set relevant environment variables:

```sh
export WASI_SYSROOT=/opt/wasi-sysroot
export CLANG_ROOT="$(echo /usr/local/opt/llvm/lib/clang/10*)"
export CLANG=/usr/local/opt/llvm/bin/clang
```

Finally, compile and install toolchain:

```sh
env LUCET_PREFIX=/opt/lucet make install
```

Change `LUCET_PREFIX` to the directory you would like to install Lucet into. `/opt/lucet` is the default directory.
The Lucet executable files can be found in the `target/release/` directory.
