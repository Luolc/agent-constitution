"""The environment for a git subprocess that names its repository itself.

A git hook runs with GIT_DIR and GIT_INDEX_FILE set (in a worktree, both),
and they win over `git -C <dir>`: a git run from a hook on another
directory, a temporary repository in a test for one, acts on the repository
of the commit instead. Every git these tools start passes `env=clean()`
and finds its repository from `-C`.
"""

import functools
import os
import subprocess


@functools.cache
def local_vars() -> frozenset[str]:
  """List the variables `git rev-parse --local-env-vars` names.

  Returns:
    The names of git's repository-local environment variables.
  """
  return frozenset(
    subprocess.run(
      ["git", "rev-parse", "--local-env-vars"],
      capture_output=True,
      text=True,
      check=True,
    ).stdout.split()
  )


def clean() -> dict[str, str]:
  """Copy os.environ without the local variables of git.

  Returns:
    os.environ, read at each call, without the local variables of git.
  """
  return {k: v for k, v in os.environ.items() if k not in local_vars()}
