#!/bin/sh

. "$(dirname ${0})/config.inc"

if ! docker image inspect lucet-dev:latest > /dev/null; then
	${HOST_BASE_PREFIX}/devenv_build_container.sh
fi

if docker ps -f name=lucet | grep -Fq lucet ; then
	echo "container is already running" >&2
	exit 1
fi

docker run --name=lucet --detach --mount type=bind,src="$(cd $(dirname ${0}); pwd -P),target=/lucet" \
	lucet-dev:latest /bin/sleep 99999999 > /dev/null

if [ -z "$DEVENV_NO_INSTALL" ]; then
	if ! docker exec -t -w "$HOST_LUCET_MOUNT_POINT" lucet stat "$LUCET_PREFIX" > /dev/null ; then
		echo "Lucet hasn't been installed yet... installing..."
		docker exec -t -w "$HOST_LUCET_MOUNT_POINT" lucet make install
	fi
fi
