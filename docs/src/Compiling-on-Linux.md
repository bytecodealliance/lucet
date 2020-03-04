# Compiling on Linux

We successfully compiled Lucet on Arch Linux, Fedora, Gentoo and Ubuntu. Only x86_64 CPUs are
supported at this time.

## Installation on Ubuntu, with a sidecar installation of LLVM/clang

The following instructions only work on Ubuntu. They install a recent version of LLVM and `clang`
(in `/opt/wasi-sdk`), so that WebAssembly code can be compiled on Ubuntu versions older than 19.04.

First, the `curl` and `cmake` package must be installed:

```sh
apt install curl ca-certificates
```

You will need to install `wasi-sdk` as well. Note that you may need to run `dpkg` with elevated
privileges to install the package.

```sh
curl -sS -L -O https://github.com/CraneStation/wasi-sdk/releases/download/wasi-sdk-8/wasi-sdk_8.0_amd64.deb \
    && dpkg -i wasi-sdk_8.0_amd64.deb && rm -f wasi-sdk_8.0_amd64.deb
```

Install the latest stable version of the Rust compiler:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

Enter your clone of the Lucet repository, and then fetch/update the submodules:

```sh
cd lucet

git submodule update --init --recursive
```

Finally, compile the toolchain:

```sh
make install
```

In order to use `clang` to compile WebAssembly code, you need to adjust your `PATH` to use tools
from `/opt/wasi-sdk/bin` instead of the system compiler. Or use set of commands prefixed by
`wasm-wasi-`, such as `wasm32-wasi-clang` instead of `clang`.

## Installation on any recent Linux system, using the base compiler

Support for WebAssembly was introduced in LLVM 8, released in March 2019.

As a result, Lucet can be compiled with an existing LLVM installation, provided that it is up to
date. Most distributions now include LLVM 8 or LLVM 9, so that an additional installation is not
required to compile to WebAssembly .

On distributions such as Ubuntu (19.04 or newer) and Debian (bullseye or newer), the following
command installs the prerequisites:

```sh
apt install curl ca-certificates clang lld cmake
```

On Arch Linux:

```sh
pacman -S curl clang lld cmake
```

Next, install the WebAssembly compiler builtins:

```sh
curl -sL https://github.com/CraneStation/wasi-sdk/releases/download/wasi-sdk-8/libclang_rt.builtins-wasm32-wasi-8.0.tar.gz | \
sudo tar x -zf - -C /usr/lib/llvm-*/lib/clang/*
```

Install the latest stable version of the Rust compiler:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

Install the WASI sysroot:

```sh
mkdir -p /opt
curl -L https://github.com/CraneStation/wasi-sdk/releases/download/wasi-sdk-8/wasi-sdk-8.0-linux.tar.gz | \
sudo tar x -zv -C /opt -f - wasi-sdk-8.0/share && \
  sudo ln -s /opt/wasi-sdk-*/share/wasi-sysroot /opt/wasi-sysroot
```

Enter your clone of the Lucet repository, and then fetch/update the submodules:

```sh
cd lucet

git submodule update --init --recursive
```

Set the `LLVM` path:

```sh
export LLVM_BIN=/usr/lib/llvm-*/bin
```

Finally, install the Lucet toolchain with:

```sh
make install
```

The standard system compiler can be used to compile to WebAssembly, simply by adding
`--host=wasm32-wasi` to the compilation flags.
