#! /bin/sh

LUCET_DIR="."
LUCET_TARGET_DIR="${LUCET_DIR}/target/debug"
TMPDIR="$(mktemp -d)"

if [ -d "${LUCET_DIR}/target/release" ]; then
    LUCET_TARGET_DIR="${LUCET_DIR}/target/release"
fi

if ! command -v rsign >/dev/null; then
    cargo install rsign2
fi

echo "Creating a key pair to sign the WebAssembly code"
(
    echo x
    echo x
) | rsign generate -p "${TMPDIR}/src_public.key" -s "${TMPDIR}/src_secret.key" -f >/dev/null

echo "Signing the WebAssembly code"
cp "${LUCET_DIR}/lucetc/tests/wasm/call.wat" "${TMPDIR}/test.wat"
echo x | rsign sign -p "${TMPDIR}/src_public.key" -s "${TMPDIR}/src_secret.key" "${TMPDIR}/test.wat" >/dev/null

echo "Creating a key pair using lucetc for the compiled code"
if ! "${LUCET_TARGET_DIR}/lucetc" \
    --signature-keygen \
    --signature-pk="${TMPDIR}/public.key" \
    --signature-sk="raw:${TMPDIR}/secret.key"; then
    echo "Keypair generation failed" >&2
    exit 1
fi

echo "Trying to compile source code whose signature is invalid"
if "${LUCET_TARGET_DIR}/lucetc" \
    "${TMPDIR}/test.wat" \
    -o "${TMPDIR}/test.so" \
    --signature-verify \
    --signature-pk="${TMPDIR}/public.key" 2>/dev/null; then
    echo "Source signature verification with the wrong public key shouldn't have passed" >&2
    exit 1
fi

echo "Compiling the verified source code"
if ! "${LUCET_TARGET_DIR}/lucetc" \
    "${TMPDIR}/test.wat" \
    -o "${TMPDIR}/test.so" \
    --signature-verify \
    --signature-pk="${TMPDIR}/src_public.key" 2>/dev/null; then
    echo "Source signature verification with the correct public key didn't pass" >&2
    exit 1
fi

echo "Compiling the verified source code and embedding a signature into the resulting object"
if ! "${LUCET_TARGET_DIR}/lucetc" \
    "${TMPDIR}/test.wat" \
    -o "${TMPDIR}/test.so" \
    --signature-create \
    --signature-verify \
    --signature-pk="${TMPDIR}/src_public.key" \
    --signature-sk=raw:"${TMPDIR}/secret.key" 2>/dev/null; then
    echo "Compilation failed" >&2
    exit 1
fi

echo "Running the resulting object"
if ! "${LUCET_TARGET_DIR}/lucet-wasi" \
    "${TMPDIR}/test.so" \
    --signature-verify \
    --signature-pk="${TMPDIR}/public.key" \
    --entrypoint main; then
    echo "Runtime failed" >&2
    exit 1
fi

echo >>"${TMPDIR}/test.so"

echo "Trying to run a tampered version of the object"
if "${LUCET_TARGET_DIR}/lucet-wasi" \
    "${TMPDIR}/test.so" \
    --signature-verify \
    --signature-pk="${TMPDIR}/public.key" \
    --entrypoint main 2>/dev/null; then
    echo "Signature verification of tampered module shouldn't have passed" >&2
    exit 1
fi

rm -fr "$TMPDIR"

echo "Done."
