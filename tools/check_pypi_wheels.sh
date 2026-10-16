#!/usr/bin/env bash
# Assert that every PyPI wheel of statute carries, byte for byte, the
# binary of the GitHub Release.
#
#   tools/check_pypi_wheels.sh <tag> <assets-dir> <out-dir>
#
# For each target it prints and compares two values:
#
#   release  sha256 of `statute` extracted here from the Release tarball in
#            <assets-dir>, once that tarball has passed its `.sha256`
#            sidecar, so the value is chained to what the Release publishes;
#   wheel    sha256 of `statute` extracted here from each wheel in
#            <out-dir> that belongs to the target, unpacked with `wheel
#            unpack`, not read from the directory the wheel was built from.
#
# It is red on the first mismatch. It also asserts the set of wheels
# exactly: each expected wheel is found once, no two patterns match the
# same file, and nothing else is in <out-dir>. Looping over whatever is
# there would pass on an empty directory, and only counting matches would
# pass with a stray wheel, which the upload would publish, or with one
# wheel carrying two tags standing in for two.

set -euo pipefail
shopt -s nullglob

fail() {
  printf 'check_pypi_wheels: %s\n' "$1" >&2
  exit 1
}

[[ $# -eq 3 ]] || fail 'usage: tools/check_pypi_wheels.sh <tag> <assets-dir> <out-dir>'
version=${1#statute-v}
[[ $version != "$1" ]] || fail "tag '$1' does not start with statute-v"
assets=$(cd "$2" && pwd)
out=$(cd "$3" && pwd)
work=$(mktemp -d "${TMPDIR:-/tmp}/statute-check-wheels.XXXXXX")
trap 'rm -rf "${work:?}"' EXIT

# target | one platform tag per wheel the target must have, the rows of
# tools/build_pypi_wheels.sh
mapping='
x86_64-unknown-linux-musl|manylinux_2_17_x86_64 musllinux_1_2_x86_64
aarch64-unknown-linux-musl|manylinux_2_17_aarch64 musllinux_1_2_aarch64
x86_64-apple-darwin|macosx_10_12_x86_64
aarch64-apple-darwin|macosx_11_0_arm64
'

sha256_of() {
  sha256sum "$1" | cut -d' ' -f1
}

matched=()
while IFS='|' read -r target wheel_tags; do
  [[ -n $target ]] || continue

  archive="statute-$target.tar.gz" sidecar="statute-$target.sha256"
  [[ -f "$assets/$archive" && -f "$assets/$sidecar" ]] || fail "$target: Release assets missing in $assets"
  (cd "$assets" && sha256sum --check --strict --quiet "$sidecar")
  mkdir -p "$work/release-$target"
  tar -xzf "$assets/$archive" -C "$work/release-$target" statute
  release=$(sha256_of "$work/release-$target/statute")

  for tag in $wheel_tags; do
    wheels=("$out"/statute-"$version"-py3-none-*"$tag"*.whl)
    [[ ${#wheels[@]} -eq 1 ]] ||
      fail "$target: expected exactly one wheel tagged $tag in $out, found ${#wheels[@]}"
    wheel=${wheels[0]}
    dir="$work/wheel-$tag"
    uvx --from 'wheel==0.45.1' wheel unpack --dest "$dir" "$wheel" >/dev/null
    binaries=("$dir"/statute-*/statute-*.data/scripts/statute)
    [[ ${#binaries[@]} -eq 1 ]] ||
      fail "$(basename "$wheel") does not hold exactly one .data/scripts/statute"
    actual=$(sha256_of "${binaries[0]}")
    printf '%s\n  release %s\n  wheel   %s  %s\n' "$target" "$release" "$actual" "$(basename "$wheel")"
    [[ $actual == "$release" ]] || fail "$(basename "$wheel") does not carry the Release binary"
    matched+=("$wheel")
  done
done <<<"$mapping"

[[ ${#matched[@]} -eq 6 ]] || fail "expected 6 wheels, compared ${#matched[@]}"

# The sorted matches, duplicates kept, must equal everything on disk, and
# hold no duplicate.
printf '%s\n' "${matched[@]}" | sort >"$work/matched"
find "$out" -type f | sort >"$work/present"
diff "$work/matched" "$work/present" >&2 ||
  fail "$out does not hold exactly the expected wheels (diff above: < expected, > present)"
[[ $(sort -u "$work/matched" | wc -l) -eq ${#matched[@]} ]] ||
  fail 'two expected tags matched the same wheel'
printf 'check_pypi_wheels: %s wheels carry the Release binary of their target\n' "${#matched[@]}"
