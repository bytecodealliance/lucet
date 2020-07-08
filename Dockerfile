FROM ubuntu:bionic

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
	python-sphinx \
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
	clang-6.0 \
	llvm-6.0 \
	&& rm -rf /var/lib/apt/lists/*

RUN update-alternatives --install /usr/bin/clang clang /usr/bin/clang-6.0 100
RUN update-alternatives --install /usr/bin/llvm-config llvm-config /usr/bin/llvm-config-6.0 100

# Setting a consistent LD_LIBRARY_PATH across the entire environment prevents unnecessary Cargo
# rebuilds.
ENV LD_LIBRARY_PATH=/usr/local/lib

# Install our supported version of Rust, rustfmt, and the wasm32-wasi cross-compilation target
RUN curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain 1.44.1 -y
ENV PATH=/root/.cargo/bin:$PATH
RUN rustup component add rustfmt
RUN rustup target add wasm32-wasi

# Optional additional Rust programs
RUN cargo install --debug rsign2 cargo-audit mdbook

RUN curl -sSLO https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-11/wasi-sdk_11.0_amd64_ubuntu20.04.deb \
    && dpkg -i wasi-sdk_11.0_amd64_ubuntu20.04.deb \
    && rm -f wasi-sdk_11.0_amd64_ubuntu20.04.deb

ENV WASI_SDK=/opt/wasi-sdk

# optional install of wasm-opt and wasm-reduce for fuzzing and benchmarking
ENV BINARYEN_DIR=/opt/binaryen
ENV BINARYEN_VERSION=86
RUN curl -sS -L "https://github.com/WebAssembly/binaryen/releases/download/version_${BINARYEN_VERSION}/binaryen-version_${BINARYEN_VERSION}-x86_64-linux.tar.gz" | tar xzf - && \
    install -d -v "${BINARYEN_DIR}/bin" && \
    for tool in wasm-opt wasm-reduce; do install -v "binaryen-version_${BINARYEN_VERSION}/${tool}" "${BINARYEN_DIR}/bin/"; done && \
    rm -fr binaryen-version_${BINARYEN_VERSION}
ENV PATH=$BINARYEN_DIR/bin:$PATH
