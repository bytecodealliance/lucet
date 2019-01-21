
include Makefile.vars

ifdef LUCET_LIBUNWIND_INCLUDE
	INCLUDES:=-I$(LUCET_LIBUNWIND_INCLUDE)
endif


default: build/liblucet-backtrace.so
default: build/liblucet-backtrace.a

build/liblucet-backtrace.so: $(LUCET_BACKTRACE_OBJECTS)
	@mkdir -p $(@D)
	LIBRARY_PATH=$(LD_LIBRARY_PATH) \
		$(CC) -shared -o $@ $^ \
		-L../lucet-runtime-c/build -llucet-runtime-c -lunwind

build/liblucet-backtrace.a: $(LUCET_BACKTRACE_OBJECTS)
	ar $(ARFLAGS) $@ $^

build/%.o: src/%.c
	@mkdir -p $(@D)
	 $(CC) $(CFLAGS) $(INCLUDES) -c $< -o $@

build/%.o: src/%.S
	@mkdir -p $(@D)
	$(CC) $(ASMFLAGS) -c $< -o $@

.PHONY: test
test: build/liblucet-backtrace.so
	make -C test

.PHONY: clean
clean:
	-rm -rf build
	make -C test clean
