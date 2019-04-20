
LIBBUILTINS_OBJS:=$(addprefix build/, \
	memcpy.o \
	memmove.o \
	strcasecmp.o \
	strchr.o \
	strcmp.o \
	strlen.o )

COMMON_CFLAGS:= --std=gnu99 -Ofast -Wall -Werror -march=native -fPIC \
	-I../lucet-runtime/include

default: build/libbuiltins.so

build/%.o: src/%.c
	@mkdir -p $(@D)
	$(CC) $(COMMON_CFLAGS) -c $^ -o $@

build/libbuiltins.so: $(LIBBUILTINS_OBJS)
	# -U is not supported by the GNU linker. Retry without this option if the first command fails.
	$(CC) -Wl,-U,_lucet_vmctx_current_memory -Wl,-U,_lucet_vmctx_get_heap -shared $^ -o $@ 2> /dev/null || \
	$(CC) -shared $^ -o $@

.PHONY: clean
clean:
	rm -rf build
