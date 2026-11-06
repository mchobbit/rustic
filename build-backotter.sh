#!/usr/bin/env bash
# Build BackOtter's rustic for each Rust target given (default: this
# machine's) into dist/.  See BACKOTTER.md.

set -euo pipefail

cd "$(dirname "$0")"
version=$(awk -F'"' '/^version = /{print $2; exit}' Cargo.toml)
[[ $version == *+backotter.* ]] || { echo "build-backotter: version is '$version', not X.Y.Z+backotter.N" >&2; exit 1; }
# The lock must pin BackOtter's rustic_core of this version, and its
# walkdir (BACKOTTER.md, "Building"): after rustic_core's version changes,
# cargo update -p rustic_core -p walkdir refreshes the lock.
bko_core=0.13.0+backotter.5
locked() {  # the version and source Cargo.lock records for package $1
    awk -v name="$1" '$0 == "name = \"" name "\"" {p = 1; next}
        p && /^version = / {gsub(/"/, "", $3); v = $3}
        p && /^source = / {gsub(/"/, "", $3); s = $3}
        p && /^$/ {print v, (s == "" ? "path" : s); exit}' Cargo.lock
}
core_version=$(locked rustic_core | cut -d' ' -f1)
[[ $core_version == "$bko_core" ]] || {
    echo "build-backotter: Cargo.lock pins rustic_core $core_version; expected $bko_core: push rustic_core and run cargo update -p rustic_core" >&2
    exit 1
}
walkdir_source=$(locked walkdir | cut -d' ' -f2)
[[ $walkdir_source != registry+* ]] || {
    echo "build-backotter: Cargo.lock pins crates.io's walkdir, not rustic_core's: patch it as BACKOTTER.md says and run cargo update -p walkdir" >&2
    exit 1
}
targets=("$@")
# The musl target of a glibc Linux target: -linux-gnu becomes -linux-musl,
# its ABI suffix kept (x86_64-unknown-linux-gnu -> x86_64-unknown-linux-musl,
# armv7-unknown-linux-gnueabihf -> armv7-unknown-linux-musleabihf,
# arm-unknown-linux-gnueabi -> arm-unknown-linux-musleabi).
musl_of() { printf '%s\n' "${1/-linux-gnu/-linux-musl}"; }
# With no target: this machine's, but musl on a glibc Linux: a static glibc
# build dies (SIGFPE) in NSS (below), and a dynamic one is refused.
if [[ ${#targets[@]} -eq 0 ]]; then
    host=$(rustc -vV | sed -n 's/^host: //p')
    if [[ $host == *-linux-gnu* ]]; then
        musl=$(musl_of "$host")
        rustc --print target-list | grep -xF -- "$musl" >/dev/null || {
            echo "build-backotter: this machine is $host, whose musl target would be $musl, which rustc doesn't have (rustc --print target-list); name a target (BACKOTTER.md, \"Building\")" >&2
            exit 1
        }
        host=$musl
    fi
    targets=("$host")
fi
for t in "${targets[@]}"; do
    [[ $t != *-linux-gnu* ]] || {
        echo "build-backotter: $t: a static glibc build dies with SIGFPE in NSS (a file owner without a name, systemd in nsswitch.conf), and a dynamic one isn't portable; build $(musl_of "$t") (BACKOTTER.md, \"Building\")" >&2
        exit 1
    }
done

mkdir -p dist
for t in "${targets[@]}"; do
    # Linux builds are static, as restic-bko is: a dynamic one needs
    # /lib64/ld-linux-x86-64.so.2 and the glibc it was built against, which
    # NixOS (a stub loader) and older systems don't give it.  BackOtter's
    # Linux target is x86_64-unknown-linux-musl, static by default: a static
    # glibc still loads NSS modules for names, and dies (SIGFPE) in one when
    # a file's owner has no name on a host with systemd in nsswitch.conf.
    # musl, like Go without cgo, reads /etc/passwd, /etc/group and
    # /etc/hosts itself (BACKOTTER.md, "Building").  musl's malloc is slow
    # with many threads, so musl builds take mimalloc: without it a backup
    # took 26% longer (round 23: 5.1 s, not 4.0 s, for a 35,000-path tree),
    # though with a sixth of the memory (0.2 GB, not 1.2 GB; BACKOTTER.md,
    # "Building").  *-linux-gnu targets are refused above.
    flags=${RUSTFLAGS:-} features=()
    [[ $t == *-linux-musl* ]] && features=(--features mimalloc)
    # Default features: not "release", which adds self-update, which would
    # replace this build with stock rustic.  Without PROJECT_VERSION, which
    # rustic's own build.sh sets from git describe, the version is Cargo's.
    env -u PROJECT_VERSION RUSTFLAGS="$flags" cargo build --release --locked --target "$t" "${features[@]}"
    if [[ $t == *-linux-* ]] && readelf -l "target/$t/release/rustic" | grep -q 'program interpreter'; then
        echo "build-backotter: target/$t/release/rustic is linked dynamically" >&2
        exit 1
    fi
    out=dist/rustic-backotter-$version-$t
    cp "target/$t/release/rustic" "$out"
    echo "$out"
done
