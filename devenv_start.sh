#!/bin/sh

. "$(dirname ${0})/config.inc"

if ! docker image inspect lucet-dev:latest > /dev/null; then
	${HOST_BASE_PREFIX}/devenv_build_container.sh
	sleep 30
fi

if docker ps -f name=lucet | grep -Fq lucet ; then
	echo "container is already running" >&2
	exit 1
fi

docker run --name=lucet --detach --mount type=bind,src="$(pwd),target=/lucet" \
	lucet-dev:latest /bin/sleep 99999999 > /dev/null
