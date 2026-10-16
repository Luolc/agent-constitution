#!/usr/bin/env bash
# Asserts that the version Homebrew reads out of the statute formula is the
# release's.
#
#   tools/check_formula_version.sh statute-v0.1.17 luolc/local/statute
#
# `brew audit` passes a formula whose version is wrong just as it passes one
# whose version is right, so this asks Homebrew what it read and requires it
# to equal the tag without `statute-v`.
set -euo pipefail

fail() {
  printf 'check_formula_version: %s\n' "$1" >&2
  exit 1
}

[[ $# -eq 2 ]] || fail 'usage: tools/check_formula_version.sh <tag> <formula>'
tag=$1
formula=$2

[[ $tag =~ ^statute-v([0-9]+\.[0-9]+\.[0-9]+)$ ]] ||
  fail "\`$tag\` is not a statute-v<major>.<minor>.<patch> tag"
expected=${BASH_REMATCH[1]}

info=$(brew info --json=v2 "$formula") || fail "brew info could not read $formula"

# `brew info --json=v2` prints an object whose `formulae` array carries
# `versions.stable` for each formula. Any other shape fails here.
actual=$(printf '%s' "$info" | jq -er '
  if (.formulae | length) != 1 then
    error("brew info named \(.formulae | length) formulae")
  elif (.formulae[0].versions.stable | type) != "string" or (.formulae[0].versions.stable | length) == 0 then
    error("brew info reported no stable version")
  else .formulae[0].versions.stable end
') || fail "brew info did not report one stable version for $formula"

[[ $actual == "$expected" ]] ||
  fail "$formula is version $actual, expected $expected (from $tag)"
printf 'check_formula_version: %s is version %s, as %s says\n' "$formula" "$actual" "$tag"
