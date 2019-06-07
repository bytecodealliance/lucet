#! /bin/sh

LUCET_SRC_PREFIX=${LUCET_SRC_PREFIX:-"$(
    cd $(dirname $(dirname ${0}))
    pwd -P
)"}
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
WASI_TARGET=${WASI_TARGET:-"wasm32-wasi"}
WASI_BIN_PREFIX=${WASI_BIN_PREFIX:-"$WASI_TARGET"}

if [ "$(uname -s)" = "Darwin" ]; then
    DYLIB_SUFFIX="dylib"
else
    DYLIB_SUFFIX="so"
fi

BINS="lucet-analyze lucet-wasi lucetc sightglass spec-test wasmonkey"
LIBS="liblucet_runtime.${DYLIB_SUFFIX}"
DOCS="lucet-wasi/README.md sightglass/README.md"
BUNDLE_DOCS="README.md"

if test -t 0; then
    echo
    echo "The Lucet toolchain is going to be installed in [${LUCET_PREFIX}]."
    echo "The installation prefix can be changed by defining a LUCET_PREFIX environment variable."
    echo "Hit Ctrl-C right now to abort before the installation begins."
    echo
    sleep 10
fi

install -d -v "$LUCET_BIN_DIR" || exit 1
for bin in $BINS; do
    install -p -v "${LUCET_SRC_RELEASE_DIR}/${bin}" "${LUCET_BIN_DIR}/${bin}"
done

install -d -v "$LUCET_LIB_DIR" || exit 1
for lib in $LIBS; do
    install -p -v "${LUCET_SRC_RELEASE_DIR}/${lib}" "${LUCET_LIB_DIR}/${lib}"
done

install -d -v "$LUCET_LIBEXEC_DIR" || exit 1
install -p -v "${LUCET_SRC_PREFIX}/lucet-builtins/build/libbuiltins.so" \
    "${LUCET_LIBEXEC_DIR}/libbuiltins.${DYLIB_SUFFIX}"

devenv_setenv_file="$(mktemp)"
cat >"$devenv_setenv_file" <<EOT
#! /bin/sh

export PATH="${LUCET_BIN_DIR}:\${PATH}"
export LD_LIBRARY_PATH="${LUCET_LIB_DIR}:\${LD_LIBRARY_PATH}"
export DYLD_LIBRARY_PATH="${LUCET_LIB_DIR}:\${DYLD_LIBRARY_PATH}"

if [ \$# -gt 0 ]; then
    exec "\$@"
fi
EOT

install -p -v "$devenv_setenv_file" "${LUCET_BIN_DIR}/devenv_setenv.sh"
rm -f "$devenv_setenv_file"

install -d -v "${LUCET_EXAMPLES_DIR}/sightglass" || exit 1
install -p -v -m 0644 "${LUCET_SRC_PREFIX}/sightglass/sightglass.toml" "${LUCET_EXAMPLES_DIR}/sightglass/sightglass.toml"

install -d -v "${LUCET_SHARE_DIR}/lucet-wasi" || exit 1
install -p -v -m 0644 "${LUCET_SRC_PREFIX}/lucet-wasi/bindings.json" "${LUCET_SHARE_DIR}/lucet-wasi/bindings.json"

for doc in $DOCS; do
    install -d -v "${LUCET_DOC_DIR}/$(dirname $doc)" || exit 1
    install -p -v -m 0644 "$doc" "${LUCET_DOC_DIR}/${doc}"
done

for doc in $BUNDLE_DOCS; do
    install -d -v "${LUCET_BUNDLE_DOC_DIR}/$(dirname $doc)" || exit 1
    install -p -v -m 0644 "$doc" "${LUCET_BUNDLE_DOC_DIR}/${doc}"
done

for file in clang clang++; do
    wrapper_file="$(mktemp)"
    cat >"$wrapper_file" <<EOT
#! /bin/sh

exec "${WASI_BIN}/${file}" --target="$WASI_TARGET" --sysroot="$WASI_SYSROOT" "\$@"
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

ln -svf "${LUCET_BIN_DIR}/${WASI_BIN_PREFIX}-clang" "${LUCET_BIN_DIR}/${WASI_BIN_PREFIX}-gcc"
ln -svf "${LUCET_BIN_DIR}/${WASI_BIN_PREFIX}-clang++" "${LUCET_BIN_DIR}/${WASI_BIN_PREFIX}-g++"

wrapper_file="$(mktemp)"
cat >"$wrapper_file" <<EOT
#! /bin/sh

exec "${LUCET_BIN_DIR}/lucetc" "\$@" --bindings "${LUCET_SHARE_DIR}/lucet-wasi/bindings.json"
EOT
install -p -v "$wrapper_file" "${LUCET_BIN_DIR}/lucetc-wasi"
rm -f "$wrapper_file"

(
    cd "$LUCET_SRC_PREFIX" || exit 1
    find assemblyscript -type d -exec install -d -v "${LUCET_SHARE_DIR}/{}" \;
    find assemblyscript -type f -exec install -p -v -m 0644 "{}" "${LUCET_SHARE_DIR}/{}" \;
)

if test -t 0; then
    echo
    echo "Lucet has been installed in [${LUCET_PREFIX}]"
    if [ "$(basename $SHELL)" == "fish" ]; then
        echo "Add ${LUCET_BIN_DIR} to your shell's search paths."
    else
        echo "Type 'source ${LUCET_BIN_DIR}/devenv_setenv.sh' to add the Lucet paths to your environment."
    fi
    echo "That command can also be added to your shell configuration."
    echo
fi
