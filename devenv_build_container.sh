#!/bin/sh
set -e

. "$(dirname ${BASH_SOURCE:-$0})/config.inc"

git submodule update --init 2>/dev/null ||:

if docker image inspect lucet-dev:latest > /dev/null; then
        if [ -z "$DEVENV_FORCE_REBUILD" ]; then
                echo "A lucet-dev image is already present"
                echo "Hit Ctrl-C right now if you don't want to rebuild it"
                echo "or skip this wait by setting the DEVENV_FORCE_REBUILD variable"
                sleep 30
        fi
fi

echo "Building lucet-dev:latest"
docker build -t lucet-dev:latest .

docker tag lucet-dev:latest lucet:latest

if [ ! -z "$DEVENV_SKIP_LUCET_BUILD" ]; then
    echo "Done"
    exit 0
fi

if docker image inspect lucet:latest > /dev/null; then
        if [ -z "$DEVENV_FORCE_REBUILD" ]; then
                echo "A lucet image is already present"
                echo "Hit Ctrl-C right now if you don't want to rebuild it"
                echo "or skip this wait by setting the DEVENV_FORCE_REBUILD variable"
                sleep 30
        fi
fi

echo "Now creating lucet:latest on top of lucet-dev:latest"
docker run --name=lucet-dev --detach --mount type=bind,src="$(cd $(dirname ${0}); pwd),target=/lucet" \
        lucet-dev:latest /bin/sleep 99999999 > /dev/null

echo "Building and installing optimized files in [$HOST_LUCET_MOUNT_POINT]"
if [ -z "$UNOPTIMIZED_BUILD" ]; then
        docker exec -t -w "$HOST_LUCET_MOUNT_POINT" lucet-dev make install
else
        docker exec -t -w "$HOST_LUCET_MOUNT_POINT" lucet-dev make install-dev
fi

echo "Cleaning"
docker exec -t -w "$HOST_LUCET_MOUNT_POINT" lucet-dev make clean

echo "Tagging the new image"
docker container commit lucet-dev lucet:latest

echo "Cleaning"
docker kill lucet-dev
docker rm lucet-dev

echo "Done"
