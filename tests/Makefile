
include Makefile.vars

LIBLUCET_RUNTIME_C:=../lucet-runtime-c
LIBLUCET_BACKTRACE:=../lucet-backtrace
LUCETC:=../lucetc/target/debug/lucetc
LUCETCC:=../lucetc/scripts/bin/lucetcc
LUCETLD:=../lucetc/scripts/bin/lucetld

LUCET_LIBC_PATH=../lucet-libc
LUCET_LIBC_HOST_LIB_PATH=$(LUCET_LIBC_PATH)/build/lib/liblucet_libc.so

HOST_GUEST_OBJECTS:= $(addprefix build/host_guests/, \
	null.so \
	oob.so \
	hello.so \
	hostcall_error.so \
	fpe.so \
	)

HOST_GUEST_BINDINGS:= --no-std-bindings --bindings=host_guests/bindings.json

SESSION_GUEST_OBJECTS:= $(addprefix build/session_guests/, \
	hello.so \
	alloc.so \
	stdio.so \
	)

ENTRYPOINT_GUEST_OBJECTS:= $(addprefix build/entrypoint_guests/, \
	calculator.so \
	use_allocator.so \
	ctype.so \
	callback.so \
	)

ENTRYPOINT_GUEST_BINDINGS := --bindings=entrypoint_guests/bindings.json

MEMORY_GUEST_OBJECTS:= $(addprefix build/memory_guests/, \
	current_memory.so \
	grow_memory.so \
	musl_alloc.so \
	)

SESSION_GUEST_BINDINGS:= --bindings=session_guests/bindings.json

LUCET_GUEST_LIBRARIES:= $(LUCET_GUEST_NATIVE_LIBRARIES)

VARNISH_GUEST_OBJECTS:= $(addprefix build/strcmp_guests/, \
	guest.so \
	fault_guest.so \
	)

VARNISH_GUEST_BINDINGS:= --bindings=strcmp_guests/bindings.json

STACK_GUEST_OBJECTS:= $(addprefix build/stack_guests/, \
	locals_64.so \
	locals_1page.so \
	locals_multipage.so \
	)

START_GUEST_OBJECTS:= $(addprefix build/start_guests/, \
	global_init.so \
	start_and_call.so \
	no_start.so \
	)

GLOBALS_GUEST_OBJECTS:= $(addprefix build/globals_guests/, \
	definition.so \
	import.so \
	)


.PHONY: default
default: test

.PHONY: guests
guests: $(HOST_GUEST_OBJECTS)
guests: $(SESSION_GUEST_OBJECTS)
guests: $(ENTRYPOINT_GUEST_OBJECTS)
guests: $(MEMORY_GUEST_OBJECTS)
guests: $(VARNISH_GUEST_OBJECTS)
guests: $(STACK_GUEST_OBJECTS)
guests: $(START_GUEST_OBJECTS)
guests: $(GLOBALS_GUEST_OBJECTS)

.PHONY: build
build: guests
build: build/lucet_test

.PHONY: test
test: check_mmap

.PHONY: check_mmap
check_mmap: build/lucet_test guests
	build/lucet_test


# Host guests have a simple build process - each C file turns into a single guest

build/host_guests/%.o: host_guests/%.c
	@mkdir -p $(@D)
	$(LUCETCC) -c $< -o $@

build/host_guests/%.wasm: build/host_guests/%.o
	$(LUCETLD) $^ -o $@

build/host_guests/%.so: build/host_guests/%.wasm
	$(LUCETC) $(HOST_GUEST_BINDINGS) $< -o $@

build/session_guests/%.o: session_guests/%.c
	@mkdir -p $(@D)
	$(LUCETCC) -I. -c $< -o $@

build/session_guests/%.wasm: build/session_guests/%.o
	$(LUCETLD) $^ -o $@

build/session_guests/%.so: build/session_guests/%.wasm
	$(LUCETC) $(SESSION_GUEST_BINDINGS) $< -o $@

build/entrypoint_guests/%.o: entrypoint_guests/%.c
	@mkdir -p $(@D)
	$(LUCETCC) -I. -c $< -o $@

build/entrypoint_guests/%.wasm: build/entrypoint_guests/%.o
	$(LUCETLD) $^ -o $@

build/entrypoint_guests/%.so: build/entrypoint_guests/%.wasm
	@mkdir -p $(@D)
	$(LUCETC) $(ENTRYPOINT_GUEST_BINDINGS) $< -o $@

build/entrypoint_guests/%.so: entrypoint_guests/%.wat
	@mkdir -p $(@D)
	$(LUCETC) $< -o $@

build/memory_guests/%.o: memory_guests/%.c
	@mkdir -p $(@D)
	$(LUCETCC) -c $< -o $@

build/memory_guests/%.wasm: build/memory_guests/%.o build/memory_guests/observe.o
	$(LUCETLD) $^ -o $@

build/memory_guests/%.so: build/memory_guests/%.wasm
	$(LUCETC) $< -o $@

build/memory_guests/%.so: memory_guests/%.wat
	@mkdir -p $(@D)
	$(LUCETC) $< -o $@

build/strcmp_guests/%.o: strcmp_guests/%.c
	@mkdir -p $(@D)
	$(LUCETCC) -c $< -o $@

build/strcmp_guests/%.wasm : build/strcmp_guests/%.o
	$(LUCETLD) $^ -o $@

build/strcmp_guests/%.so: build/strcmp_guests/%.wasm
	$(LUCETC) $(VARNISH_GUEST_BINDINGS) $< -o $@

build/stack_guests/locals_64.wat:
	@mkdir -p $(@D)
	python stack_guests/generate.py 64 > $@

build/stack_guests/locals_1page.wat:
	@mkdir -p $(@D)
	python stack_guests/generate.py 1050 > $@

build/stack_guests/locals_multipage.wat:
	@mkdir -p $(@D)
	python stack_guests/generate.py 5000 > $@

build/stack_guests/%.so: build/stack_guests/%.wat
	@mkdir -p $(@D)
	$(LUCETC) $< -o $@

build/start_guests/%.so: start_guests/%.wat
	@mkdir -p $(@D)
	$(LUCETC) $< -o $@

build/globals_guests/%.so: globals_guests/%.wat
	@mkdir -p $(@D)
	$(LUCETC) $< -o $@

build/%.o: %.c
	@mkdir -p $(@D)
	$(CC) $(CFLAGS) \
		-I$(LIBLUCET_RUNTIME_C)/include \
		-I$(LIBLUCET_BACKTRACE)/include \
		-I$(LUCET_LIBC_PATH)/build/include \
		-c $< -o $@

build/lucet_test: $(LUCET_LIBC_HOST_LIB_PATH)
build/lucet_test: $(LIBLUCET_RUNTIME_C)/build/liblucet-runtime-c.so
build/lucet_test: $(LIBLUCET_BACKTRACE)/build/liblucet-backtrace.so
build/lucet_test: $(LUCET_TEST_OBJECTS)
	LIBRARY_PATH=$(LD_LIBRARY_PATH) \
		$(CC) -rdynamic -pthread \
		-L$(LIBLUCET_RUNTIME_C)/build/ -Wl,-rpath=$(LIBLUCET_RUNTIME_C)/build/ \
		-L$(LIBLUCET_BACKTRACE)/build/ -Wl,-rpath=$(LIBLUCET_BACKTRACE)/build/ \
		-o $@ $^ -ldl -lunwind

$(LIBLUCET_RUNTIME_C)/build/liblucet-runtime-c.so:
	make -C $(LIBLUCET_RUNTIME_C)

.PHONY: clean
clean:
	-rm -rf build
