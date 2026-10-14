"""The environment for a git subprocess that names its repository itself.

A git hook runs with GIT_DIR and GIT_INDEX_FILE set (in a worktree, both),
and they win over `git -C <dir>`: a git run from a hook on another
directory, a temporary repository in a test for one, acts on the repository
of the commit instead. Every git these tools start passes `env=clean()`
and finds its repository from `-C`.
"""

import os
import subprocess


def clean() -> dict[str, str]:
    """os.environ without the variables `git rev-parse --local-env-vars` lists."""
    local = subprocess.run(["git", "rev-parse", "--local-env-vars"],
                           capture_output=True, text=True, check=True).stdout.split()
    return {k: v for k, v in os.environ.items() if k not in local}
