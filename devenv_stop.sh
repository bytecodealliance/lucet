#!/bin/sh

. "$(dirname ${BASH_SOURCE:-$0})/config.inc"

echo "Stopping container"
docker stop lucet

echo "Removing container"
docker rm lucet
