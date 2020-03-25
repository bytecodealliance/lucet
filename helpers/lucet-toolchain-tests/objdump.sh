#! /bin/sh

LUCET_DIR="."
TMPDIR="$(mktemp -d)"

if [ -x "${LUCET_DIR}/target/release/lucetc" ]; then
    LUCETC="${LUCET_DIR}/target/release/lucetc"
elif [ -x "${LUCET_DIR}/target/debug/lucetc" ]; then
    LUCETC="${LUCET_DIR}/target/debug/lucetc"
else
    echo "lucetc not found" >&2
    exit 1
fi

if [ -x "${LUCET_DIR}/target/release/lucet-objdump" ]; then
    LUCET_OBJDUMP="${LUCET_DIR}/target/release/lucet-objdump"
elif [ -x "${LUCET_DIR}/target/debug/lucet-objdump" ]; then
    LUCET_OBJDUMP="${LUCET_DIR}/target/debug/lucet-objdump"
else
    echo "lucet-objdump not found" >&2
    exit 1
fi

OBJ="$TMPDIR/objdump_test.so"

echo "Compiling a test WebAssembly module"

"$LUCETC" -o "$OBJ" lucetc/tests/wasm/icall_sparse.wat

echo "objdump'ing the compiled module"
if ! "$LUCET_OBJDUMP" "$OBJ" > /dev/null; then
  echo "lucet-objdump exited with $?"
  exit 1
fi

rm -rf "$TMPDIR"
