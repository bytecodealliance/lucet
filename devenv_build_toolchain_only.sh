#!/bin/sh

. "$(dirname ${BASH_SOURCE:-$0})/config.inc"

git submodule update --init 2>/dev/null || :

if ! docker image inspect lucet:latest >/dev/null; then
        echo "A lucet image is not present"
        exit 1
fi

echo "Building lucet-toolchain:latest"
docker build -t lucet-toolchain:latest -f Dockerfile.toolchain .

echo "Starting the lucet container"
docker run --name=lucet --detach --mount type=bind,src="$(cd $(dirname ${0}); pwd),target=/lucet" \
        lucet:latest /bin/sleep 99999999 > /dev/null

echo "Creating a container from the lucet-toolchain:latest image"
docker run --name=lucet-toolchain --detach --mount type=bind,src="$(
        cd $(dirname ${0}) || exit 1
        pwd -P
),target=/lucet" \
        lucet-toolchain:latest /bin/sleep 99999999 >/dev/null

docker exec lucet tar c -pf - -C /opt lucet |
        docker exec -i lucet-toolchain tar x -pf - -C /opt

docker exec lucet-toolchain mkdir /opt/wasi-sysroot

docker exec lucet tar c -pf - -C /opt/wasi-sdk/share/wasi-sysroot . |
        docker exec -i lucet-toolchain tar x -pf - -C /opt/wasi-sysroot

docker exec lucet-toolchain sh -c 'rm -f /opt/lucet/bin/wasm32-*'

docker exec -i lucet-toolchain sh -c 'cat > /opt/lucet/bin/wasm32-wasi-clang; chmod 755 /opt/lucet/bin/wasm32-wasi-clang' <<EOT
#! /bin/sh
exec "clang" -fno-trapping-math -mthread-model single -W,--no-threads --target="wasm32-wasi" --sysroot="/opt/wasi-sysroot" "\$@"
EOT

docker exec -i lucet-toolchain sh -c 'cat > /opt/lucet/bin/wasm32-wasi-clang++; chmod 755 /opt/lucet/bin/wasm32-wasi-clang++' <<EOT
#! /bin/sh
exec "clang++" -fno-trapping-math -mthread-model single -W,--no-threads --target="wasm32-wasi" --sysroot="/opt/wasi-sysroot" "\$@"
EOT

docker container commit lucet-toolchain lucet-toolchain:latest &&
        docker tag lucet-toolchain:latest fastly/lucet-toolchain:latest

echo "Cleaning"
docker kill lucet
docker rm lucet
docker kill lucet-toolchain
docker rm lucet-toolchain

echo "Done"
