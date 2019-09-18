#!/bin/bash
set -e
ARG=$1
cleanup() {
	if [[ $ARG == "check" ]]; then
		echo ""
		echo "Formatting diffs detected! run \"./indent\" to correct."
	fi
	rm -f .formatted
}
trap cleanup 1 2 3 6 15

if ! rustfmt --version | grep -q "rustfmt 1.2.2-stable"; then
	echo "indent requires rustfmt 1.2.2-stable"
	exit 1
fi

RUST_DIRS=$(find lucetc lucet-* benchmarks/lucet-* -type f -name 'Cargo.toml' -print)

if [[ $ARG == "check" ]]; then
	for RUST_DIR in ${RUST_DIRS}; do
		pushd "$(dirname ${RUST_DIR})" >/dev/null
		cargo fmt -- --check
		popd >/dev/null
	done
elif [[ $ARG == "" ]]; then
	for RUST_DIR in ${RUST_DIRS}; do
		pushd "$(dirname ${RUST_DIR})" >/dev/null
		cargo fmt
		popd >/dev/null
	done
else
	echo "unsupported argument: $1"
	exit 1
fi
