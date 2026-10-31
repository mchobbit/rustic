#!/usr/bin/env bash
# Build BackOtter's rustic for each Rust target given (default: this
# machine's) into dist/.  See BACKOTTER.md.

set -euo pipefail

cd "$(dirname "$0")"
version=$(awk -F'"' '/^version = /{print $2; exit}' Cargo.toml)
[[ $version == *+backotter.* ]] || { echo "build-backotter: version is '$version', not X.Y.Z+backotter.N" >&2; exit 1; }
# The lock must pin BackOtter's rustic_core of this version, and its
# walkdir (BACKOTTER.md, "Building"): until rustic_core's backotter branch is
# pushed and the lock refreshed, it pins an older one and crates.io's walkdir.
bko_core=0.13.0+backotter.3
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
[[ ${#targets[@]} -gt 0 ]] || targets=("$(rustc -vV | sed -n 's/^host: //p')")

mkdir -p dist
for t in "${targets[@]}"; do
    # Default features: not "release", which adds self-update, which would
    # replace this build with stock rustic.  Without PROJECT_VERSION, which
    # rustic's own build.sh sets from git describe, the version is Cargo's.
    env -u PROJECT_VERSION cargo build --release --locked --target "$t"
    out=dist/rustic-backotter-$version-$t
    cp "target/$t/release/rustic" "$out"
    echo "$out"
done
