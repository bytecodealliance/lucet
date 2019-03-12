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

# Setting a consistent LD_LIBRARY_PATH across the entire environment prevents unnecessary Cargo
# rebuilds.
#
# TODO: remove these first two paths once the C runtime and lucet-libc, respectively, are deprecated
ENV LD_LIBRARY_PATH=/lucet/lucet-runtime-c/build:/lucet/lucet-libc/build/lib:/usr/local/lib

RUN curl -L -O https://static.rust-lang.org/dist/rust-1.31.0-x86_64-unknown-linux-gnu.tar.gz \
	&& tar xzf rust-1.31.0-x86_64-unknown-linux-gnu.tar.gz \
	&& cd rust-1.31.0-x86_64-unknown-linux-gnu \
	&& ./install.sh \
	&& cd .. \
	&& rm -rf rust-1.31.0-x86_64-unknown-linux-gnu rust-1.31.0-x86_64-unknown-linux-gnu.tar.gz
ENV PATH=/usr/local/bin:$PATH
RUN cargo install --root /usr/local cargo-audit cargo-watch

