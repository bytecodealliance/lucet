FROM ubuntu:20.04

# This env variable makes sure installing the tzdata package doesn't hang in prompt
ENV DEBIAN_FRONTEND=noninteractive

# This env variable makes sure installing the tzdata package doesn't hang in prompt
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update \
	&& apt-get install -y --no-install-recommends \
	build-essential \
	curl \
	git \
	libbsd-dev \
	doxygen \
	sphinx-doc \
	cmake \
	ninja-build \
	ca-certificates \
	software-properties-common \
	libssl-dev \
	pkg-config \
	csmith \
	libcsmith-dev \
	creduce \
	gcc-multilib \
	clang \
	llvm \
	lld \
	wabt \
	&& rm -rf /var/lib/apt/lists/*

RUN update-alternatives --install /usr/bin/wasm-ld wasm-ld /usr/bin/wasm-ld-10 100

# Setting a consistent LD_LIBRARY_PATH across the entire environment prevents unnecessary Cargo
# rebuilds.
ENV LD_LIBRARY_PATH=/usr/local/lib

# Install our supported version of Rust, rustfmt, and the wasm32-wasi cross-compilation target
RUN curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain 1.43.1 -y
ENV PATH=/root/.cargo/bin:$PATH
RUN rustup component add rustfmt
RUN rustup target add wasm32-wasi

# Optional additional Rust programs
RUN cargo install --debug rsign2 cargo-audit mdbook

RUN curl -sS -L https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-10/libclang_rt.builtins-wasm32-wasi-10.0.tar.gz | \
	tar x -zf - -C /usr/lib/llvm-10/lib/clang/10.0.0

RUN curl -sS -L https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-10/wasi-sysroot-10.0.tar.gz | \
	tar x -zf - -C /opt

ENV WASI_SYSROOT=/opt/wasi-sysroot
ENV CLANG=/usr/bin/clang
ENV CLANG_ROOT=/usr/lib/llvm-10/lib/clang/10.0.0

# optional install of wasm-opt and wasm-reduce for fuzzing and benchmarking
ENV BINARYEN_DIR=/opt/binaryen
ENV BINARYEN_VERSION=93
RUN curl -sS -L "https://github.com/WebAssembly/binaryen/releases/download/version_${BINARYEN_VERSION}/binaryen-version_${BINARYEN_VERSION}-x86_64-linux.tar.gz" | tar xzf - && \
	install -d -v "${BINARYEN_DIR}/bin" && \
	for tool in wasm-opt wasm-reduce; do install -v "binaryen-version_${BINARYEN_VERSION}/${tool}" "${BINARYEN_DIR}/bin/"; done && \
	rm -fr binaryen-version_${BINARYEN_VERSION}
ENV PATH=$BINARYEN_DIR/bin:$PATH
