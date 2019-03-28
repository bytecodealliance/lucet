#! /bin/sh

HOST_BASE_PREFIX=${HOST_BASE_PREFIX:-$(cd "$(dirname ${BASH_SOURCE:-$0})" && pwd)}
if [ ! -x "${HOST_BASE_PREFIX}/helpers/install.sh" ]; then
    echo "Unable to find the current script base directory" >&2
    exit 1
fi
HOST_PREFIX=${HOST_PREFIX:-"${HOST_BASE_PREFIX}/host"}
HOST_RUN=${HOST_RUN:-"${HOST_BASE_PREFIX}/devenv_run.sh"}
LUCET_PREFIX=${LUCET_PREFIX:-"/opt/lucet"}
LUCET_BIN_DIR=${LUCET_BIN_DIR-"${LUCET_PREFIX}/bin"}

HOST_BIN_DIR=${HOST_BIN_DIR:-${HOST_PREFIX}/bin}
HOST_LUCET_MOUNT_POINT=${HOST_LUCET_MOUNT_POINT:-/lucet}
