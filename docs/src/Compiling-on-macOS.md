# Compiling on macOS

## Prerequisites

Install `llvm`, `rust` and `cmake` using [Homebrew](https://brew.sh):

```sh
brew install llvm rust cmake
```

In order to compile applications written in C to WebAssembly, `clang` builtins need to be installed:

```sh
curl -sL https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-11/libclang_rt.builtins-wasm32-wasi-11.0.tar.gz | \
  tar x -zf - -C /usr/local/opt/llvm/lib/clang/10*
```

As well as the WASI sysroot:

```sh
sudo mkdir -p /opt

curl -sS -L https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-11/wasi-sysroot-11.0.tar.gz | \
  sudo tar x -zf - -C /opt
```

## Compiling and installing Lucet

Enter the Lucet git repository clone, and fetch/update the submodules:

```sh
cd lucet

git submodule update --init --recursive
```

Define the location of the WASI sysroot installation:

```sh
export WASI_SYSROOT=/opt/wasi-sysroot
```

Finally, compile and install the toolchain:

```sh
env LUCET_PREFIX=/opt/lucet make install
```

Change `LUCET_PREFIX` to the directory you would like to install Lucet into. `/opt/lucet` is the default directory.

## Setting up the environment

In order to add `/opt/lucet` to the command search path, as well register the library path for the Lucet runtime, the following command can be run interactively or added to the shell startup files:

```sh
source /opt/lucet/bin/setenv.sh
```

## Running the test suite

If you want to run the test suite, and in addition to `WASI_SYSROOT`, the following environment variables must be set:

```sh
export CLANG_ROOT="$(echo /usr/local/opt/llvm/lib/clang/10*)"
export CLANG=/usr/local/opt/llvm/bin/clang
```

And the test suite can then be run with the following command:

 ```sh
 make test
```
