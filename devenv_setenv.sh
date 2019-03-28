#!/bin/sh

. $(dirname ${BASH_SOURCE:-$0})/config.inc

if ! "$HOST_RUN" true ; then
    echo "Unable to run commands in the container" >&2
    return 1 2> /dev/null || exit 1
fi

install -d "$HOST_BIN_DIR"

"$HOST_RUN" find "$LUCET_BIN_DIR" \( -type f -o -type l \) -perm -0500 -print | \
    awk '{ sub("\r$", ""); print }' | \
while read -r wrapper_path ; do
    original_file=$(basename "$wrapper_path")
    wrapper_file="$(mktemp)"
    env cat >> "$wrapper_file" << EOT
#! /bin/sh

exec "$HOST_RUN" ${LUCET_BIN_DIR}/devenv_setenv.sh $wrapper_path "\$@"
EOT
    install -p "$wrapper_file" "${HOST_BIN_DIR}/${original_file}"
    rm -f "$wrapper_file"
done

export PATH="${HOST_BIN_DIR}:$PATH"
rehash 2> /dev/null ||:
