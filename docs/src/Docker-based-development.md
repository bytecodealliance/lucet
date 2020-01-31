# Getting started

The easiest way to get started with the Lucet toolchain is by using the provided
Docker-based development environment.

This repository includes a `Dockerfile` to build a complete environment for
compiling and running WebAssembly code with Lucet, but you shouldn't have to use
Docker commands directly. A set of shell scripts with the `devenv_` prefix are
used to manage the container.

## Setting up the environment

1) The Lucet repository uses Git submodules. Make sure they are checked out by running `git
   submodule update --init --recursive`.

2) Install and run the `docker` service. We do not support `podman` at this time. On macOS, [Docker
   for Mac](https://docs.docker.com/docker-for-mac/install/) is an option.

3) Once Docker is running, in a terminal at the root of the cloned repository, run: `source
   devenv_setenv.sh`. (This command requires the current shell to be `zsh`, `ksh` or `bash`). After
   a couple minutes, the Docker image is built and a new container is run.

4) Check that new commands are now available:

```sh
lucetc --help
```

You're now all set! You can now compile and run a ["Hello World" using
Lucet](./Your-first-Lucet-application.md).

## Top-level scripts for the Docker environment

* `./devenv_build_container.sh` rebuilds the container image. This is never
  required unless you edit the `Dockerfile`.
* `./devenv_run.sh [<command>] [<arg>...]` runs a command in the container. If
  a command is not provided, an interactive shell is spawned. In this
  container, Lucet tools are installed in `/opt/lucet` by default. The command
  `source /opt/lucet/bin/devenv_setenv.sh` can be used to initialize the
  environment.
* `./devenv_start.sh` and `./devenv_stop.sh` start and stop the container.
