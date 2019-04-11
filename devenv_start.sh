#!/bin/sh

. "$(dirname ${BASH_SOURCE:-$0})/config.inc"

if ! docker image inspect lucet:latest > /dev/null; then
	${HOST_BASE_PREFIX}/devenv_build_container.sh
fi

if docker ps -f name='lucet' --format '{{.Names}}' | grep -q '^lucet$' ; then
	echo "container is already running" >&2
	exit 1
fi

docker run --name=lucet --detach --mount type=bind,src="$(cd $(dirname ${0}); pwd -P),target=/lucet" \
	lucet:latest /bin/sleep 99999999 > /dev/null
