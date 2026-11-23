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
- `restore --exact` and `--clone-from PATH=DIR` (only with `--exact`), for
  bko-holt load's shared btrfs snapshots: rustic_core does the work
  (`commands/restore/exact.rs`; see BACKOTTER.md there, with the known
  limits). Since 0.11.4+backotter.14 (rustic_core 0.13.0+backotter.13),
  as BackOtter's restic 0.19.1+backotter.12: a clone or a broken hard
  link goes into a new `O_EXCL` temporary file renamed over the file, a
  failed clone leaves the file as it was, a file whose link can't be
  broken is left as it is (an error), a hard-link group already exactly
  the snapshot's keeps its inode, restricting flags come off only for
  what the restore changes and go back on the other names of a file it
  replaced, `--delete` removes after the walk, and a second `--exact`
  restore over a matching tree changes nothing (mount it `noatime`).
  Since 0.11.4+backotter.15 (rustic_core 0.13.0+backotter.14): `--delete`
  leaves what is a snapshot entry under another name in a
  case-insensitive directory (`SAME` for `same`: .14 removed it after
  planning the file, then panicked, exit 134, the rest of the restore
  undone), a file that can't be read for the blobs planned from it is an
  error for the files that were to get them, not a panic, and the later
  names of a hard-link group `--exact` breaks are removed early, so that
  an unexpected immutable name of their old file goes too.
  Since 0.11.4+backotter.16 (rustic_core 0.13.0+backotter.15): on NetBSD,
  whose FFS moves a file's atime on the reads that verify its contents
  (no `O_NOATIME`), an entry that matches but for its atime gets that
  back alone, which moves no ctime there, so a second `--exact` restore
  changes nothing on NetBSD too (.15 set every compared file's metadata
  again). Don't mount a NetBSD destination `noatime`: its FFS then takes
  no atime from `utimensat(2)` and says nothing, so every restored atime
  is lost there.
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
  the scan runs on every system, as in stock rustic. `rewrite` keeps the
  stored atimes (`--set-atime yes`), as restic does; stock rustic's
  rewrite gave each node its mtime instead (`--set-atime mtime` still
  does).
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
  files on XFS (extent size hints, nodefrag, filestream, a directory's
  realtime inheritance, `t`, which needs no realtime device, ...:
  `linux.xfs_options`), on Linux, as restic does, and `restore` puts them
  back there, last, before the file flags (an extent size hint also as
  soon as it creates the file, while it is empty; a directory that was
  there, the destination too, loses its options until then, so that the
  entries made in it inherit none, and a restore that stops before, on an
  error or an interrupt, puts them back); `--dry-run-warmup` without `--dry-run` plans as `--dry-run`
  does, changing nothing in the destination; onto another file system they are one warning per
  restore. A dense restore leaves XFS's preallocated flag (`p`) on the
  files it preallocates, as restic's does (see BACKOTTER.md in
  rustic_core).
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
  the version without `+backotter` (`rustic 0.12.0`), so that a stock
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
- A partial `restore` (`--glob`, `--iglob`) gives the directories above
  its matches, which don't match themselves, their own metadata and
  casefolding, as BackOtter's restic does (stock rustic created them
  bare); and a backup keeps a file's other extended attributes when one
  can't be read, with a warning naming it (see BACKOTTER.md in
  rustic_core).
- `diff SNAP:PATH LOCALDIR` no longer counts the local top directory as
  added: this build's `LocalSource` gives that directory's own node (a
  backup keeps it), which the snapshot side has no entry for, and `diff`
  now leaves it out (`local_path` in `src/commands/diff.rs`). A
  regression of this build, not in stock rustic.
- `--strict-exit` (a global option; `RUSTIC_STRICT_EXIT`, or
  `strict-exit = true` in `[global]`) exits as BackOtter's restic does. A
  backup that left out entries, or parts of their metadata, it couldn't
  read still saves its snapshot, and exits 3 as a failure (`at least one
  source file could not be read: ...`, or with `--json`/`--json-progress`
  upstream's JSON `exit_error` line, code 3); what counts is what restic
  counts (see `SnapshotSummary::items_incomplete` in rustic_core's
  BACKOTTER.md), which is more than stock rustic's `error_count` (below). A
  restore that couldn't set metadata for a reason other than the target
  not supporting it (or, not running as root, one restic doesn't count
  then), make a special file, or remove or replace what was in its way
  exits 1 (`There were N errors`; rustic_core's BACKOTTER.md lists them).
  An interrupt (SIGINT, SIGTERM, SIGHUP) exits 130, except for `mount` and
  `webdav`, which an interrupt ends as they are meant to end ("press
  Ctrl-C to cancel"): they exit 0 with the after hooks, as BackOtter's
  restic's `mount` exits 0 then. Hooks: a non-zero exit runs the failed
  hooks at every level whose scope it ends, the global, repository and
  `[backup]` hooks, except the source-specific hooks of an incomplete
  snapshot, which was saved, and run their after hooks; an interrupt runs
  the global failed and finally hooks, not the after ones, and once it is
  taken the main thread starts no command-level hook command and no
  command: it stops before the next command of a hook list
  (`Hooks::run_all`), and after a `run-before` list and after the command
  whatever they returned (`hooks.use_with`; `halt_if_interrupted` and
  `unless_interrupted` in `src/application.rs`, which let the thread that
  shuts rustic down through, for its global hooks). A hook command that is
  running then finishes first: a SIGTERM sent to rustic alone doesn't
  reach it. Where the interrupt ends the hook's command too (Ctrl-C at a
  terminal), the main thread can see that command fail a moment before the
  interrupt is taken, and then still starts that level's failed hooks.
  0.12.0+backotter.1 stopped only before a `run-before` list and after
  the command: an interrupt during a `run-before` hook let the rest of its
  list and the command start beside the global failed hooks, or, the hook
  failing, that level's failed and finally hooks
  (`tests/strict_exit_interrupt.rs`). Unlike
  restic, which cancels what is under way, a backup's snapshot whose save
  was already under way while the failed hooks run can still be written
  (then the run exits 130 with that snapshot saved). Off by default:
  then a restore exits 0 with these as warnings and an interrupt exits 0
  with the after hooks, as in stock rustic; and a backup exits as stock
  rustic does since v0.12.0 (ed88576, #1908): 3 where rustic_core's
  `SnapshotSummary::error_count` isn't 0 (entries the walk or the archiver
  couldn't read, not parts of their metadata), as a graceful exit (`Warning:
  at least one source file could not be read`, the after hooks), else 0.
  Before 0.12.0+backotter.1 such a backup exited 0 without the option.
- Since 0.11.4+backotter.17 (rustic_core 0.13.0+backotter.17): paths past
  `PATH_MAX` (Linux and Android) are backed up and restored whole, a tree
  deeper than a path a system call takes (4096 bytes) included; stock
  rustic and 0.11.4+backotter.16 left out of a backup everything past it,
  warning "File name too long" and exiting 0 without `--strict-exit`, and
  couldn't restore it (`ENAMETOOLONG`). BackShot2's corpus case fail-001
  plants one (rustic_core 0.13.0+backotter.16, `walkdir::short_path`; see
  BACKOTTER.md there). It also takes two of RustAudit1's fixes: R2-01, a
  restored file with btrfs's compression property and immutable or
  append-only has its flags read back after `FS_IOC_FSSETXATTR` and all
  set through `FS_IOC_SETFLAGS` where they didn't take (on OpenZFS 2.2 it
  came back immutable instead of append-only); and R4-02, directory
  entries' names are read through the entry pointer, not libc's
  fixed-size `d_name` (undefined behaviour on illumos and Haiku and in the
  vendored walkdir's noatime reads; nothing read changes). rustic_core
  0.13.0+backotter.18 (RustAudit1 R3-01, R4-01) was taken in
  0.11.4+backotter.19.
- Since 0.11.4+backotter.19 (rustic_core 0.13.0+backotter.19): paths past
  `PATH_MAX` are kept with `$TMPDIR` on XFS too. 0.11.4+backotter.17 reached
  them through symbolic links to prefixes of up to 2048 bytes, and XFS
  refuses a link's target over 1024 (ext2/3/4 on 1 KiB blocks, over 1023):
  there a backup still warned "File name too long" and left the long paths
  out (29 entries backed up, 24 restored). The prefixes are now at most
  1000 bytes. It also takes rustic_core 0.13.0+backotter.18, RustAudit1's
  R3-01 (FreeBSD and NetBSD extended attributes are read by their names'
  bytes, so a non-UTF-8 name keeps its value) and R4-01 (illumos skips, with
  a warning, a snapshot's attribute names that aren't attribute files: '',
  '.', '..', a '/', NUL, the system views).
- Since 0.11.4+backotter.20 (rustic_core 0.13.0+backotter.20): paths past
  `PATH_MAX` are reached through links to prefixes of up to 2048 bytes
  again where `$TMPDIR`'s file system takes such a link, and of up to 1000
  only where it doesn't (XFS; ext2/3/4 on 1 KiB blocks), decided once per
  run. A system call follows at most 40 links in a path (`ELOOP` past
  it), so 0.11.4+backotter.19's 1000-byte prefixes everywhere had cut the
  deepest path kept from about 80 KB to about 40 KB (36 KB with 200-byte
  names) with `$TMPDIR` on tmpfs, ext4 or btrfs too; a tree of 50 KB paths
  that .17 kept left its deepest paths out of a backup there. With
  `$TMPDIR` on XFS the limit stays about 40 KB (14 KB at worst: 255-byte
  names under a 495-byte `$TMPDIR`); a deeper path fails (`ELOOP`) as
  before (BACKOTTER.md in rustic_core).
- Since 0.11.4+backotter.21 (rustic_core 0.13.0+backotter.21, RustAudit1's
  re-audit of paths past `PATH_MAX`, XR1-01 to XR2-04): a backup warns
  about a custom ignore file, `.gitignore` or `.git` past `PATH_MAX`, which
  the ignore crate can't read, and counts it as incomplete (exit 3 with
  `--strict-exit`), where .16 to .20 backed up what it excludes without a
  word. Messages name a long path itself, not the short one under
  `$TMPDIR/.bko-longpath-PID-N` (gone at exit). Why a long path can't be
  reached (no directory for the links in `$TMPDIR`, a link refused) is
  warned about once, where .16 to .20 said only "File name too long" for
  each path. The links are made in `$TMPDIR`'s canonical path, so a
  symbolic link in `$TMPDIR` no longer halves the depth reached, and a
  `$TMPDIR` of 501 to 1024 bytes is served again where it takes links to
  2048 bytes (.19 and .20 refused it everywhere). ext2/3/4 on 2 KiB
  blocks takes 1000-byte prefixes, as on 1 KiB blocks.
- Since 0.11.4+backotter.22 (rustic_core 0.13.0+backotter.22): RustAudit1's
  fixes in rustic_core's FreeBSD/NetBSD extended attribute and illumos
  extended attribute backends (formatting and doc wording); it behaves as
  .21 does.
- Since 0.12.0+backotter.1 (rustic_core 0.14.0+backotter.1): on rustic
  v0.12.0 and rustic_core 0.14.0 (see "Base: rustic v0.12.0" below); a
  backup that couldn't read some source files exits 3 without
  `--strict-exit` too, as stock rustic now does.

## Building

    ./build-backotter.sh [TARGET ...]     # default: this machine's target,
                                          # musl on a glibc Linux

It refuses to build unless Cargo.lock pins rustic_core at the version it
names (`bko_core`, now 0.14.0+backotter.1) and rustic_core's walkdir (see
BACKOTTER.md there). `[patch.crates-io]` in Cargo.toml takes both from
rustic_core's `backotter` branch, and Cargo.lock pins a commit of it. After
rustic_core's `backotter` branch moves:

1. Run `cargo update -p rustic_core --precise SHA`, SHA being the commit
   of rustic_core's `backotter` branch to take; walkdir's pin moves with
   it (one git source). Without `--precise` cargo takes the branch's tip
   on GitHub, which lacks commits not pushed yet. Until they are, let git
   fetch them from a local clone of rustic_core, for this and for the
   build: `CARGO_NET_GIT_FETCH_WITH_CLI=true GIT_CONFIG_COUNT=1
   GIT_CONFIG_KEY_0=url.CLONE.insteadOf
   GIT_CONFIG_VALUE_0=https://github.com/mchobbit/rustic_core.git`.
2. Check `git diff Cargo.lock`: only rustic-rs's version and the
   rustic_core and walkdir entries should change. Revert anything else
   cargo re-resolved (windows-sys and getrandom, for each of .19 to .22
   and for 0.12.0+backotter.1),
   and check that `cargo metadata --locked` takes the lock.
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

Linux binaries are static, as BackOtter's restic is: a dynamic one needs
`/lib64/ld-linux-x86-64.so.2` and a glibc at least as new as the one it was
built against, and NixOS has only a stub loader there that refuses to start
it. Build them for `x86_64-unknown-linux-musl` (static by default; the
script refuses a Linux binary with a program interpreter, and stops before
building one without `readelf` (binutils) to check it). With no target
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

## Base: rustic v0.12.0

Since 2026-11-22 the branch is on rustic v0.12.0 (c4ce22f, #1899, which
takes rustic_core 0.14.0 and rustic_backend 0.8.0), version
0.12.0+backotter.1 on rustic_core 0.14.0+backotter.1 (see "Base:
rustic_core 0.14.0" in BACKOTTER.md there); it was on v0.11.4
(0.11.4+backotter.1 to .22, on rustic_core 0.13.0+backotter.N). The rebase
kept every BackOtter commit but for these:

- d00e3a7, the cherry-pick of 8e5d8af (#1898, `docs` prints the URL before
  trying to open it), which v0.12.0 has.
- The commits that only moved Cargo.lock's pin of rustic_core (21c7087,
  77a296d, 42a7f4c, 46f51d9, 758c062, ea44449, a1f210b): v0.12.0's lock
  replaced theirs, so they were left empty. The rebased commits keep
  v0.12.0's Cargo.lock as it is; the first commit on v0.12.0 to pin
  rustic_core again is the one that makes 0.12.0+backotter.1. The versions
  in Cargo.toml of the commits from before read 0.12.0+backotter.N with
  their old N (their messages, build-backotter.sh's `bko_core` and their
  BACKOTTER.md name 0.11.4's and 0.13.0's).

With rustic_core 0.14.0's cherry-picks gone (they are upstream's: #544,
#546, #547, #548, #585 to #592 (#590 and #634 with the fork's own parts
kept, see there), #602, #607, #630, #632, #633, #634, #640),
so are the Cargo.lock pins of 0.11.4+backotter.4 to .6: v0.12.0's lock has
the same or newer releases (zstd 0.14.3, zstd-safe 8.3.0, zstd-sys 2.1.3,
still zstd 1.5.7, so the same compressed bytes; rustls 0.23.46,
rustls-webpki 0.103.17, ctutils 0.4.8, chacha20 0.10.2, rustix 1.1.6,
which builds for DragonFly), and fixes two of the advisories left open
before: suppaftp 10.0.2 (RUSTSEC-2026-0271, with opendal 0.59.5 and
rustic_backend 0.8.0) and zip 9.6.1 (RUSTSEC-2026-0367..0369, -0429). Left
as it is: rsa 0.9.10 (RUSTSEC-2023-0071), which has no fixed release.

What came with v0.12.0 and changes what users see:

- ed88576 (#1908): a backup that couldn't read some source files exits 3
  without `--strict-exit` too, gracefully (after hooks), with `Warning: at
  least one source file could not be read` (with `--json` or
  `--json-progress`, JSON `error` lines and a final `exit_error`, code 3);
  it counts rustic_core's `error_count` (#561). `--strict-exit` keeps its
  own path (above): it exits 3 as a failure on `items_incomplete`, which
  also counts parts of metadata, so where it counts more than
  `error_count` it is the only one to exit 3; its JSON output is upstream's
  `exit_error`. `test_backup_incomplete_exits_3_with_strict_exit` checks
  both, and upstream's `test_backup_unreadable_file_exits_3` runs as it is.
- 0051a02 (#1950): `copy` exits with an error when copying to a target
  failed (it exited 0).
- 8f5c891 (#1937): the TUI's `ls` doesn't panic on a directory node
  without a subtree.
- 143d073 (#1907): `[init]` in a config profile; ef5dd4b (#1952): `ls
  --long --json`; 87a6272 (#1938): `mount` fixes (the `mount` feature,
  which this build leaves out); 66383d8 (#1990), 5e34d1b (#1997, breaking)
  dependency updates (dav-server 0.12, zip 9, cached 2, self_update 1.3).
- rustic_core 0.14.0's own changes, among them #563 (backup paths no
  longer merged into their parents), #637 (chunk size bounds), #639
  (prune's rewrite: `keep-delete` is `keep-marked`), #638 (the chunker
  hashes the whole window, as rustic_core 0.13.0+backotter did already:
  the same chunks); see BACKOTTER.md there.

### Before v0.12.0

0.11.4+backotter.6 took 8e5d8af (#1898) from rustic's `main`,
cherry-picked, and rustic_core 0.13.0+backotter.5, which had rustic_core's
fixes above cherry-picked; .7 and .8 were .6 on rustic_core
0.13.0+backotter.6 (a new Haiku volume's root) and .7 (`--strict-exit`,
restored parents of partial restores, rewrite's atimes, per-attribute xattr
warnings, diff's local top directory). rustic_core's 01fe284 (#640), in
its `crates/backend`, which this build doesn't use, came after
0.13.0+backotter.5 with no new version.

## Moving to a new rustic release

First move rustic_core's `backotter` branch to the rustic_core release the
new rustic uses, then:

    git fetch upstream --tags
    git rebase --onto vNEW vOLD backotter
    # set Cargo.toml's version to "NEW+backotter.1"
    cargo update -p rustic_core --precise SHA   # as in "Building"
    # set bko_core in build-backotter.sh to rustic_core's new version
    git push --force-with-lease origin backotter

In the rebase, take the new release's Cargo.lock where ours conflicts (a
lock-only commit is then left empty, and dropped), and give a conflicting
version line `NEW+backotter.N`. After it the lock has crates.io's
rustic_core, which `--precise SHA` can't take (it wants a version there):
with rustic_core's `backotter` branch at SHA in the clone that
`url.CLONE.insteadOf` names ("Building"), run `cargo update -p rustic_core
-p walkdir`, which takes the patch, and check that the lock names SHA. Then
revert the rest of what cargo re-resolved, as in "Building" (for v0.12.0:
windows-sys 0.61.2 to 0.52.0 and getrandom 0.3.4 to 0.4.3 in dependency
lists), and check `cargo metadata --locked`.
