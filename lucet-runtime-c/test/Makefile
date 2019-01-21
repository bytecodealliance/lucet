
include Makefile.vars

LIBLUCET_RUNTIME_C:=..

.PHONY: default
default: check
default: guests
default: build/lucet_test

.PHONY: guests
guests: $(DATA_SEGMENT_GUESTS)
guests: $(GUEST_FAULT_GUESTS)
guests: $(FEATURES_GUESTS)
guests: $(ENTRYPOINT_GUESTS)
guests: $(STATS_GUESTS)
guests: $(GLOBALS_GUESTS)

check: guests build/lucet_test
	./build/lucet_test

%.so: %.o
	$(CC) -shared $< -o $@

build/data_segment:
	mkdir -p build/data_segment

build/data_segment/%.o: data_segment/%.c
	@mkdir -p $(@D)
	$(CC) $(CFLAGS) -I$(LIBLUCET_RUNTIME_C)/include  -c $< -o $@

build/guest_faults/%.o: guest_faults/%.c
	@mkdir -p $(@D)
	$(CC) $(CFLAGS) -I$(LIBLUCET_RUNTIME_C)/include --std=gnu99 -c $< -o $@

build/features/%.o: features/%.c
	@mkdir -p $(@D)
	$(CC) $(CFLAGS) -I$(LIBLUCET_RUNTIME_C)/include -c $< -o $@

build/entrypoints/%.o: entrypoints/%.c
	@mkdir -p $(@D)
	$(CC) $(CFLAGS) -I$(LIBLUCET_RUNTIME_C)/include -c $< -o $@

build/stats/%.o: stats/%.c
	@mkdir -p $(@D)
	$(CC) $(CFLAGS) -I$(LIBLUCET_RUNTIME_C)/include --std=gnu99 -c $< -o $@

build/%.o: %.c
	@mkdir -p $(@D)
	$(CC) $(CFLAGS) -I$(LIBLUCET_RUNTIME_C)/include -c $< -o $@

build/lucet_test: $(LUCET_TEST_OBJECTS) ../build/liblucet-runtime-c.a
	$(CC) $(CFLAGS) -rdynamic -pthread -o $@ $^ -ldl

../build/liblucet-runtime-c.a:
	make -C ..

# returnchild is a test program for the context code. Its not a greatest test - it just
# prints to stdout to observe what happened. It builds directly against the lucet_context
# sources so that it doesnt have dependencies on libunwind or dynamic code loading.

build/returnchild: returnchild.c ../src/lucet_context.c ../src/lucet_context_asm.S
	@mkdir -p $(@D)
	$(CC) $(CFLAGS) -I$(LIBLUCET_RUNTIME_C)/include -o $@ $^

.PHONY: clean
clean:
	-rm -rf build
