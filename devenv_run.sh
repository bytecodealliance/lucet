#!/bin/sh

. "$(dirname ${BASH_SOURCE:-$0})/config.inc"

if ! docker ps -f name='lucet' --format '{{.Names}}' | grep -q '^lucet$' ; then
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

opts="-i"
[ -t 0 ] && opts="-t $opts"

if [ $# -eq 0 ]; then
	docker exec $opts -w "$lucet_workdir" lucet /bin/bash
else
	docker exec $opts -w "$lucet_workdir" lucet "$@"
fi
