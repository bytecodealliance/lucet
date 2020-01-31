# End-to-end integrity and authentication of Lucet assets

Lucet tools have the ability to verify digital signatures of their input, and produce signed output, ensuring continuous trust even if assets have to transit over unsecured networks.

* `lucetc` can be configured to only compile source code (`.wasm`, `.wat` files) if a signature is present, and can be verified using a pre-configured public key.
* Shared libraries produced by the `lucetc` compiler can themselves embed a signature, computed using the same secret key as the source code, or a different key.
* The `lucet-wasi` runtime can accept to run native code from `lucetc` only if it embeds a valid signature for a pre-configured public key.

Secret keys can be protected by a password for interactive use, or be password-less for automation.

[Minisign](https://jedisct1.github.io/minisign/) is the highly secure signature system used in Lucet via the [minisign crate](https://crates.io/crates/minisign). Key pairs and signatures are fully compatible with other implementations.

## Lucetc signature verification

Source files (`.wasm`, `.wat`) can be signed with [`minisign`](https://jedisct1.github.io/minisign/), [`rsign2`](https://github.com/jedisct1/rsign2), or another Minisign implementation.

The Lucet container ships with `rsign2` preinstalled.

### Creating a new key pair

```sh
rsign generate
```

```text
Please enter a password to protect the secret key.
Password:
Password (one more time):
Deriving a key from the password in order to encrypt the secret key... done

The secret key was saved as /Users/j/.rsign/rsign.key - Keep it secret!
The public key was saved as rsign.pub - That one can be public.

Files signed using this key pair can be verified with the following command:

rsign verify <file> -P RWRJwC2NawX3xnBK6mvAAehmFWQ6Z1PLXoyIz78LYkLsklDdaeHEcAU5
```

### Signing a WebAssembly input file

```sh
rsign sign example.wasm
```

```text
Password:
Deriving a key from the password and decrypting the secret key... done
```

The resulting signature is stored into a file with the same name as the file having been signed, with a `.minisig` suffix (in the example above: `example.wasm.minisig`).

### Configuring lucetc to verify signatures

Source files can be verified by adding the following command-line switches to `lucetc`.

```text
--signature-verify
--signature-pk=<path to the public key file>
```

`lucetc` assumes that a source file and its signature are in the same directory.

Compilation will only start if the signature is valid for the given public key.

## Producing signed shared objects

Shared libraries produced by the `lucetc` compiler can embed a signature.

### Creating a key pair

This requires a secret key, that can be either created using a 3rd party minisign implementation, or by `lucetc` itself:

```sh
lucetc --signature-keygen \
  --signature-sk <file to store the secret key into> \
  --signature-pk <file to store the public key into>
```

By default, secret keys are protected by a password. If this is inconvenient, `lucetc` also supports `raw`, unencrypted secret keys.
In order to use raw keys, add a `raw:` prefix before the file name (ex: `--signature-sk=raw:/opt/etc/lucet.key`).

### Signing shared objects produced by lucetc

In order to embed a signature in a shared object produced by `lucetc` or `lucetc-wasi`, the following command-line switches should be present:

```text
--signature-create
--signature-sk <path to the secret key file>
```

If the secret key was encrypted with a password, the password will be asked interactively.

Signatures are directly stored in the `.so` or `.dylib` files.

Key pairs used for source verification and for signing compiled objects can be different, and both operations are optional.

## Signature verification in the Lucet runtime

`lucet-wasi` can be configured to run only trusted native code, that includes a valid signature for a pre-configured key. In order to do so, the following command-line switches have to be present:

```text
--signature-verify
--signature-pk <path to the public key file>
```
