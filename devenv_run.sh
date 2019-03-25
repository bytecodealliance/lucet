#!/bin/sh

. "$(dirname ${0})/config.inc"

if ! docker ps -f name=lucet | grep -Fq lucet ; then
	${HOST_BASE_PREFIX}/devenv_start.sh
fi

lucet_workdir="$HOST_LUCET_MOUNT_POINT"
prefix="$(pwd)"
relpath=""
while [ -n "$prefix" -a "$prefix" != "/" -a "$prefix" != "$HOST_BASE_PREFIX" ]; do
	relpath="$(basename $prefix)/${relpath}"
	prefix=$(dirname "$prefix")
done
if [ "$prefix" = "$HOST_BASE_PREFIX" ]; then
	lucet_workdir="${HOST_LUCET_MOUNT_POINT}/${relpath}"
fi

if [ $# -eq 0 ]; then
	docker exec -it -w "$lucet_workdir" lucet /bin/bash
else
	docker exec -t -w "$lucet_workdir" lucet $@
fi
