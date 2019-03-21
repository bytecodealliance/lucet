#!/bin/sh

. "$(dirname ${0})/config.inc"

if ! docker ps -f name=lucet | grep -Fq lucet ; then
	${HOST_BASE_PREFIX}/devenv_start.sh
fi

if ! docker exec -t -w /lucet lucet stat "$LUCET_PREFIX" > /dev/null ; then
	echo "Lucet hasn't been installed yet... installing..."
	docker exec -t -w /lucet lucet make install
fi

if [ $# -eq 0 ]; then
	docker exec -it -w /lucet lucet /bin/bash
else
	docker exec -it -w /lucet lucet $@
fi
