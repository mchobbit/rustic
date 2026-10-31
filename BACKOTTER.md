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
- Without a time zone (no `/etc/localtime`, as FreeBSD and DragonFly
  install, and `TZ` unset) times are in UTC with no warning (the debug log
  says so). jiff warned each time it looked for the time zone, for every
  time in every node: hundreds of warnings in a backup, and once more
  where there is no `/usr/share/zoneinfo` (AlpineBSD). Its warnings are
  all about time zones, and now go to the log only with
  `--log-level-dependencies` above `warn`. A `TZ` or `/etc/localtime` that
  jiff can't read gets one warning, and UTC.

Backups and restores work as BackOtter's restic does them, by default (see
BACKOTTER.md in rustic_core):

- `backup` keeps each file's access time; stock rustic stores the mtime
  there, as `--set-atime mtime` still does. `config/full.toml` shows the new
  default. On Linux the backup reads files and directories without changing
  their access times (`O_NOATIME`, where it may), except symlinks':
  `readlink(2)` moves them, with no way around it, though the snapshot
  keeps the earlier ones. `rewrite` keeps the saved access times too.
- `restore` gives files the uid and gid they were saved with; stock rustic
  looks owners up by name, in the users of the system it runs on, as
  `--by-name` (or `--ownership-by-name`, restic's name for it) still does.
  `--numeric-id` is accepted, and changes nothing.
- `restore` makes files that had holes sparse again, where it restores them
  into new files, and allocates every other file in full, zeros and all,
  so that swap files and preallocated files come back allocated; files
  restored over existing ones are never made sparse. `--sparse by-content`
  makes every new file sparse, as in stock rustic.
- `restore` over an existing tree replaces what is in the way of another
  type, with or without `--delete`: a symlink where the snapshot has a
  directory becomes the directory again; stock rustic restored the
  directory's files through the symlink, wherever it pointed.

## Building

    ./build-backotter.sh [TARGET ...]     # default: this machine's target

It refuses to build unless Cargo.lock pins rustic_core at the version it
names (`bko_core`, now 0.13.0+backotter.2) and rustic_core's walkdir (see
BACKOTTER.md there). Until rustic_core's `backotter` branch with that
version is pushed, Cargo.lock pins its commit fcac5fa (0.13.0+backotter.1),
so the build stops there. Once it is pushed:

1. Add `walkdir = { git = "https://github.com/mchobbit/rustic_core.git",
   branch = "backotter" }` to `[patch.crates-io]` in Cargo.toml. It can't
   go in before the push: cargo would look for walkdir in the pinned
   commit, which hasn't got it.
2. Run `cargo update -p rustic_core -p walkdir`.
3. Commit Cargo.lock.
4. When rustic_core's version changes, change `bko_core` in
   build-backotter.sh to match.

To build against a local copy of rustic_core instead (for tests), point
both entries of `[patch.crates-io]` at its paths (`crates/core` and
`vendor/walkdir`) and run `cargo update -p rustic_core -p walkdir`, in a
copy of this repository.

Builds with rustic's default features, which leave out `self-update`, so
there is no command to replace this build with stock rustic. Binaries go to
`dist/`, named `rustic-backotter-VERSION-TARGET`.

Unlike Go, Rust needs the target's C library and linker to build for another
system. Build the FreeBSD, OpenBSD, NetBSD and illumos binaries on those
systems (OpenBSD has no prebuilt Rust standard library for cross builds;
NetBSD's pkgsrc has Rust), or for FreeBSD and illumos with a cross toolchain
such as `cross`. AlpineBSD runs the NetBSD binary.

DragonFly's own Rust (dports) is too old for rustic, and
x86_64-unknown-dragonfly is a tier 3 target without a prebuilt standard
library: build it with nightly and `-Zbuild-std`, against a sysroot and
linker for DragonFly. Cargo.lock has rustix 1.1.6 rather than 1.1.4 for it:
1.1.4 wants source-specific multicast socket options for DragonFly that
libc doesn't have there, and doesn't build.

## Moving to a new rustic release

First move rustic_core's `backotter` branch to the rustic_core release the
new rustic uses, then:

    git fetch upstream --tags
    git rebase --onto vNEW vOLD backotter
    # set Cargo.toml's version to "NEW+backotter.1"
    cargo update -p rustic_core
    git push --force-with-lease origin backotter
