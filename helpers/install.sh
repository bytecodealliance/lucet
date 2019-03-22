#! /bin/sh

LUCET_SRC_PREFIX=${LUCET_SRC_PREFIX:-"$(readlink -e $(dirname $(dirname ${0})))"}
if [ ! -x "${LUCET_SRC_PREFIX}/helpers/install.sh" ]; then
    echo "Unable to find the current script base directory" >&2
    exit 1
fi

LUCET_PREFIX=${LUCET_PREFIX:-"/opt/lucet"}
LUCET_SRC_PREFIX=${LUCET_SRC_PREFIX:-"/lucet"}
LUCET_SRC_RELEASE_DIR=${LUCET_SRC_RELEASE_DIR:-"${LUCET_SRC_PREFIX}/target/release"}
LUCET_BIN_DIR=${LUCET_BIN_DIR:-"${LUCET_PREFIX}/bin"}
LUCET_LIB_DIR=${LUCET_LIB_DIR:-"${LUCET_PREFIX}/lib"}
LUCET_LIBEXEC_DIR=${LUCET_LIBEXEC_DIR:-"${LUCET_PREFIX}/libexec"}
LUCET_SYSCONF_DIR=${LUCET_SYSCONF_DIR:-"${LUCET_PREFIX}/etc"}
LUCET_SHARE_DIR=${LUCET_SHARE_DIR:-"${LUCET_PREFIX}/share"}
LUCET_EXAMPLES_DIR=${LUCET_EXAMPLES_DIR:-"${LUCET_SHARE_DIR}/examples"}
LUCET_DOC_DIR=${LUCET_DOC_DIR:-"${LUCET_SHARE_DIR}/doc"}
LUCET_BUNDLE_DOC_DIR=${LUCET_BUNDLE_DOC_DIR:-"${LUCET_DOC_DIR}/lucet"}
WASI_PREFIX=${WASI_PREFIX:-${WASI_SDK:-"/opt/wasi-sdk"}}
WASI_BIN=${WASI_BIN:-"${WASI_PREFIX}/bin"}
WASI_SYSROOT=${WASI_SYSROOT:-"${WASI_PREFIX}/share/sysroot"}
WASI_TARGET=${WASI_TARGET:-"wasm32-unknown-wasi"}
WASI_BIN_PREFIX=${WASI_BIN_PREFIX:-"$WASI_TARGET"}

BINS="lucet-analyze lucet-wasi lucetc sightglass spec-test wasmonkey"
LIBS="liblucet_runtime.so"
DOCS="lucet-wasi/README.md sightglass/README.md"
BUNDLE_DOCS="README.md"

install -d -v "$LUCET_BIN_DIR"
for bin in $BINS; do
    install -p -v "${LUCET_SRC_RELEASE_DIR}/${bin}" "${LUCET_BIN_DIR}/${bin}"
done

install -d -v "$LUCET_LIB_DIR"
for lib in $LIBS; do
    install -p -v "${LUCET_SRC_RELEASE_DIR}/${lib}" "${LUCET_LIB_DIR}/${lib}"
done

install -d -v "$LUCET_LIBEXEC_DIR"
install -p -v "${LUCET_SRC_PREFIX}/lucet-builtins/build/libbuiltins.so" \
    "${LUCET_LIBEXEC_DIR}/libbuiltins.so"

lucet_setenv_file="$(mktemp)"
cat > "$lucet_setenv_file" << EOT
#! /bin/sh

export PATH="${LUCET_BIN_DIR}:${PATH}"
export LD_LIBRARY_PATH="${LUCET_LIB_DIR}:${LD_LIBRARY_PATH}"

if [ \$# -gt 0 ]; then
    exec \$@
fi
EOT

install -p -v "$lucet_setenv_file" "${LUCET_BIN_DIR}/lucet_setenv.sh"
rm -f "$lucet_setenv_file"

install -d -v "${LUCET_EXAMPLES_DIR}/sightglass"
install -p -v -m 0644 "${LUCET_SRC_PREFIX}/sightglass/sightglass.toml" "${LUCET_EXAMPLES_DIR}/sightglass/sightglass.toml"

install -d -v "${LUCET_SHARE_DIR}/lucet-wasi"
install -p -v -m 0644 "${LUCET_SRC_PREFIX}/lucet-wasi/bindings.json" "${LUCET_SHARE_DIR}/lucet-wasi/bindings.json"

for doc in $DOCS; do
    install -d -v "${LUCET_DOC_DIR}/$(dirname $doc)"
    install -p -v -m 0644 "$doc" "${LUCET_DOC_DIR}/${doc}"
done

for doc in $BUNDLE_DOCS; do
    install -d -v "${LUCET_BUNDLE_DOC_DIR}/$(dirname $doc)"
    install -p -v -m 0644 "$doc" "${LUCET_BUNDLE_DOC_DIR}/${doc}"
done

for file in clang clang++; do
    wrapper_file="$(mktemp)"
    cat > "$wrapper_file" << EOT
#! /bin/sh

exec "${WASI_BIN}/${file}" --target="$WASI_TARGET" --sysroot="$WASI_SYSROOT" \$@
EOT
    install -p -v "$wrapper_file" "${LUCET_BIN_DIR}/${WASI_BIN_PREFIX}-${file}"
    rm -f "$wrapper_file"
done

for file in ar dwarfdump nm ranlib size; do
    ln -sfv "${WASI_BIN}/llvm-${file}" "${LUCET_BIN_DIR}/${WASI_BIN_PREFIX}-${file}"
done

for file in ld; do
    ln -sfv "${WASI_BIN}/wasm-${file}" "${LUCET_BIN_DIR}/${WASI_BIN_PREFIX}-${file}"
done

wrapper_file="$(mktemp)"
cat > "$wrapper_file" << EOT
#! /bin/sh

exec "${LUCET_BIN_DIR}/lucetc" \$@ --bindings "${LUCET_SHARE_DIR}/lucet-wasi/bindings.json"
EOT
install -p -v "$wrapper_file" "${LUCET_BIN_DIR}/lucetc-wasi"
rm -f "$wrapper_file"
