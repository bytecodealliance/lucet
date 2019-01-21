

LUCETC_BIN_PATH=$(abspath ../../lucetc/target/debug/lucetc)
LIBC_PATH=$(abspath ../../lucet-libc)

LUCETCC:=../../lucetc/scripts/bin/lucetcc --verbose
LUCETLD:=../../lucetc/scripts/bin/lucetld --verbose

LIBLUCET_RUNTIME_C:=../../lucet-runtime-c
LIBLUCET_BACKTRACE:=..

LUCET_TEST_OBJECTS:= $(addprefix build/, \
	lucet_test.o \
	backtrace_suite.o \
	)

GUESTS:= $(addprefix build/guests/, \
	test.so \
	)

CFLAGS:= -Wall -Werror --std=c99 -g3 -fPIC -D_GNU_SOURCE -fvisibility=default


default: $(LUCETC_BIN_PATH)
default: build/lucet_test
default: $(GUESTS)
	build/lucet_test

build/guests/%.so: build/guests/a.o build/guests/b.o
	@mkdir -p $(@D)
	$(LUCETLD) -shared $^ -o $@ --bindings=guests/bindings.json

build/guests/%.o: guests/%.c
	@mkdir -p $(@D)
	$(LUCETCC) $(CFLAGS) -c $< -o $@

build/%.o: %.c
	@mkdir -p $(@D)
	$(CC) $(CFLAGS) -I$(LIBLUCET_RUNTIME_C)/include -I$(LIBLUCET_BACKTRACE)/include -c $< -o $@

build/lucet_test: $(LUCET_TEST_OBJECTS) $(LIBLUCET_RUNTIME_C)/build/liblucet-runtime-c.a $(LIBLUCET_BACKTRACE)/build/liblucet-backtrace.a
	LIBRARY_PATH=$(LD_LIBRARY_PATH) \
		$(CC) $(CFLAGS) -rdynamic -pthread -o $@ $^ -ldl -lunwind

$(LIBLUCET_BACKTRACE)/build/liblucet-backtrace.a:
	make -C $(LIBLUCET_BACKTRACE)

$(LIBLUCET_RUNTIME_C)/build/liblucet-runtime-c.a:
	make -C $(LIBLUCET_RUNTIME_C)

$(LUCETC_BIN_PATH):
	cd ../../lucetc/ && cargo build

.PHONY: clean
clean:
	-rm -rf build
