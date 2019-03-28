#!/bin/sh

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

docker build -t lucet-dev:latest .
