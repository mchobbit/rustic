# rustic for BackOtter

This branch (`backotter`) is rustic built on BackOtter's rustic_core: the
`backotter` branch of github.com/mchobbit/rustic_core, which carries the
patches BackOtter's rescue stick needs (see BACKOTTER.md there). It is not a
separate project: the branch is rebased onto every new rustic release rather
than merged.

## What it adds

- `[patch.crates-io]` in Cargo.toml, which swaps crates.io's rustic_core for
  BackOtter's. Cargo.lock pins the exact commit.
- Restoring `SNAP:subfolder` gives the destination directory the
  subfolder's own metadata, as everything below it gets its own; stock
  rustic leaves the destination as it was, so a restored file system's root
  keeps mkfs's mode and has no SELinux label.
- The version reads `X.Y.Z+backotter.N`, so `rustic --version` and every
  snapshot's program version say which build made it.

Backups and restores work as BackOtter's restic does them, by default (see
BACKOTTER.md in rustic_core):

- `backup` keeps each file's access time; stock rustic stores the mtime
  there, as `--set-atime mtime` still does. `config/full.toml` shows the new
  default.
- `restore` gives files the uid and gid they were saved with; stock rustic
  looks owners up by name, in the users of the system it runs on, as
  `--by-name` still does. `--numeric-id` is accepted, and changes nothing.
- `restore` makes files that had holes sparse again and writes out every
  byte of any other, so that preallocated files come back allocated;
  `--sparse by-content` makes every file sparse, as in stock rustic.

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
