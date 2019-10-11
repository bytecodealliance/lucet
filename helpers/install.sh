#! /bin/sh

LUCET_SRC_PREFIX=${LUCET_SRC_PREFIX:-"$(
    cd $(dirname $(dirname ${0}))
    pwd -P
)"}
if [ ! -x "${LUCET_SRC_PREFIX}/helpers/install.sh" ]; then
    echo "Unable to find the current script base directory" >&2
    exit 1
fi

if [ "$1" = "--unoptimized" ]; then
    LUCET_BUILD_TYPE="debug"
else
    LUCET_BUILD_TYPE="release"
fi

LUCET_PREFIX=${LUCET_PREFIX:-"/opt/lucet"}
LUCET_SRC_PREFIX=${LUCET_SRC_PREFIX:-"/lucet"}
LUCET_SRC_RELEASE_DIR=${LUCET_SRC_RELEASE_DIR:-"${LUCET_SRC_PREFIX}/target/${LUCET_BUILD_TYPE}"}
LUCET_BIN_DIR=${LUCET_BIN_DIR:-"${LUCET_PREFIX}/bin"}
LUCET_LIB_DIR=${LUCET_LIB_DIR:-"${LUCET_PREFIX}/lib"}
LUCET_LIBEXEC_DIR=${LUCET_LIBEXEC_DIR:-"${LUCET_PREFIX}/libexec"}
LUCET_SYSCONF_DIR=${LUCET_SYSCONF_DIR:-"${LUCET_PREFIX}/etc"}
LUCET_SHARE_DIR=${LUCET_SHARE_DIR:-"${LUCET_PREFIX}/share"}
LUCET_EXAMPLES_DIR=${LUCET_EXAMPLES_DIR:-"${LUCET_SHARE_DIR}/examples"}
LUCET_DOC_DIR=${LUCET_DOC_DIR:-"${LUCET_SHARE_DIR}/doc"}
LUCET_BUNDLE_DOC_DIR=${LUCET_BUNDLE_DOC_DIR:-"${LUCET_DOC_DIR}/lucet"}
WASI_SDK_PREFIX=${WASI_SDK_PREFIX:-${WASI_SDK:-"/opt/wasi-sdk"}}
WASI_TARGET=${WASI_TARGET:-"wasm32-wasi"}
WASI_BIN_PREFIX=${WASI_BIN_PREFIX:-"$WASI_TARGET"}
BINARYEN_DIR=${BINARYEN_DIR:-"/opt/binaryen"}
BINARYEN_BIN_DIR=${BINARYEN_BIN_DIR:-"${BINARYEN_DIR}/bin"}

if [ "$(uname -s)" = "Darwin" ]; then
    DYLIB_SUFFIX="dylib"
else
    DYLIB_SUFFIX="so"
fi

BINS="lucet-objdump lucet-validate lucet-wasi lucetc sightglass spec-test wasmonkey"
LIBS="liblucet_runtime.${DYLIB_SUFFIX}"
DOCS="sightglass/README.md"
BUNDLE_DOCS="README.md"

if test -t 0; then
    echo
    echo "The Lucet toolchain is going to be installed in [${LUCET_PREFIX}]."
    echo "The installation prefix can be changed by defining a LUCET_PREFIX environment variable."
    echo "Hit Ctrl-C right now to abort before the installation begins."
    echo
    sleep 10
fi

if ! install -d "$LUCET_PREFIX" 2>/dev/null; then
    SUDO=""
    if command -v doas >/dev/null; then
        SUDO="doas"
    elif command -v sudo >/dev/null; then
        SUDO="sudo"
    else
        echo "[${LUCET_PREFIX}] doesn't exist and cannot be created" >&2
        exit 1
    fi
    echo "[${LUCET_PREFIX}] doesn't exist and the $SUDO command is required to create it"
    if ! "$SUDO" install -o "$(id -u)" -d "$LUCET_PREFIX"; then
        echo "[${LUCET_PREFIX}] doesn't exist and cannot be created even with additional privileges" >&2
        exit 1
    fi
fi

# Find a WASI sysroot
for wasi_sysroot in $WASI_SYSROOT ${WASI_SDK_PREFIX}/share/wasi-sysroot /opt/wasi-sysroot; do
    if [ -e "${wasi_sysroot}/include/wasi/core.h" ]; then
        WASI_SYSROOT="$wasi_sysroot"
    fi
done
if [ -z "$WASI_SYSROOT" ]; then
    echo "The WASI sysroot was not found." >&2
    echo "You may have to define a WASI_SYSROOT environment variable set to its base directory."
    exit 1
fi
echo "* WASI sysroot: [$WASI_SYSROOT]"

# Find:
# - A clang/llvm installation able to compile to WebAssmbly/WASI
# - The base path to this installation
# - The optional suffix added to clang (e.g. clang-8)
# - The optional suffix added to LLVM tools (e.g. ar-8) that differs from the clang one on some Linux distributions

TMP_OBJ=$(mktemp)
for llvm_bin_path_candidate in "$LLVM_BIN" "${WASI_SDK_PREFIX}/bin" /usr/local/opt/llvm/bin $(echo "$PATH" | sed s/:/\ /g); do
    [ -d "$llvm_bin_path_candidate" ] || continue
    clang_candidate=$(find "$llvm_bin_path_candidate" -maxdepth 1 \( -type f -o -type l \) \( -name "clang" -o -name "clang-[0-9]*" \) -print |
        sort | while read -r clang_candidate; do
            echo "int main(void){return 0;}" | "$clang_candidate" --target=wasm32-wasi -o "$TMP_OBJ" -c -x c - 2>/dev/null || continue
            echo "$clang_candidate"
            break
        done)
    [ -z "$clang_candidate" ] && continue
    llvm_bin=$(dirname "$clang_candidate")
    clang_candidate_bn=$(basename "$clang_candidate")
    case "$clang_candidate_bn" in
    clang) clang_bin_suffix="none" ;;
    clang-[0-9]*) clang_bin_suffix=$(echo "$clang_candidate_bn" | sed "s/^clang//") ;;
    *) continue ;;
    esac
    CLANG_BIN_SUFFIX="$clang_bin_suffix"
    LLVM_BIN="$llvm_bin"
    if [ -z "$CLANG_BIN_SUFFIX" ] || [ -z "$LLVM_BIN" ]; then
        continue
    fi
    if [ "$CLANG_BIN_SUFFIX" = "none" ]; then
        CLANG_BIN_SUFFIX=""
    fi
    break
done
rm -f "$TMP_OBJ"

if [ -z "$LLVM_BIN" ]; then
    echo "No clang/LLVM installation able to compile to WebAssembly/WASI was found." >&2
    echo "The builtins might be missing -- See the Lucet documentation." >&2
    exit 1
fi
echo "* LLVM installation directory: [$LLVM_BIN]"
echo "* Suitable clang executable: [clang${CLANG_BIN_SUFFIX}]"

LLVM_BIN_SUFFIX="$CLANG_BIN_SUFFIX"
if ! command -v "${LLVM_BIN}/llvm-ar${LLVM_BIN_SUFFIX}" >/dev/null; then
    LLVM_BIN_SUFFIX=""
    if ! command -v "${LLVM_BIN}/llvm-ar${LLVM_BIN_SUFFIX}" >/dev/null; then
        echo "LLVM not found" >&2
        exit 1
    fi
    echo test
fi
echo "* LLVM tools suffix: [${LLVM_BIN_SUFFIX}] (ex: [llvm-ar${LLVM_BIN_SUFFIX}])"
echo

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

exec "${LLVM_BIN}/${file}${CLANG_BIN_SUFFIX}" --target="$WASI_TARGET" --sysroot="$WASI_SYSROOT" "\$@"
EOT
    install -p -v "$wrapper_file" "${LUCET_BIN_DIR}/${WASI_BIN_PREFIX}-${file}"
    rm -f "$wrapper_file"
done

for file in ar dwarfdump nm ranlib size; do
    ln -sfv "${LLVM_BIN}/llvm-${file}${LLVM_BIN_SUFFIX}" "${LUCET_BIN_DIR}/${WASI_BIN_PREFIX}-${file}"
done

for file in ld; do
    ln -sfv "${LLVM_BIN}/wasm-${file}${LLVM_BIN_SUFFIX}" "${LUCET_BIN_DIR}/${WASI_BIN_PREFIX}-${file}"
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

for file in wasm-opt wasm-reduce; do
    ln -sfv "${BINARYEN_BIN_DIR}/${file}" "${LUCET_BIN_DIR}/${file}"
done

if test -t 0; then
    echo
    echo "Lucet has been installed in [${LUCET_PREFIX}]"
    if [ "$(basename $SHELL)" = "fish" ]; then
        echo "Add ${LUCET_BIN_DIR} to your shell's search paths."
    else
        echo "Type 'source ${LUCET_BIN_DIR}/devenv_setenv.sh' to add the Lucet paths to your environment."
    fi
    echo "That command can also be added to your shell configuration."
    echo
fi
