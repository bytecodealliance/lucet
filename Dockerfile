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
    sh -s -- --default-toolchain 1.36.0 -y && \
        /root/.cargo/bin/rustup update nightly
ENV PATH=/root/.cargo/bin:$PATH

RUN rustup component add rustfmt --toolchain 1.36.0-x86_64-unknown-linux-gnu
RUN rustup target add wasm32-wasi

RUN cargo install cargo-audit cargo-watch rsign2

RUN curl -sS -L -O https://github.com/CraneStation/wasi-sdk/releases/download/wasi-sdk-5/wasi-sdk_5.0_amd64.deb \
	&& dpkg -i wasi-sdk_5.0_amd64.deb && rm -f wasi-sdk_5.0_amd64.deb

ENV WASI_SDK=/opt/wasi-sdk
