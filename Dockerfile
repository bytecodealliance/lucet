FROM ubuntu:xenial

RUN apt-get update \
 && apt-get install -y --no-install-recommends \
	build-essential \
	curl \
	git \
	libbsd-dev \
	libhwloc-dev \
	doxygen \
	python-sphinx \
	cmake \
	ninja-build \
	ca-certificates \
	software-properties-common \
	libssl-dev \
	pkg-config \
 && curl https://apt.llvm.org/llvm-snapshot.gpg.key | apt-key add - \
 && add-apt-repository "deb http://apt.llvm.org/xenial/ llvm-toolchain-xenial-7 main" \
 && apt-get update \
 && apt-get install -y \
	clang-7 \
	lld-7 \
	clang-format-7 \
 && rm -rf /var/lib/apt/lists/* \
 && update-alternatives --install /usr/bin/clang clang /usr/bin/clang-7 100 \
 && update-alternatives --install /usr/bin/clang++ clang++ /usr/bin/clang++-7 100 \
 && update-alternatives --install /usr/bin/wasm-ld wasm-ld /usr/bin/wasm-ld-7 100 \
 && update-alternatives --install /usr/bin/llvm-ar llvm-ar /usr/bin/llvm-ar-7 100 \
 && update-alternatives --install /usr/bin/clang-format clang-format /usr/bin/clang-format-7 100

# Xenial ships with libunwind 1.1, we need 1.2
RUN curl -L -O http://download.savannah.nongnu.org/releases/libunwind/libunwind-1.2.1.tar.gz \
	&& tar xzf libunwind-1.2.1.tar.gz \
	&& cd libunwind-1.2.1 \
	&& ./configure \
	&& make \
	&& make install \
	&& cd .. \
	&& rm -rf libunwind-1.2.1 libunwind-1.2.1.tar.gz
ENV LD_LIBRARY_PATH=/usr/local/lib

RUN curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain=1.31.0 -y
ENV PATH=/root/.cargo/bin:$PATH
RUN rustup component add rustfmt
RUN cargo install cargo-audit
