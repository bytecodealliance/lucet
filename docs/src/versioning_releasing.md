# Versioning and releasing to crates.io

This document describes how to appropriately decide the version of a new release, and the steps to
actually get it published on crates.io.

## Versioning

We release new versions of the Lucet crates all at once, keeping the versions in sync across
crates. As a result, our adherence to [semver](https://semver.org/) is project-wide, rather than
per-crate.

The versioning reflects the semantics of the *public* interface to Lucet. That is, any breaking
change to the following crates requires a semver major version bump:

- `lucetc`
- `lucet-objdump`
- `lucet-runtime`
- `lucet-validate`
- `lucet-wasi`
- `lucet-wasi-sdk`

For the other Lucet crates that are primarily meant for internal consumption, a breaking change does
*not* inherently require a semver major version bump unless either:

1. The changed interfaces are reexported as part of the public interface via the above crates, or
2. The binary format of a compiled Lucet module is changed.

For example, a change to the type of [`Instance::run()`][public-method] would require a major
version bump, but a change to the type of [`InstanceInternal::alloc()`][internal-method] would not.

[public-method]: https://docs.rs/lucet-runtime-internals/latest/lucet_runtime_internals/instance/struct.Instance.html#method.run
[internal-method]: https://docs.rs/lucet-runtime-internals/latest/lucet_runtime_internals/instance/trait.InstanceInternal.html#tymethod.alloc

Likewise, a change to a field on [`ModuleData`][module-data] would require a major version bump, as
it would change the serialized representation in a compiled Lucet module.

[module-data]: https://docs.rs/lucet-module/latest/lucet_module/struct.ModuleData.html

## The release process

The release process for a normal (non-hotfix) release consists of several phases:

1. [Preparing the release commit](#preparing-the-release-commit)

1. [Releasing to crates.io](#releasing-to-cratesio)

1. [Tagging and annotating the release in Git](#tagging-and-annotating-the-release-in-git)

1. [Merging the release commit](#merging-the-release-commit)

### Preparing the release commit

**Note** This is a new practice since we've introduced the practice of `-dev` versions and the
changelog, and is expected to be refined as we get more experience with it.

1. Determine the version for the new release (see [Versioning](#versioning)).

1. Create a new release branch based on the commit you want to eventually release. For example:

   ```shell
   $ git checkout -b 0.5.2-release origin/main
   ```

1. Replace the development version with the final version in the crates' `Cargo.toml` files. For
   example, `0.5.2-dev` should become `0.5.2`. Run the test suite in order to make sure `Cargo.lock`
   is up to date.

1. Edit `CHANGELOG.md` to add a new header with the version number and date of release.

1. Commit, then open a pull request for the release and mark it with the **DO NOT MERGE** label.

1. Secure review and approval from the Lucet team for the pull request.

At this point, you should have a commit on your release branch that you are prepared to release to
crates.io. Do not merge the pull request yet! Instead, proceed to [release](#releasing-to-cratesio)
the crates.

### Releasing to crates.io

Releasing a workspace full of interdependent crates can be challenging. Crates must be published in
the correct order, and any cyclic dependencies that might be introduced via `[dev-dependencies]`
must be broken. While there is [interest in making this smoother][publish-dev-deps], for now we have
to muddle through more manually.

[publish-dev-deps]: https://github.com/rust-lang/cargo/issues/4242

1. Authenticate with `cargo login` using a Github account with the appropriate access to the Lucet
   repository. You should only have to do this once per development environment.

1. Ensure that you have the commit checked out that you would like to release.

1. Ensure that the version in all of the Lucet `Cargo.toml` files matches the version you expect to
   release. Between releases, the versions will end in `-dev`; if this is still the case, you'll
   need to replace this version with the appropriate version according to the guidelines above,
   likely through a PR.

1. Edit `lucet-validate/Cargo.toml` and make the following change (note the leading `#`):

   ```diff
    [dev-dependencies]
   -lucet-wasi-sdk = { path = "../lucet-wasi-sdk", version = "=0.5.2" }
   +#lucet-wasi-sdk = { path = "../lucet-wasi-sdk", version = "=0.5.2" }
    tempfile = "3.0"
   ```

   This breaks the only cycle that exists among the crates as of `0.5.1`; if other cycles develop,
   you'll need to similarly break them by temporarily removing the dev dependency.

1. Begin publishing the crates in a topological order by `cd`ing to the each crate and running
   `cargo publish --allow-dirty` (the tree should only be dirty due to the cycles broken
   above). While we would like to run `cargo publish --dry-run` beforehand to ensure all
   of the crates will be successfully published, this will fail for any crates that depend on other
   Lucet crates, as the new versions will not yet be available to download.

   Do not worry too much about calculating the order ahead of time; if you get it wrong, `cargo
   publish` will tell you which crates need to be published before the one you tried. An order which
   worked for the `0.5.1` release was:

   1. `lucet-module`
   1. `lucet-validate`
   1. `lucetc`
   1. `lucet-wasi-sdk`
   1. `lucet-objdump`
   1. `lucet-runtime-macros`
   1. `lucet-runtime-internals`
   1. `lucet-runtime-tests`
   1. `lucet-runtime`
   1. `lucet-wasi`

   It is unlikely but not impossible that a publish will fail in the middle of this process, leaving
   some of the crates published but not others. What to do next will depend on the situation; please
   consult with the Lucet team.

1. Ensure the new crates have been published by checking for matching version tags on the [Lucet
   crates](https://crates.io/search?q=lucet).

Congratulations, the new crates are now on crates.io! 🎉

### Tagging and annotating the release in Git

1. Undo any changes in your local tree to break cycles.

1. Tag the release; `--sign` is optional but recommended if you have code signing configured:

   ```shell
   $ git tag --annotate --sign -m '0.5.2 crates.io release' 0.5.2
   $ git push --tags
   ```

1. Browse to this version's tag on the Github [tags page][tags-page], click **Edit tag**, and then
   paste this release's section of `CHANGELOG.md` into the description. Enter a title like `0.5.2
   crates.io release`, and then click **Publish release**.

[tags-page]: https://github.com/bytecodealliance/lucet/tags

### Merging the release commit

1. Edit the versions in the repo once more, this time to the next patch development version. For
   example, if we just released `0.5.2`, change the version to `0.5.3-dev`.

1. Commit, remove the **DO NOT MERGE** tag from your release PR, and seek final approval from the
   Lucet team.

1. Merge the release PR, and make sure the release branch is deleted. The release *tag* will not be
   deleted, and will be the basis for any future hotfix releases that may be required.
