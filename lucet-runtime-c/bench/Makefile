include Makefile.vars

LIBLUCET_RUNTIME_C:=..

default: build/bench
default: $(GUESTS)
	build/bench

%.so: %.o
	$(CC) -shared $< -o $@

build/guests/%.o: guests/%.c
	@mkdir -p $(@D)
	$(CC) $(CFLAGS) -I$(LIBLUCET_RUNTIME_C)/include --std=gnu99 -c $< -o $@

build/%.o: %.c
	@mkdir -p $(@D)
	$(CC) $(CFLAGS) -I$(LIBLUCET_RUNTIME_C)/include -c $< -o $@

build/bench: $(BENCH_OBJECTS) ../build/liblucet-runtime-c.a
	$(CC) $(CFLAGS) -rdynamic -pthread -o $@ $^ -lrt -ldl

../build/liblucet-runtime-c.a:
	make -C ..

.PHONY: clean
clean:
	-rm -rf build
