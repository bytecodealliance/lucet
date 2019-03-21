#!/bin/sh

. config.inc

if docker image inspect lucet-dev:latest > /dev/null; then
	echo "A lucet-dev image is already present"
	echo "Hit Ctrl-C right now if you don't want to rebuild it"
	sleep 30
fi

docker build -t lucet-dev:latest .
