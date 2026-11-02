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
  `--log-level-dependencies` above `warn`. Nor is an `/etc/localtime` that
  links to nothing (Alpine without tzdata links it to
  `/usr/share/zoneinfo/UTC`) any news: UTC, debug log only, as restic says
  nothing. A `TZ`, or an `/etc/localtime` there is, that jiff can't read
  gets one warning, and UTC.

Backups and restores work as BackOtter's restic does them, by default (see
BACKOTTER.md in rustic_core):

- `backup` keeps each file's access time; stock rustic stores the mtime
  there, as `--set-atime mtime` still does. `config/full.toml` shows the new
  default. On Linux the backup reads files and directories without changing
  their access times (`O_NOATIME`, where it may), except symlinks':
  `readlink(2)` moves them, with no way around it, though the snapshot
  keeps the earlier ones. Elsewhere (FreeBSD and the other BSDs, illumos,
  Haiku), with no `O_NOATIME`, listing a directory moves its access time:
  the backup takes a directory's metadata before it lists it, and doesn't
  scan the source for its size first (as `--no-scan`: no total or ETA), so
  that the snapshot has the access times from before the backup, as in
  BackOtter's restic. `rewrite` keeps the saved access times too.
- `restore` gives files the uid and gid they were saved with; stock rustic
  looks owners up by name, in the users of the system it runs on, as
  `--by-name` (or `--ownership-by-name`, restic's name for it) still does.
  `--numeric-id` is accepted, and changes nothing. A restore not running
  as root says nothing about an owner it can't set (`EPERM`, `EACCES`), as
  restic doesn't; stock rustic warned about each such file.
- `backup` skips Unix sockets and `restore` leaves out the socket nodes
  of older snapshots, as restic does (see BACKOTTER.md in rustic_core).
- `backup` stores the directories above the backed-up path (`/home` in a
  backup of `/home/m`) with their own metadata, as restic does, so a
  restore of the whole snapshot gives them their own mode, owner and
  times; stock rustic stored them as `0755` with no owner or times. It
  leaves out a device node's device number where it is 0, as restic does
  (see BACKOTTER.md in rustic_core).
- `backup` leaves out a regular file's mode where it is 0 (`/etc/shadow`
  on Rocky), as restic does; skips illumos doors, as it skips sockets; and
  reads extended attributes over 4 KiB whole on Linux (the xattr crate
  read them as empty). `restore` never removes a file's SELinux label
  where its node has none (SELinux refuses), clears append-only on ZFS on
  Linux before restoring over a file (`FS_IOC_FSSETXATTR` doesn't there),
  and on illumos tmpfs sets no birth time without a warning, and warns
  about each NFSv4 ACL tmpfs or UFS refuses (`ENOSYS`) and exits 0, as
  BackOtter's restic now does too (see BACKOTTER.md in rustic_core).
- `backup` keeps the attributes of files on FAT (hidden, system, archive
  cleared, a directory's read-only: `linux.fat_attrs`) and the options of
  files on XFS (extent size hints, nodefrag, filestream, ...:
  `linux.xfs_options`), on Linux, as restic does, and `restore` puts them
  back there, last, before the file flags (an extent size hint also as
  soon as it creates the file, while it is empty); onto another file system
  they are one warning per restore. A dense restore leaves XFS's
  preallocated flag (`p`) on the files it preallocates, as restic's does
  (see BACKOTTER.md in rustic_core).
- `backup` reads files with holes by their data extents (`SEEK_DATA`,
  `SEEK_HOLE`, on Linux, FreeBSD, macOS and illumos) and gives the chunker
  zeros for the holes without reading them: the same bytes as plain
  reading, so the same chunks and snapshots.
- `restore` makes files that had holes sparse again, where it restores them
  into new files: of each file system block a blob reaches into, the part
  inside the blob is written only if it isn't all zeros, so a block is
  allocated only where the file has data, as in BackOtter's restic. It
  allocates every other file in full, zeros and all, so that swap files
  and preallocated files come back allocated; files restored over
  existing ones are never made sparse. `--sparse by-content` makes every
  new file sparse, as in stock rustic. On Linux's JFS and nilfs2, whose
  `SEEK_HOLE` finds no hole before the end, the blocks decide which files
  have holes (fewer blocks than the size), as in BackOtter's restic; such a
  file is backed up by plain reading, holes and all, and comes back sparse.
  The blocks are the destination file system's own down to 512 bytes
  (4096 where its block size isn't known), as in BackOtter's restic: a
  file with 512-byte holes on bcachefs formatted with `block_size` 512
  comes back with them; until round 23 they were at least 4096 bytes, and
  such a file came back dense (see BACKOTTER.md in rustic_core).
- `restore` leaves an extended attribute a file has already with its
  node's value alone, as before, and BackOtter's restic now does too:
  bcachefs refuses `bcachefs.casefold` on a directory with entries, even
  the value it has, which the directory got when the restore created it.
  Where it still has to change on a directory with entries (restored
  over), bcachefs's refusal is a warning, "casefolding (bcachefs.casefold)
  not restored on DIR: directory not empty", as for chattr's F, in both
  tools.
- `restore` over an existing tree replaces what is in the way of another
  type, with or without `--delete`: a symlink where the snapshot has a
  directory becomes the directory again; stock rustic restored the
  directory's files through the symlink, wherever it pointed.
- `restore` over an existing tree first clears the flags that would refuse
  it on what it finds there (immutable, append-only, undeletable: FreeBSD's
  `schg`, `sappnd`, `sunlnk` and the rest, Linux's `i` and `a`), and gives
  every file its snapshot's flags last, so a restore over an installed
  FreeBSD puts its `schg` libc back as it was. A file whose contents can't
  be written is reported once, the rest is restored, and the restore exits
  1, as BackOtter's restic does; stock rustic panicked.
- FreeBSD's ZFS project IDs (`zfs project`) are kept and restored
  (`freebsd.zfs_project`), as on illumos.
- `backup` stores a device ID only for nodes with more than one link (the
  default, `--set-devid hardlink`), now not for a hard-linked FIFO either,
  whose link count isn't stored; BackOtter's restic now stores device IDs
  by this same rule (its `device-id-for-hardlinks`, on by default there),
  so the two tools' snapshots of one tree have the same nodes. `restore`
  on illumos leaves an extended attribute a file has already with its
  node's value alone, as on every other system (see BACKOTTER.md in
  rustic_core).
- `backup` is about three times as fast as the static musl build was:
  rustic_core keeps zstd's contexts and the chunker's Rabin tables per
  thread, for the same bytes (see BACKOTTER.md in rustic_core, and
  "Building" below).

## Building

    ./build-backotter.sh [TARGET ...]     # default: this machine's target

It refuses to build unless Cargo.lock pins rustic_core at the version it
names (`bko_core`, now 0.13.0+backotter.3) and rustic_core's walkdir (see
BACKOTTER.md there). `[patch.crates-io]` in Cargo.toml takes both from
rustic_core's `backotter` branch, and Cargo.lock pins a commit of it. After
rustic_core's `backotter` branch moves:

1. Run `cargo update -p rustic_core -p walkdir`.
2. Commit Cargo.lock.
3. When rustic_core's version changes, change `bko_core` in
   build-backotter.sh to match.

To build against a local copy of rustic_core instead (for tests), point
both entries of `[patch.crates-io]` at its paths (`crates/core` and
`vendor/walkdir`) and run `cargo update -p rustic_core -p walkdir`, in a
copy of this repository.

Builds with rustic's default features, which leave out `self-update`, so
there is no command to replace this build with stock rustic. Binaries go to
`dist/`, named `rustic-backotter-VERSION-TARGET`.

Linux binaries are static, as BackOtter's restic is: a dynamic one needs
`/lib64/ld-linux-x86-64.so.2` and a glibc at least as new as the one it was
built against, and NixOS has only a stub loader there that refuses to start
it. Build them for `x86_64-unknown-linux-musl` (static by default; the
script refuses a Linux binary with a program interpreter). Like restic's
pure-Go user lookup and resolver, musl reads `/etc/passwd`, `/etc/group`,
`/etc/hosts` and `/etc/resolv.conf` itself and loads no NSS modules, so
names from LDAP, sssd or systemd (`systemd`, `resolve`, `mymachines`) don't
resolve. A static glibc build (`x86_64-unknown-linux-gnu`, built with
`-C target-feature=+crt-static`) is no substitute: glibc still loads those
modules for a name it doesn't find in the files, and the static binary
dies of SIGFPE inside `libnss_systemd.so.2` when a file's owner has no name
on a host with `systemd` in nsswitch.conf. musl's malloc is slow with many
threads, so the script builds musl targets with rustic's `mimalloc`
feature. Measured in round 23 on Chopin (12 threads), backing up a
35,000-path, 1.4 GB Debian tree as an unprivileged user (page-cached, into
a new repository in RAM), release builds, mean of six interleaved runs:

| build                                   | wall   | user   | sys    | peak RSS |
|-----------------------------------------|--------|--------|--------|----------|
| musl + mimalloc, before round 23        | 13.5 s | 31.6 s | 37.5 s | 1.09 GB  |
| musl + mimalloc (the build)             | 4.0 s  | 19.3 s | 3.8 s  | 1.20 GB  |
| musl, its own malloc                    | 5.1 s  | 19.4 s | 8.5 s  | 0.21 GB  |
| BackOtter's restic (three runs)         | 4.5 s  | 33.2 s | 3.8 s  | 0.48 GB  |

Without mimalloc the backup takes 26% longer, for a sixth of the memory;
mimalloc stays. Round 23's other change (rustic_core: zstd contexts and
Rabin tables kept per thread, see BACKOTTER.md there) made the first row
the second: musl's malloc mapped and unmapped zstd's workspaces for every
blob.

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
