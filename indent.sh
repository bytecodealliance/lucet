#!/bin/bash
set -e
ARG=$1
cleanup () {
	if [[ $ARG == "check" ]]; then
		echo ""
		echo "Formatting diffs detected! run \"./indent\" to correct."
	fi
	rm -f .formatted
}
trap cleanup 1 2 3 6 9 15

if test -z "${LUCET_CLANG_FORMAT}"; then
	CLANG_FORMAT="clang-format";
else
	CLANG_FORMAT="${LUCET_CLANG_FORMAT}";
fi

if ! $($CLANG_FORMAT --version | grep -q "version 7.0"); then
	echo "indent requires clang-format 7.0.0"
	exit 1;
fi

if ! $(rustfmt --version | grep -q "rustfmt 1.0.0-stable"); then
	echo "indent requires rustfmt 1.0.0-stable"
	exit 1;
fi

C_DIRS="lucet-runtime-c lucet-backtrace tests"
C_FILES=$(find ${C_DIRS} -type f \( -name '*.h' -or -name '*.c' \) -and -not \( -name 'greatest.h' -or -path '*/target/*' \) -print)
RUST_DIRS=$(find lucet-analyze lucet-idl lucet-rs lucet-spectest lucetc lucet-runtime lucet-wasi lucet-wasi-sdk -type f -name 'Cargo.toml' -print)

if [[ $ARG == "check" ]]; then
	for C_FILE in ${C_FILES}; do
		${CLANG_FORMAT} ${C_FILE} > .formatted
		diff -u ${C_FILE} .formatted
		rm -f .formatted
	done
	for RUST_DIR in ${RUST_DIRS}; do
		pushd $(dirname ${RUST_DIR}) > /dev/null
		cargo fmt --all -- --check
		popd > /dev/null
	done
elif [[ $ARG == "" ]]; then
	for C_FILE in ${C_FILES}; do
		${CLANG_FORMAT} -i ${C_FILE}
	done
	for RUST_DIR in ${RUST_DIRS}; do
		pushd $(dirname ${RUST_DIR}) > /dev/null
		cargo fmt --all
		popd > /dev/null
	done
else
	echo "unsupported argument: $1"
	exit 1
fi

