include Makefile.vars

default: build/liblucet-runtime-c.so
default: build/liblucet-runtime-c.a

states.pdf: states.dot
	dot -Tpdf $^ -o $@

build/liblucet-runtime-c.so: $(LUCET_OBJECTS)
	$(CC) -shared -o $@ $^

build/liblucet-runtime-c.a: $(LUCET_OBJECTS)
	ar $(ARFLAGS) $@ $^

build/%.o: src/%.c
	@mkdir -p $(@D)
	 $(CC) $(CFLAGS) -Iinclude -c $< -o $@

build/%.o: src/%.S
	@mkdir -p $(@D)
	$(CC) $(ASMFLAGS) -c $< -o $@

.PHONY: test
test: build/liblucet-runtime-c.so
	make -C test
	make -C bench

.PHONY: docs
docs:
	make -C docs html

.PHONY: bench
bench: build/liblucet-runtime-c.a
	make -C bench

.PHONY: clean
clean:
	-rm -rf build
	make -C test clean
	make -C bench clean
	make -C docs clean
