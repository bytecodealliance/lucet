FROM ubuntu:xenial

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
	&& rm -rf /var/lib/apt/lists/*

RUN update-alternatives --install /usr/bin/clang clang /usr/bin/clang-6.0 100

# Setting a consistent LD_LIBRARY_PATH across the entire environment prevents unnecessary Cargo
# rebuilds.
ENV LD_LIBRARY_PATH=/usr/local/lib

RUN curl https://sh.rustup.rs -sSf | \
    sh -s -- --default-toolchain nightly-2019-09-25 -y && \
        /root/.cargo/bin/rustup update nightly
ENV PATH=/root/.cargo/bin:$PATH

RUN rustup component add rustfmt --toolchain nightly-2019-09-25-x86_64-unknown-linux-gnu
RUN rustup target add wasm32-wasi

RUN cargo install --debug cargo-audit cargo-watch rsign2 cargo-deb

RUN curl -sS -L -O https://github.com/CraneStation/wasi-sdk/releases/download/wasi-sdk-7/wasi-sdk_7.0_amd64.deb \
	&& dpkg -i wasi-sdk_7.0_amd64.deb && rm -f wasi-sdk_7.0_amd64.deb

ENV WASI_SDK=/opt/wasi-sdk

ENV BINARYEN_DIR=/opt/binaryen
ENV BINARYEN_VERSION=86
RUN curl -sS -L "https://github.com/WebAssembly/binaryen/archive/version_${BINARYEN_VERSION}.tar.gz" | tar xzf - && \
    mkdir -p binaryen-build && ( cd binaryen-build && cmake "../binaryen-version_${BINARYEN_VERSION}" && \
    make wasm-opt wasm-reduce ) && \
    install -d -v "${BINARYEN_DIR}/bin" && \
    for tool in wasm-opt wasm-reduce; do install -v "binaryen-build/bin/${tool}" "${BINARYEN_DIR}/bin/"; done && \
    rm -fr binaryen-build binaryen-version_${BINARYEN_VERSION}
ENV PATH=$BINARYEN_DIR:$PATH
