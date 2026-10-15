"""Count the tokens of the built constitution for the static site.

Writes web/data/lengths.json, which Hugo reads (docs/design/static-site.md):
for each language and each version, the full one and the one for agents, the
SHA-256 of the text whose length the site gives (the generated file from its
title on) and its tokens, as counted by Anthropic's count_tokens API
for the model `params.tokenCount.model` in web/hugo.toml, and the full SHA
of the commit the count ran on, which the site links. The site shows a count
only when its SHA-256 matches the text it builds, so a file left from
another commit does not show stale numbers.

The credential comes from CONSTITUTION_TOKEN_COUNT_ACCESS_TOKEN or
CONSTITUTION_TOKEN_COUNT_API_KEY. Without either, or when a call fails, the
tokens are null and the site says they were not counted; with
--require-tokens the script fails instead and writes nothing.
"""

import argparse
from collections.abc import Mapping
import hashlib
import http.client
import json
import os
from pathlib import Path
import subprocess
import sys
import tomllib
import urllib.error
import urllib.request

from constitution_site import git_env

ROOT = Path(__file__).resolve().parents[4]
OUT = ROOT / "web/data/lengths.json"
KEY_VARIABLE = "CONSTITUTION_TOKEN_COUNT_API_KEY"  # sent as x-api-key
# A short-lived access token from Workload Identity Federation, sent as
# Authorization: Bearer; it wins over the key when both are set.
TOKEN_VARIABLE = "CONSTITUTION_TOKEN_COUNT_ACCESS_TOKEN"
COUNT_URL = "https://api.anthropic.com/v1/messages/count_tokens"
COUNT_TIMEOUT = (
  10  # seconds per connect or read; a stalled API must not hold up the build
)
VERSIONS = {"full": "", "agent": ".agent"}  # key in the JSON: file suffix


def edition() -> Path:
  """Return the edition directory.

  That is the one directory under constitution/ that holds an index.toml.

  Returns:
    The directory, relative to the checkout.

  Raises:
    ValueError: when there is none, or more than one.
  """
  dirs = sorted(p.parent for p in ROOT.glob("constitution/*/index.toml"))
  if len(dirs) != 1:
    raise ValueError(f"{len(dirs)} edition directories under constitution/")
  return dirs[0].relative_to(ROOT)


def output(name: str) -> Path:
  """Return a generated constitution, relative to the checkout.

  Args:
    name: the language, with `.agent` after it for the version for agents.

  Returns:
    The path.
  """
  return edition() / "generated" / f"constitution.{name}.md"


def counted(markdown: str) -> str:
  """Return the text whose length the site gives.

  That is the built constitution from its `# ` title on, anchors and link
  definitions included: the full version has a header line and a blank
  line above the title, the version for agents starts with it.

  Args:
    markdown: A built constitution.

  Returns:
    The text from the title on.
  """
  return markdown if markdown.startswith("# ") else markdown.split("\n", 2)[2]


def credential(environ: Mapping[str, str]) -> dict[str, str] | None:
  """Return the header that authenticates the count.

  Args:
    environ: The environment to read the credential from.

  Returns:
    The header, or None without a credential.
  """
  if token := environ.get(TOKEN_VARIABLE):
    return {"authorization": f"Bearer {token}"}
  if key := environ.get(KEY_VARIABLE):
    return {"x-api-key": key}
  return None


def count_tokens(text: str, model: str, auth: dict[str, str]) -> int | None:
  """Count the tokens of text for model.

  A failure prints one line with the HTTP status or the exception's class,
  never the request or the exception, which may hold the credential.

  Args:
    text: The text to count.
    model: The model to count for.
    auth: The header from credential().

  Returns:
    The tokens, or None when the call fails.
  """
  request = urllib.request.Request(
    COUNT_URL,
    data=json.dumps(
      {"model": model, "messages": [{"role": "user", "content": text}]}
    ).encode(),
    headers={
      **auth,
      "anthropic-version": "2023-06-01",
      "content-type": "application/json",
    },
  )
  try:
    with urllib.request.urlopen(request, timeout=COUNT_TIMEOUT) as response:
      return int(json.load(response)["input_tokens"])
  except urllib.error.HTTPError as e:
    print(f"token count: HTTP {e.code}; tokens not counted", file=sys.stderr)
  except (
    OSError,
    http.client.HTTPException,
    ValueError,
    KeyError,
    TypeError,
  ) as e:
    print(
      f"token count: {type(e).__name__}; tokens not counted", file=sys.stderr
    )
  return None


def commit_sha() -> str | None:
  """Return the full SHA of the commit the count runs on.

  Returns:
    The commit, or None outside git.
  """
  try:
    result = subprocess.run(
      ["git", "-C", str(ROOT), "rev-parse", "HEAD"],
      capture_output=True,
      text=True,
      env=git_env.clean(),
      timeout=COUNT_TIMEOUT,
    )
  except OSError, subprocess.TimeoutExpired:
    return None
  return result.stdout.strip() if result.returncode == 0 else None


def main(argv: list[str] | None = None) -> int:
  """Count the tokens and write the JSON file.

  Args:
    argv: The command-line arguments, or None for sys.argv.

  Returns:
    The exit status: 0 on success, 1 when --require-tokens fails.
  """
  parser = argparse.ArgumentParser(description=(__doc__ or "").split("\n")[0])
  parser.add_argument(
    "--out",
    type=Path,
    default=OUT,
    help="the JSON file (default: web/data/lengths.json)",
  )
  parser.add_argument(
    "--require-tokens",
    action="store_true",
    help="fail when a text's tokens are not counted (the Pages workflow)",
  )
  args = parser.parse_args(argv)
  with (ROOT / edition() / "index.toml").open("rb") as f:
    languages = tomllib.load(f)["languages"]
  with (ROOT / "web/hugo.toml").open("rb") as f:
    model = tomllib.load(f)["params"]["tokenCount"]["model"]
  auth = credential(os.environ)
  if not auth:
    print(
      f"neither {TOKEN_VARIABLE} nor {KEY_VARIABLE} is set; tokens not counted",
      file=sys.stderr,
    )
    if args.require_tokens:
      return 1
  lengths: dict[str, object] = {"commit": commit_sha()}
  for lang in languages:
    versions = lengths[lang] = {}
    for version, suffix in VERSIONS.items():
      source = output(f"{lang}{suffix}")
      text = counted((ROOT / source).read_text(encoding="utf-8"))
      tokens = count_tokens(text, model, auth) if auth else None
      if tokens is None and args.require_tokens:
        print(
          f"{source}: tokens not counted; --require-tokens fails",
          file=sys.stderr,
        )
        return 1
      versions[version] = {
        "sha256": hashlib.sha256(text.encode()).hexdigest(),
        "tokens": tokens,
      }
  args.out.parent.mkdir(parents=True, exist_ok=True)
  args.out.write_text(json.dumps(lengths, indent=2) + "\n", encoding="utf-8")
  return 0


if __name__ == "__main__":
  sys.exit(main())
