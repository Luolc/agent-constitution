#!/usr/bin/env bash
# Build the PyPI wheels of statute from the binaries of a GitHub Release.
#
#   tools/build_pypi_wheels.sh <tag> <assets-dir> <out-dir>
#
# <tag> is statute-v<version>. <assets-dir> holds the eight Release assets
# of <tag>: for each of the four targets, `statute-<target>.tar.gz` and its
# sidecar `statute-<target>.sha256` (what `gh release download <tag>
# --pattern 'statute-*'` fetches). The wheels land in <out-dir>, which the
# upload action takes whole.
#
# Nothing is compiled here. Each binary is taken out of its Release
# tarball, once the tarball has passed its sidecar, and put into the wheel
# as it is, at `.data/scripts/statute`, which installers put on PATH.
# tools/check_pypi_wheels.sh reads the finished wheels back and asserts
# that they carry those bytes.
#
# The package version is the tag's, not the manifest's: the binaries are
# the tag's, and a workflow_dispatch rehearsal checks out main while the
# assets belong to an older tag. The summary, license and repository come
# from the statute crate in `cargo metadata`, and the long description and
# license file from crates/statute, all from the checkout.
#
# One row per Release target. Linux has one static musl binary per
# architecture, which also runs on glibc systems, so it goes into two
# wheels, a manylinux one and a musllinux one with the same file in them:
# pip picks by the libc it detects, and a single wheel carrying both tags is
# a shape PyPI is not known to accept. The macOS floors are the targets'
# minimum supported versions in rustc (10.12 on x86_64, 11.0 on aarch64).
# No Windows binary is built, so there is no Windows wheel.

set -euo pipefail

fail() {
  printf 'build_pypi_wheels: %s\n' "$1" >&2
  exit 1
}

[[ $# -eq 3 ]] || fail 'usage: tools/build_pypi_wheels.sh <tag> <assets-dir> <out-dir>'
tag=$1
assets=$(cd "$2" && pwd)
version=${tag#statute-v}
[[ $version != "$tag" ]] || fail "tag '$tag' does not start with statute-v"
[[ $version =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || fail "tag '$tag' is not statute-v<major>.<minor>.<patch>"

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
crate=$repo_root/crates/statute
work=$(mktemp -d "${TMPDIR:-/tmp}/statute-wheels.XXXXXX")
trap 'rm -rf "${work:?}"' EXIT
mkdir -p "$3"
out=$(cd "$3" && pwd)

manifest=$(cd "$repo_root" && cargo metadata --no-deps --format-version 1 --locked |
  jq -ec '.packages[] | select(.name == "statute")')
description=$(jq -er '.description' <<<"$manifest")
license=$(jq -er '.license' <<<"$manifest")
repository=$(jq -er '.repository' <<<"$manifest")

# target | wheel platform tags (space separated within a wheel, wheels
# separated by `,`)
mapping='
x86_64-unknown-linux-musl|manylinux_2_17_x86_64 manylinux2014_x86_64,musllinux_1_2_x86_64
aarch64-unknown-linux-musl|manylinux_2_17_aarch64 manylinux2014_aarch64,musllinux_1_2_aarch64
x86_64-apple-darwin|macosx_10_12_x86_64
aarch64-apple-darwin|macosx_11_0_arm64
'

# `sha256sum --check` reads the file name from the sidecar, so it runs in
# the assets directory.
extract_binary() {
  local target=$1 dest=$2
  local archive="statute-$target.tar.gz" sidecar="statute-$target.sha256"
  [[ -f "$assets/$archive" ]] || fail "$assets/$archive is missing"
  [[ -f "$assets/$sidecar" ]] || fail "$assets/$sidecar is missing"
  (cd "$assets" && sha256sum --check --strict "$sidecar")
  mkdir -p "$dest"
  tar -xzf "$assets/$archive" -C "$dest" statute
  [[ -x "$dest/statute" ]] || fail "$archive did not hold an executable statute"
}

build_wheel() {
  local tags=$1 binary=$2
  local dir="$work/wheel-${tags%% *}"
  local distinfo="$dir/statute-$version.dist-info"
  mkdir -p "$distinfo/licenses" "$dir/statute-$version.data/scripts"
  cp -p "$binary" "$dir/statute-$version.data/scripts/statute"
  cp "$crate/LICENSE" "$distinfo/licenses/LICENSE"
  {
    printf 'Metadata-Version: 2.4\n'
    printf 'Name: statute\n'
    printf 'Version: %s\n' "$version"
    printf 'Summary: %s\n' "$description"
    printf 'License-Expression: %s\n' "$license"
    printf 'License-File: LICENSE\n'
    printf 'Project-URL: Repository, %s\n' "$repository"
    printf 'Classifier: Programming Language :: Rust\n'
    printf 'Description-Content-Type: text/markdown\n'
    printf '\n'
    cat "$crate/README.md"
  } >"$distinfo/METADATA"
  {
    printf 'Wheel-Version: 1.0\n'
    printf 'Generator: tools/build_pypi_wheels.sh\n'
    printf 'Root-Is-Purelib: false\n'
    local t
    for t in $tags; do
      printf 'Tag: py3-none-%s\n' "$t"
    done
  } >"$distinfo/WHEEL"
  uvx --from 'wheel==0.45.1' wheel pack --dest-dir "$out" "$dir"
}

while IFS='|' read -r target tag_groups; do
  [[ -n $target ]] || continue
  extract_binary "$target" "$work/release-$target"
  IFS=',' read -ra groups <<<"$tag_groups"
  for tags in "${groups[@]}"; do
    build_wheel "$tags" "$work/release-$target/statute"
  done
done <<<"$mapping"

printf 'built into %s:\n' "$out"
ls -1 "$out"
