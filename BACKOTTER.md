# rustic for BackOtter

This branch (`backotter`) is rustic built on BackOtter's rustic_core: the
`backotter` branch of github.com/mchobbit/rustic_core, which carries the
patches BackOtter's rescue stick needs (see BACKOTTER.md there). It is not a
separate project: the branch is rebased onto every new rustic release rather
than merged.

## What it adds

- `[patch.crates-io]` in Cargo.toml, which swaps crates.io's rustic_core for
  BackOtter's. Cargo.lock pins the exact commit.
- Restoring `SNAP:subfolder` into a destination directory that is fresh
  gives it the subfolder's own metadata. Fresh means the restore creates
  it, or every entry found walking it (no symlink followed) is one of:
  (a) an entry a file system makes itself, whatever it holds
  (`lost+found`, a directory; FreeBSD UFS's `.snap`, a directory, and
  `.sujournal`, a regular file; nilfs2's `.nilfs`, an empty regular file,
  which mkfs.nilfs2 2.0.0 to 2.2.9 make, as in Ubuntu 24.04 and Zorin
  18; on Haiku, the empty `trash` directory Tracker makes on every volume
  as it is mounted, a new one too); (b) a mount point, a directory whose device (`st_dev`) differs
  from its parent's (also a ZFS dataset's or a btrfs subvolume's root,
  which have devices of their own), what it holds again (a), (b) or (c);
  (c) a directory on the path to one, holding only
  (a), (b) and (c), and at least one (b). That is a new file system's
  root, with the system's other new file systems mounted below it
  (`/boot`, `/boot/efi`, `/home`, ZFS datasets, btrfs subvolumes),
  BackOtter's bare-metal restore. Anything else (a file, a symlink, a
  device, a plain directory on the destination's own file system, even
  an empty one, a directory that can't be read), or more than 4096
  directories, means the destination isn't fresh. vfat (Linux's
  mkfs.fat, and the BSDs' newfs_msdos, whose volume label entry is never
  listed), jfs, f2fs, xfs, btrfs, ZFS, NetBSD, OpenBSD and DragonFly UFS,
  HAMMER2 and Haiku's BFS make no entries at all. A new nilfs2 made
  by mkfs.nilfs2 before 2.0.22 (2010), whose root also holds an empty
  regular file `.sketch`, isn't, nor is a ZFS dataset with
  `snapdir=visible`, whose root lists its `.zfs` directory (with the
  default, `snapdir=hidden`, it lists nothing). Nor is an existing
  destination that is sticky or that others may write in (mode bits
  01000 or 0002), even an empty one: an empty /tmp (in a container, on a
  system without systemd, a new tmpfs) or /dev/shm. mkfs, newfs,
  `zfs create` and `btrfs subvolume create` never make such a root.
  Where there are no device numbers (Windows) nothing is a mount point,
  and there are no such mode bits. As BackOtter's restic.
  Stock rustic leaves the destination as it was, so a restored file
  system's root keeps mkfs's mode and has no SELinux label. A destination
  that isn't fresh stays exactly as it is
  (owner, mode, times, extended attributes, ACLs, flags), as in stock
  rustic: `restore latest:/root /tmp` as root leaves /tmp 1777, even an
  empty /tmp or a freshly booted server's /tmp holding nothing but empty
  `.X11-unix`, `.ICE-unix` and `systemd-private-*/tmp` directories
  (`rustic_core::restore_target_is_fresh`, looked at before the restore
  starts).
- `restore` (and the TUI's) passes the snapshot's origin to rustic_core
  (`RestoreOptions::origin`, from its program version): in a snapshot of
  BackOtter's restic or rustic (`+backotter`) a node's missing attributes
  were zero or absent at backup (no flags, no project ID, no holes); in a
  stock restic or rustic one they are unknown, and the restore leaves the
  destination's as they are, as stock rustic does (see BACKOTTER.md in
  rustic_core).
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

Backups and restores work as BackOtter's restic does them (see BACKOTTER.md
in rustic_core). Where that needs other defaults than stock rustic's, the
defaults stay stock rustic's, and BackOtter passes the options:

- `backup` stores the mtime as each file's access time, as stock rustic
  does; `backup --set-atime yes` (or `set-atime = "yes"` in the `[backup]`
  config), which BackOtter passes, keeps each file's own, as BackOtter's
  restic does with `--with-atime`. Real atimes change whenever something
  reads a file, so they cost new trees on backups where nothing else
  changed. With them, on Linux the backup reads files and directories
  without changing their access times (`O_NOATIME`, where it may), except
  symlinks': `readlink(2)` moves them, with no way around it, though the
  snapshot keeps the earlier ones. Elsewhere (FreeBSD and the other BSDs,
  illumos, Haiku), with no `O_NOATIME`, listing a directory moves its
  access time: the backup takes a directory's metadata before it lists it,
  and with `--set-atime yes` doesn't scan the source for its size first
  (as `--no-scan`: no total or ETA), so that the snapshot has the access
  times from before the backup, as in BackOtter's restic. With the default
  the scan runs on every system, as in stock rustic. `rewrite --set-atime`
  defaults to `mtime`, as in stock rustic.
- `restore` looks owners up by name, falling back to the saved uid and
  gid, as stock rustic does (`--by-name`, or `--ownership-by-name`,
  restic's name for it, says so explicitly); `restore --numeric-id`, which
  BackOtter passes, gives files the uid and gid they were saved with, as
  restic does. A restore not running as root says nothing about an owner
  it can't set (`EPERM`, `EACCES`), as restic doesn't; stock rustic warned
  about each such file.
- `backup` skips Unix sockets and `restore` leaves out the socket nodes
  of older snapshots, as restic does (see BACKOTTER.md in rustic_core).
- `backup` stores the directories above the backed-up path (`/home` in a
  backup of `/home/m`) with their own metadata, as restic does, so a
  restore of the whole snapshot gives them their own mode, owner and
  times; stock rustic stored them as `0755` with no owner or times. It
  leaves out a device node's device number where it is 0, as restic does
  (see BACKOTTER.md in rustic_core).
- `backup` writes a regular file's mode where it is 0 (`/etc/shadow` on
  Rocky) as `"mode":0`, as stock rustic does (restic leaves it out; a
  restore of a restic or BackOtter snapshot gives a Unix file without one
  0000, and a stdin or Windows file the mode it is created with, as stock
  rustic); skips illumos doors, as it skips sockets; and
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
- `restore` over an existing tree fills the holes of a file it leaves as it
  is (a swap file left with holes by an interrupted restore) only in a
  snapshot of BackOtter's, for a file not marked `unix.sparse` (it had
  none), and not with `--sparse by-content`; it looks first, and opens for
  writing only a file that has holes. Stock restic and rustic snapshots
  mark no file, so their files are left as they are, as stock rustic
  leaves them (see BACKOTTER.md in rustic_core). Only Linux fills holes
  (`fallocate(2)`; not on ZFS, nor on btrfs but for a no-copy-on-write file,
  nor on bcachefs, which don't compress what it allocates); macOS and the other systems fill none and
  write nothing into a file whose contents already match, as BackOtter's
  restic: macOS's `F_PREALLOCATE` only adds space past a file's end. A
  file that got blocks counts as modified, even where the fill then failed
  (`ENOSPC`), and `--dry-run` counts one whose holes it would fill as
  modified, as BackOtter's restic reports it as updated.
- `merge` gives the snapshot it makes the program version of this build
  (`+backotter`) only if every snapshot merged is BackOtter's; otherwise
  the version without `+backotter` (`rustic 0.11.4`), so that a stock
  snapshot's missing attributes stay unknown (see BACKOTTER.md in
  rustic_core).
- `restore` makes files that had holes sparse again, where it restores them
  into new files: of each file system block a blob reaches into, the part
  inside the blob is written only if it isn't all zeros, so a block is
  allocated only where the file has data, as in BackOtter's restic. It
  allocates every other file in full, zeros and all, so that swap files
  and preallocated files come back allocated (on btrfs, only
  no-copy-on-write files, swap files among them, are preallocated, and on
  bcachefs none: the others get every byte, which compression keeps
  small); files restored over
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
  FreeBSD puts its `schg` libc back as it was: only where the snapshot says
  which flags the file had (a node with flags, or any node of a BackOtter
  snapshot, which records them wherever there are any). In a stock restic
  or rustic snapshot what the restore finds keeps its flags, as in stock
  rustic, and what they refuse fails with its error. What `--delete`
  removes never loses its flags: a protected file stays, with an error,
  as in stock rustic. A file whose contents can't
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

    ./build-backotter.sh [TARGET ...]     # default: this machine's target,
                                          # musl on a glibc Linux

It refuses to build unless Cargo.lock pins rustic_core at the version it
names (`bko_core`, now 0.13.0+backotter.6) and rustic_core's walkdir (see
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
script refuses a Linux binary with a program interpreter). With no target
on a glibc Linux (`*-linux-gnu*`) the script builds the musl target of the
same name, ABI suffix kept: `-linux-gnu` becomes `-linux-musl`
(`x86_64-unknown-linux-musl`, `armv7-unknown-linux-musleabihf` on an
`armv7-unknown-linux-gnueabihf` host, `*-musleabi` for `*-gnueabi`); where
rustc has no such target (`rustc --print target-list`) it stops and says
so. It refuses a `*-linux-gnu*` target named explicitly, saying why
(below) and naming its musl target. Like restic's
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

## Upstream fixes

0.11.4+backotter.6 takes one fix from rustic's `main` after 0.11.4,
cherry-picked (`git cherry-pick -x`): 8e5d8af (#1898) `docs` prints the
URL before trying to open it, so a failure to open it still shows it.

0.11.4+backotter.7 is 0.11.4+backotter.6 on rustic_core 0.13.0+backotter.6,
which restores a new Haiku volume's root (its empty Tracker `trash` no longer
makes it look used); only Haiku's binary differs.

0.11.4+backotter.6 builds on rustic_core 0.13.0+backotter.5, which has
these fixes from rustic_core's `main` after 0.13.0, cherry-picked (see
BACKOTTER.md there for how the two that conflicted were resolved):
52a1839 (#544) cache backend guarded by locks; c4059e3 (#548) and
d56cc4a (#630) canonicalization errors name the path; 35aef00 (#547)
the local backend's delete hook on a replaced file (in rustic_core's
`crates/backend`, which this build doesn't use: rustic_backend 0.7.0
comes from crates.io); 9d9e478 (#546) tree workers stop when the
receiver closes; d7e097c (#585) no underflow in
`BlobLocation::data_length`; 4589ea7 (#586) `repair snapshots` and file
nodes without contents; be62e24 (#588) directory nodes without a
subtree in `ls` and merges; 459bb53 (#589) no overflow in
`can_coalesce`; f5b8fd6 (#591) `BlobCopier` checks blob bounds; 82b81d2
(#592) bad scrypt parameters in key files refused; 0db92b6 (#607) tree
loader threads joined before the index goes; b77ecc8 (#590) a restore
refuses node paths that leave the destination (`..`, root, prefix);
fba52fa (#587) `prune` and directory nodes without a subtree; 4c3d6fa
(#633) `forget --keep-minutely` within one hour; 00bd025 (#632)
`backup` compares inode numbers unless `--ignore-inode`, as restic;
506a1da (#634) a restore's hard link over an entry at its name (the
right link left, anything else but a directory replaced); b583e92 (#602)
`init_with_config` takes `is_hot` from the repository.

Cargo.lock takes the first patched releases of crates this build ships
that RustSec has advisories against: zstd 0.13.4, zstd-safe 7.4.0,
zstd-sys 2.1.2 (zstd 1.5.7 still), rustls 0.23.46, rustls-webpki
0.103.16, ctutils 0.4.4; and chacha20 0.10.2, as 0.10.1 is yanked: its
SSE2 backend (picked without AVX2) runs an SSE4.1 instruction in the RNG
behind every nonce and `Id::random`, SIGILL on CPUs with neither (AMD
K8/K10, Core 2 Merom/Conroe; RustCrypto/stream-ciphers#579). Left as
they are: suppaftp 8.0.5 (RUSTSEC-2026-0271, FTP command injection
through CR/LF in arguments, for `opendal:ftp` repositories), fixed only
in suppaftp 10, which needs opendal 0.59 and so a rustic_backend newer
than crates.io's 0.7.0; rsa 0.9.10 (RUSTSEC-2023-0071), which has no
fixed release; zip 8.6.0 (RUSTSEC-2026-0367..0369, -0429), whose
advisories are about extracting archives, which rustic never does
(`dump` writes zip archives), and whose fix is zip 9, a breaking
release; quinn-proto, h2, webpki-roots and webpki-root-certs, which this
build doesn't compile; quick-xml 0.38.4 (RUSTSEC-2026-0194, -0195),
which only self-update takes (the `release` feature, not built; this
build has 0.41.0).

Not ported, from rustic's `main`: 143d073 (#1907) `[init]` in a config
profile, a feature.

Taken after 0.13.0+backotter.5 with no new version: 01fe284 (#640), in
rustic_core's `crates/backend`, which this build doesn't use.

Not ported, from rustic_core's `main`: 18fa55c (#642) an HTTP connect
timeout, a feature; e01b17f (#553) iOS build, no
BackOtter platform; ca81cfe (#549) and e574765 (#552) documentation
only; b7ab1ee (#556) more reqwest options, a breaking feature; 6fa7b56
(#559) serde and Merge for `ConfigOptions`, a feature; e97a7dc (#562)
OpenDAL retries on one line, a feature; b78f648 (#561) unreadable files
counted in the summary, a feature that changes snapshots' summaries;
647d00f (#563) backup paths no longer merged, a breaking change of
behaviour; 4871132 (#629) clippy fixes and dependency updates, breaking
(`Node`'s and `IndexEntry`'s constructors removed; its advisory fixes
are taken as above); e2b61aa (#558) HTTP options renamed, breaking;
9565d4f (#580) a refactor, no fix; eb72c72 (#598) an HTTP read timeout,
a feature.

## Moving to a new rustic release

First move rustic_core's `backotter` branch to the rustic_core release the
new rustic uses, then:

    git fetch upstream --tags
    git rebase --onto vNEW vOLD backotter
    # set Cargo.toml's version to "NEW+backotter.1"
    cargo update -p rustic_core
    git push --force-with-lease origin backotter
