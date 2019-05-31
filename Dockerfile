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

RUN curl -sS -L -O https://static.rust-lang.org/dist/rust-1.35.0-x86_64-unknown-linux-gnu.tar.gz \
	&& tar xzf rust-1.35.0-x86_64-unknown-linux-gnu.tar.gz \
	&& cd rust-1.35.0-x86_64-unknown-linux-gnu \
	&& ./install.sh \
	&& cd .. \
	&& rm -rf rust-1.35.0-x86_64-unknown-linux-gnu rust-1.35.0-x86_64-unknown-linux-gnu.tar.gz
ENV PATH=/usr/local/bin:$PATH
RUN cargo install --root /usr/local cargo-audit cargo-watch

RUN curl -sS -L -O https://github.com/CraneStation/wasi-sdk/releases/download/wasi-sdk-5/wasi-sdk_5.0_amd64.deb \
	&& dpkg -i wasi-sdk_5.0_amd64.deb && rm -f wasi-sdk_5.0_amd64.deb

ENV WASI_SDK=/opt/wasi-sdk
