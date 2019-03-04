
LUCET_CLANG?=clang
LUCET_WASM_LD?=wasm-ld

GCC:=$(FST_GCC)/bin/gcc
LUCETC:=../../lucetc/target/release/lucetc
LUCETCC:=../../lucetc/scripts/bin/lucetcc
LUCETLD:=../../lucetc/scripts/bin/lucetld

LUCET_LIBC:=../../lucet-libc/
LUCET_RUNTIME:=../../lucet-runtime
SIGHTGLASS:=../../sightglass

LIBBUILTINS:=../../builtins/build/libbuiltins.so

SHOOTOUT:=../../sightglass/benchmarks/shootout
SHOOTOUT_SRCS:=$(shell ls $(SHOOTOUT)/*.c)
SHOOTOUT_NATIVE_OBJS:=
SHOOTOUT_LUCET_OBJS:=

LUCETC_FLAGS:=--opt-level best --reserved-size 4294967296
COMMON_CFLAGS:=--std=c99 -Ofast -Wall -W -I$(SIGHTGLASS)/include

SHOOTOUT_NATIVE_CFLAGS:=-march=native -fPIC \
	-DIMPL_REFERENCE -DUSE_LEND \
	-Dmalloc=lend_malloc -Dcalloc=lend_calloc -Dfree=lend_free

ifdef CI
	SIGHTGLASS_ARGS:=--quick
endif

default: run

.PHONY: bench
bench: run

.PHONY: run
run: build
	$(SIGHTGLASS)/target/release/sightglass -c sightglass.toml $(SIGHTGLASS_ARGS)

$(LUCET_LIBC)/build/lib/liblucet_libc.so:
	make -C ../../lucet-libc

.PHONY: $(LUCET_RUNTIME)/target/release/liblucet_runtime.so
$(LUCET_RUNTIME)/target/release/liblucet_runtime.so:
	make -C $(LUCET_RUNTIME) release

.PHONY: build
build: $(LUCETC)
build: build/native/implementation.so
build: build/lucet/implementation.so
build: build/lucet/module.so
build: $(SIGHTGLASS)/target/release/sightglass

build/native/shootout/%.o: $(SHOOTOUT)/%.c
	@mkdir -p $(@D)
	$(LUCET_CLANG) $(SHOOTOUT_NATIVE_CFLAGS) $(COMMON_CFLAGS) -c $^ -o $@

build/native/implementation.so: $(patsubst %.c, %.o, $(addprefix build/native/shootout/, $(notdir $(SHOOTOUT_SRCS))))
	@mkdir -p $(@D)
	$(LUCET_CLANG) -shared -o $@ $^

build/lucet/shootout/%.o: $(SHOOTOUT)/%.c
	@mkdir -p $(@D)
	$(LUCETCC) $(COMMON_CFLAGS) -c $^ -o $@

build/lucet/module.wasm: $(patsubst %.c, %.o, $(addprefix build/lucet/shootout/, $(notdir $(SHOOTOUT_SRCS))))
	@mkdir -p $(@D)
	$(LUCETLD) $^ -o $@

# Don't emit the shared object, we need a saparate link step below.
build/lucet/module.o: build/lucet/module.wasm $(LIBBUILTINS)
	@mkdir -p $(@D)
	$(LUCETC) $(LUCETC_FLAGS) --emit=obj --bindings=bindings.json --builtins=$(LIBBUILTINS) build/lucet/module.wasm -o $@

build/lucet/module.clif: build/lucet/module.wasm $(LIBBUILTINS)
	@mkdir -p $(@D)
	$(LUCETC) $(LUCETC_FLAGS) --emit=clif --bindings=bindings.json --builtins=$(LIBBUILTINS) build/lucet/module.wasm -o $@

# Because liblucet_runtime is dynamically loaded (transitively) by implementation.so, we
# need to link the wasm object to the same libraries.
build/lucet/module.so: build/lucet/module.o
	@mkdir -p $(@D)
	$(LUCET_CLANG) -rdynamic -shared build/lucet/module.o -o $@

build/lucet/%.o: %.c
	@mkdir -p $(@D)
	$(LUCET_CLANG) --std=c99 -fPIC -D_GNU_SOURCE -g3 $(COMMON_CFLAGS) \
		-fvisibility=default \
		-I$(LUCET_RUNTIME)/include \
		-I$(LUCET_LIBC)/build/include \
		-DWASM_MODULE=$(abspath build/lucet/module.so) \
		-c $^ -o $@

build/lucet/implementation.so: $(LUCET_RUNTIME)/target/release/liblucet_runtime.so
build/lucet/implementation.so: $(LUCET_LIBC)/build/lib/liblucet_libc.so
build/lucet/implementation.so: build/lucet/wrapper.o
build/lucet/implementation.so: build/lucet/hostcalls.o
build/lucet/implementation.so: $(LIBBUILTINS)
	@mkdir -p $(@D)
	$(LUCET_CLANG) -rdynamic -shared \
		-L $(LUCET_LIBC)/build/lib \
		-Wl,-rpath $(LUCET_LIBC)/build/lib \
		-L $(dir $(LIBBUILTINS)) \
		-Wl,-rpath $(dir $(LIBBUILTINS)) \
		$^ -ldl -lbuiltins -o $@

$(SIGHTGLASS)/target/release/sightglass:
	cd $(SIGHTGLASS) && cargo build --release

$(LIBBUILTINS):
	make -C ../../builtins

$(LUCETC):
	cd ../../lucetc && cargo build --release

.PHONY: clean
clean:
	-rm -rf build
