# Lucet-IDL

This is an IDL. A tool that reads descriptions of data types, and spits out these types' definitions, plus a bunch of functions to represent them as a serialized, platform-independent way.

## Usage

```text
USAGE:
    lucet-idl [FLAGS] [OPTIONS] --input <input_file>

FLAGS:
    -h, --help                    Prints help information
    -V, --version                 Prints version information
    -z, --zero-native-pointers    Do not serialize native pointers

OPTIONS:
    -b, --backend <backend>     Backend, one of: c [default: c]
    -i, --input <input_file>    Path to the input file
    -t, --target <target>       Target, one of: x86, x86_64, x86_64_32, generic [default: Generic]
```

## The description language in one example

```text
// Primitive types:
// `i8`, `i16`, `i32`, `i64`, `u8`, `u16`, `u32`, `u64`, `f32`, `f64`.

// Enumerations

enum color {
    red, blue, green
}

// Aliases

type colour = color
type col = colour

// Structures

struct st {
    a: i8,
    b: **i32,
    c: color,
    self: *st
}
```

## Sample output

[Output (C backend) for the example above](https://gist.github.com/jedisct1/db5f81aa5e21b280d6f0f0350215889e).

## Generated type definitions (C)

### Structures

No matter what the target and backend are, the generator will attempt to align data the same way as the reference target (C on x86_64).

Using the example above, the `st` structure above gets defined as:

```c
struct st {
    int8_t a;
    uint8_t ___pad8_1[7];
    int32_t **b;
    ___POINTER_PAD(8) // pad pointer `b` at offset 8 to match alignment of the reference target (8 bytes)
    color /* (enum ___color) */ c;
    struct st *self;
    ___POINTER_PAD(24) // pad pointer `self` at offset 24 to match alignment of the reference target (8 bytes)
};

#define BYTES_ST 32
```

Explicit padding has been added after the first element `a` so that `b` is 64-bit aligned.

However, native pointers can be 32-bit even though we require them to be 64-bit aligned.

The `___POINTER_PAD` macro adds extra padding after a pointer on platforms where pointers are not 64-bit long.

The `self` pointer will always be 64-bit aligned, as the previous member `c` is 64-bit aligned, and 64-bit long. Therefore, no extra padding is added before, but optional padding is added after the pointer itself.

For every structure, a macro representing the number of bytes required to store it is defined as `BYTES_<structure name>`, such as `BYTES_ST` in the example above.

### Enumerations

Enumerations are stored as a signed 64-bit integer:

```c
typedef int64_t color; // enum, should be in the [0...2] range
enum ___color {
    COLOR_RED, // 0
    COLOR_BLUE, // 1
    COLOR_GREEN, // 2
};

#define BYTES_COLOR 8
```


## Accessors (C)

The generated types are designed to be directly used by applications.

However, they can also be represented as platform-independent serialized data.

In particular, the `st` structure above generates the following accessors:

```c
static inline void store_st(unsigned char buf[static BYTES_ST], const struct st *v);

static inline void load_st(struct st *v_p, const unsigned char buf[static BYTES_ST]);
```

On platforms that can share the same endianness and alignment rules as the target platform,
these operations translate into a single `memcpy()` call.

On other platforms, individual values are re-aligned and byte-swapped accordingly.

Accessors for individual values are also generated.

Subsets of types can thus be directly loaded and modified from a serialized representation.

## Pointers

Pointers are not automatically dereferenced. Their value can be replaced with zeros in
serialized representations.

The `--zero-native-pointers` command-line option enables it.
