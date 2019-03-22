#!/bin/sh

. "$(dirname ${0})/config.inc"

if ! docker ps -f name=lucet | grep -Fq lucet ; then
	${HOST_BASE_PREFIX}/devenv_start.sh
fi

if [ $# -eq 0 ]; then
	docker exec -it -w "$HOST_LUCET_MOUNT_POINT" lucet /bin/bash
else
	docker exec -t -w "$HOST_LUCET_MOUNT_POINT" lucet $@
fi
