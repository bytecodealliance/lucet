# Using Lucet with AssemblyScript

This example shows that Lucet and the WASI interface can be used with programming languages that, unlike Rust and C, don't depend on a C library.

[AssemblyScript](https://github.com/AssemblyScript/assemblyscript) is a TypeScript variant that compiles to WebAssembly.

The base layer of WASI is a small set of external functions implemented by the runtime.

These functions can be accessed from any WebAssembly module, no matter what their original programming language is.

AssemblyScript is no exception to that, so we implemented WASA, a thin abstraction between AssemblyScript and these external WASI functions.

In the Lucet image, the WASA module is installed in the `/opt/lucet/share/assemblyscript/modules/wasa` directory. In includes the AssemblyScript code, as well as the bindings file for the Lucet compiler.

It currently implements only the subset of WASI functions supported by the Lucet runtime.

The code provided in this directory reads environment variables, loads and writes files, generates random numbers, parses the command-line arguments and writes to the console using WASI. All from AssemblyScript.

## Compilation

```sh
npm install

npm run asbuild:optimized

lucetc -o example \
  --reserved-size=64MB \
  --bindings /opt/lucet/share/assemblyscript/modules/wasa/bindings.json \
  build/optimized.wasm
```

## Running the example using lucet-wasi

```sh
lucet-wasi --entrypoint main --dir .:. example help
```

Unlike C and Rust applications, AssemblyScript requires an entry point to be explicitly defined.
It cannot be the default `_start` function.

In this example, this is the `main` function.

Since files are going to be read and written to, a descriptor to a pre-opened directory needs to be provided to runtime.

This is the purpose of the `--dir` command-line option. Without this, the webassembly module cannot access the filesystem at all.

Here, the current virtual directory (`.`), as seen by the application, maps to the current directory in the container.

## What the example does

This example application creates and verifies digital signatures for arbitrary files.

The main code is in `assembly/index.ts`.

Key pair creation:

```sh
lucet-wasi --entrypoint main --dir .:. example keypair
```

```text
Creating a new keypair...
Key pair created and saved into [keypair.bin]
Public key: [94b8eb14373eb245c1daaacb2c24e2cb554bdd723009423aae5a8ca5fa99fa16]
```

A new file `keypair.bin` is created on the local filesystem, at the root of the first mount point (the current directory).

An environment variable called `KEYPAIR_FILE`, can be used in order to change the file name and location.

The WASA wrapper automatically sets the minimum required WASI capabilities in order to create a new file, or read an existing one.

File signature:

```sh
lucet-wasi --entrypoint main --dir .:. example sign README.md
```

```text
Signature for that file: [deedf3910d5b166ca17e0e307312a422cb50efcbcc90754cf0e2d528a9159c4ad3ac973e3cd9b2c2986fb2e467a0506bc9a5ceb9c7d6d30e360fb4d1cef3c50d]
```

This command reads the `README.md` file, as well as the key pair, and computes a signature of the file's content that can be verified using the public key.

Signature verification:

```sh
lucet-wasi --entrypoint main --dir .:. example verify README.md <public key> <signature>
```

`<public key>` and `<signature>` must be replaced with output from the previous commands.

```text
This is a valid signature for that file
```

The `verify` command checks that a signature is valid for a given file and public key.
