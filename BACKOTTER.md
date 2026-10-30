# rustic for BackOtter

This branch (`backotter`) is rustic built on BackOtter's rustic_core: the
`backotter` branch of github.com/mchobbit/rustic_core, which carries the
patches BackOtter's rescue stick needs (see BACKOTTER.md there). It is not a
separate project: the branch is rebased onto every new rustic release rather
than merged.

## What it adds

- `[patch.crates-io]` in Cargo.toml, which swaps crates.io's rustic_core for
  BackOtter's. Cargo.lock pins the exact commit.
- The version reads `X.Y.Z+backotter.N`, so `rustic --version` and every
  snapshot's program version say which build made it.

## Building

    ./build-backotter.sh [TARGET ...]     # default: this machine's target

Builds with rustic's default features, which leave out `self-update`, so
there is no command to replace this build with stock rustic. Binaries go to
`dist/`, named `rustic-backotter-VERSION-TARGET`.

Unlike Go, Rust needs the target's C library and linker to build for another
system. Build the FreeBSD, OpenBSD and illumos binaries on those systems
(OpenBSD has no prebuilt Rust standard library for cross builds), or for
FreeBSD and illumos with a cross toolchain such as `cross`.

## Moving to a new rustic release

First move rustic_core's `backotter` branch to the rustic_core release the
new rustic uses, then:

    git fetch upstream --tags
    git rebase --onto vNEW vOLD backotter
    # set Cargo.toml's version to "NEW+backotter.1"
    cargo update -p rustic_core
    git push --force-with-lease origin backotter
