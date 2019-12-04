#! /bin/sh

VERSION=$(grep '#*Lucet version ' Cargo.toml | sed 's/^ *# Lucet version *//')
[ -z "$VERSION" ] && echo "Version header not found in the top Cargo.toml file" >&2 && exit 1

dry_run() {
    echo "Checking if the package can be published (dry run)..."
    echo
    find lucetc lucet-* benchmarks/lucet-* lucet-runtime/lucet-* -type f -maxdepth 1 -name 'Cargo.toml' -print | while read -r file; do
        dir="$(dirname $file)"
        echo "* Checking [$dir]"
        (cd "$dir" && cargo publish --allow-dirty --dry-run >/dev/null) || exit 1
    done || exit 1
    echo
    echo "Done."
}

version_bump() {
    echo
    echo "Setting the global Lucet version number to: [$VERSION]"
    find lucetc lucet-* benchmarks/lucet-* -type f -maxdepth 1 -name 'Cargo.toml' -print | while read -r file; do
        sed -i'.previous' "s/^ *version *=.*/version = \"${VERSION}\"/" "$file" && rm -f "${file}.previous"
    done
    echo "Done."
}

dry_run && version_bump && dry_run
