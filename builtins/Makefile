
LIBBUILTINS_OBJS:=$(addprefix build/, \
	memcpy.o \
	memmove.o \
	strcasecmp.o \
	strchr.o \
	strcmp.o \
	strlen.o )

COMMON_CFLAGS:= --std=c99 -Ofast -Wall -Werror -march=native -fPIC \
	-I../lucet-runtime-c/include

default: build/libbuiltins.so

build/%.o: src/%.c
	@mkdir -p $(@D)
	$(CC) $(COMMON_CFLAGS) -c $^ -o $@

build/libbuiltins.so: $(LIBBUILTINS_OBJS)
	$(CC) -shared $^ -o $@

.PHONY: clean
clean:
	rm -rf build
