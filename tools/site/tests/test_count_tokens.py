"""Tests for count_tokens.

The token count asks the API (mocked here) once per text, sends the
credential as its header, and survives a failure unless --require-tokens
is given; the JSON file gives each text's SHA-256 and tokens and the
commit.
"""

from collections.abc import Iterator
import contextlib
import hashlib
import http.client
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock
import urllib.error

from constitution_site import count_tokens

ROOT = Path(__file__).resolve().parents[3]
KEY = "test-key-not-real"
TOKEN = "test-token-not-real"
COMMIT = "0123456789abcdef0123456789abcdef01234567"
UNAUTHORIZED = urllib.error.HTTPError(
  "https://api.anthropic.com/",
  401,
  "Unauthorized",
  http.client.HTTPMessage(),
  None,
)


def tearDownModule():
  UNAUTHORIZED.close()


class CountTokensTest(unittest.TestCase):
  def count(self, environ=None, **urlopen):
    auth = count_tokens.credential(environ or {count_tokens.KEY_VARIABLE: KEY})
    assert auth
    stderr = io.StringIO()
    with (
      mock.patch("urllib.request.urlopen", **urlopen) as opened,
      contextlib.redirect_stderr(stderr),
    ):
      tokens = count_tokens.count_tokens("One two three.", "a-model", auth)
    return tokens, opened, stderr.getvalue()

  def test_no_credential(self):
    for environ in (
      {},
      {count_tokens.KEY_VARIABLE: "", count_tokens.TOKEN_VARIABLE: ""},
    ):
      with self.subTest(environ=environ):
        self.assertIsNone(count_tokens.credential(environ))

  def test_counted(self):
    both = {count_tokens.KEY_VARIABLE: KEY, count_tokens.TOKEN_VARIABLE: TOKEN}
    cases = [
      ({count_tokens.KEY_VARIABLE: KEY}, {"X-api-key": KEY}),
      (
        {count_tokens.TOKEN_VARIABLE: TOKEN},
        {"Authorization": f"Bearer {TOKEN}"},
      ),
      (both, {"Authorization": f"Bearer {TOKEN}"}),
    ]
    for environ, auth in cases:
      with self.subTest(environ=environ):
        response = mock.MagicMock()
        response.__enter__.return_value = io.BytesIO(b'{"input_tokens": 12345}')
        tokens, opened, stderr = self.count(environ, return_value=response)
        self.assertEqual(tokens, 12345)
        self.assertEqual(stderr, "")
        request = opened.call_args.args[0]
        self.assertEqual(
          request.full_url, "https://api.anthropic.com/v1/messages/count_tokens"
        )
        sent = {
          name: request.get_header(name)
          for name in ("X-api-key", "Authorization")
        }
        self.assertEqual(
          {name: value for name, value in sent.items() if value}, auth
        )
        self.assertIn(b'"model": "a-model"', request.data)

  def test_failures(self):
    cases = [
      (UNAUTHORIZED, "HTTP 401"),
      (TimeoutError("timed out"), "TimeoutError"),
      (urllib.error.URLError("no route"), "URLError"),
      (http.client.IncompleteRead(b'{"input_'), "IncompleteRead"),
    ]
    for error, shown in cases:
      with self.subTest(shown=shown):
        tokens, _, stderr = self.count(side_effect=error)
        self.assertIsNone(tokens)
        self.assertEqual(stderr, f"token count: {shown}; tokens not counted\n")


class CommitShaTest(unittest.TestCase):
  """commit_sha in this repo's checkout."""

  def test_full_sha(self):
    self.assertRegex(count_tokens.commit_sha() or "", r"^[0-9a-f]{40}$")


class MainTest(unittest.TestCase):
  """main on this repo's built constitution, the API mocked."""

  def main(self, argv, environ, urlopen):
    stderr = io.StringIO()
    with (
      tempfile.TemporaryDirectory() as tmp,
      mock.patch.dict(count_tokens.os.environ, environ, clear=True),
      mock.patch("urllib.request.urlopen", **urlopen) as opened,
      mock.patch(
        "constitution_site.count_tokens.commit_sha", return_value=COMMIT
      ),
      contextlib.redirect_stderr(stderr),
    ):
      out = Path(tmp) / "data/lengths.json"
      code = count_tokens.main(["--out", str(out), *argv])
      written = json.loads(out.read_text()) if out.exists() else None
    return code, written, opened, stderr.getvalue()

  def texts(self) -> Iterator[tuple[str, str, str]]:
    """Return the texts the count reads.

    Yields:
      The language, the version and the text of each count.
    """
    for lang in ("en", "zh"):
      for version, suffix in count_tokens.VERSIONS.items():
        path = count_tokens.output(f"{lang}{suffix}")
        md = (ROOT / path).read_text(encoding="utf-8")
        yield lang, version, count_tokens.counted(md)

  def test_text_from_title(self):
    for md in ("Generated.\n\n# Title\n\nText.\n", "# Title\n\nText.\n"):
      with self.subTest(md=md):
        self.assertEqual(count_tokens.counted(md), "# Title\n\nText.\n")

  def test_tokens_not_counted(self):
    for name, environ in (
      ("no credential", {}),
      ("count fails", {count_tokens.TOKEN_VARIABLE: TOKEN}),
    ):
      with self.subTest(name):
        code, written, _, _ = self.main(
          [], environ, {"side_effect": UNAUTHORIZED}
        )
        self.assertEqual(code, 0)
        assert written
        self.assertEqual(written["commit"], COMMIT)
        for lang, version, text in self.texts():
          self.assertEqual(
            written[lang][version],
            {
              "sha256": hashlib.sha256(text.encode()).hexdigest(),
              "tokens": None,
            },
          )
        code, written, _, stderr = self.main(
          ["--require-tokens"], environ, {"side_effect": UNAUTHORIZED}
        )
        self.assertEqual((code, written), (1, None))
        self.assertIn("tokens not counted", stderr)
        self.assertNotIn(TOKEN, stderr)

  def test_tokens_counted(self):
    response = mock.MagicMock()
    response.__enter__.side_effect = lambda: io.BytesIO(
      b'{"input_tokens": 12345}'
    )
    code, written, opened, _ = self.main(
      ["--require-tokens"],
      {count_tokens.TOKEN_VARIABLE: TOKEN},
      {"return_value": response},
    )
    self.assertEqual(code, 0)
    assert written
    sent = [json.loads(call.args[0].data) for call in opened.call_args_list]
    self.assertEqual(
      sorted(s["messages"][0]["content"] for s in sent),
      sorted(text for _, _, text in self.texts()),
    )
    # The model is the one web/hugo.toml names.
    self.assertEqual({s["model"] for s in sent}, {"claude-opus-5-5"})
    for lang, version, _ in self.texts():
      self.assertEqual(written[lang][version]["tokens"], 12345)


if __name__ == "__main__":
  unittest.main()
