#!/usr/bin/env bash
# Build BackOtter's rustic for each Rust target given (default: this
# machine's) into dist/.  See BACKOTTER.md.

set -euo pipefail

cd "$(dirname "$0")"
version=$(awk -F'"' '/^version = /{print $2; exit}' Cargo.toml)
[[ $version == *+backotter.* ]] || { echo "build-backotter: version is '$version', not X.Y.Z+backotter.N" >&2; exit 1; }
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
