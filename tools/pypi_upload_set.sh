#!/usr/bin/env bash
# Leave in a directory only the wheels PyPI does not hold yet, so that a
# publish job that failed part-way through its upload can be rerun.
#
#   tools/pypi_upload_set.sh <project> <version> <dir>
#
# PyPI never takes a file name twice, so a rerun that uploads everything
# fails on the wheels that did land, and the upload action's
# `skip-existing` cannot tell "this file, already uploaded" from "another
# file under this name". This script asks PyPI's JSON API what it holds for
# <project> <version>, and for each wheel in <dir>:
#
#   not on PyPI                        kept, to be uploaded
#   on PyPI with the same sha256       deleted from <dir>, already there
#   on PyPI with a different sha256    exit 1, before anything is uploaded
#
# A version PyPI answers 404 for keeps every wheel; any other answer than
# 200 or 404 is an exit 1, since not knowing what PyPI holds is not knowing
# that it holds nothing. It reads only the public API, with no credential.

set -euo pipefail
shopt -s nullglob

fail() {
  printf 'pypi_upload_set: %s\n' "$1" >&2
  exit 1
}

[[ $# -eq 3 ]] || fail 'usage: tools/pypi_upload_set.sh <project> <version> <dir>'
project=$1 version=$2 dir=$3
wheels=("$dir"/*.whl)
[[ ${#wheels[@]} -gt 0 ]] || fail "no wheels in $dir"

remote=$(mktemp "${TMPDIR:-/tmp}/statute-pypi.XXXXXX")
trap 'rm -f "${remote:?}"' EXIT
status=$(curl -sS -o "$remote" -w '%{http_code}' "https://pypi.org/pypi/$project/$version/json")
case $status in
  404)
    printf '%s %s is not on PyPI; all %s wheels are to be uploaded\n' "$project" "$version" "${#wheels[@]}"
    exit 0
    ;;
  200) ;;
  *) fail "PyPI answered HTTP $status for $project $version" ;;
esac

# Every verdict first, then the deletions: a mismatch on a later wheel
# leaves the directory as it was.
drop=()
for wheel in "${wheels[@]}"; do
  name=$(basename "$wheel")
  local_sha=$(sha256sum "$wheel" | cut -d' ' -f1)
  remote_sha=$(jq -r --arg n "$name" '.urls[] | select(.filename == $n) | .digests.sha256' "$remote")
  if [[ -z $remote_sha ]]; then
    printf '%s: not on PyPI, to be uploaded\n' "$name"
  elif [[ $remote_sha == "$local_sha" ]]; then
    printf '%s: on PyPI with the same sha256 %s, dropped\n' "$name" "$local_sha"
    drop+=("$wheel")
  else
    fail "$name is on PyPI with sha256 $remote_sha, this build has $local_sha"
  fi
done
for wheel in "${drop[@]}"; do
  rm -- "$wheel"
done
