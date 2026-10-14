"""Tests for check_site_anchors.

The site check passes a page whose links all have targets and fails a
broken link, a link to a missing page, a link that starts with `/` and
leaves the site, a repeated id or a leaked list marker.
"""

from pathlib import Path
import tempfile
import unittest

import check_site_anchors

GOOD = """<html><body>
<nav><a href="#%e5%ba%8f%e8%a8%80">序言</a></nav>
<h2 id="序言">序言</h2>
<h2 id="article-i"><a id="general-provisions"></a>Article I. General provisions</h2>
<ul>
<li><a id="terms"></a><strong>§1</strong> In this text, see <a href="#terms.principal">(1)</a>:
<ul><li><a id="terms.principal"></a>(<strong>1</strong>) Principal, under <a href="#general-provisions">Art. I</a>.</li></ul>
</li>
</ul>
</body></html>"""  # noqa: E501


class CheckTest(unittest.TestCase):
  def test_every_link_has_its_target(self):
    self.assertEqual(check_site_anchors.check(GOOD), [])

  def test_broken_link(self):
    broken = GOOD.replace('href="#terms.principal"', 'href="#terms.missing"')
    self.assertEqual(
      check_site_anchors.check(broken),
      ["link #terms.missing has no target on the page"],
    )

  def test_links_to_other_pages(self):
    with tempfile.TemporaryDirectory() as site:
      site = Path(site)
      (site / "agent").mkdir()
      (site / "agent/index.html").write_text("")
      (site / "index.html").write_text("")
      links = '<a href="./agent/">a</a><a href="agent/index.html#x">b</a><a href="https://example.test/">c</a>'
      self.assertEqual(
        check_site_anchors.check(
          GOOD.replace("</body>", links + "</body>"), site / "index.html"
        ),
        [],
      )
      self.assertEqual(
        check_site_anchors.check(
          '<a href="../">up</a>', site / "agent/index.html"
        ),
        [],
      )
      self.assertEqual(
        check_site_anchors.check(
          '<a href="./zh/agent/">x</a>', site / "index.html"
        ),
        ["link ./zh/agent/ leads to no file of the site"],
      )

  def test_links_from_the_root(self):
    """A link that starts with `/` leads into the site, not the disk."""
    with tempfile.TemporaryDirectory() as site:
      site = Path(site)
      (site / "zh/agent").mkdir(parents=True)
      (site / "agent").mkdir()
      (site / "agent/index.html").write_text("")
      page = site / "zh/agent/index.html"
      self.assertEqual(
        check_site_anchors.check('<a href="/agent/">a</a>', page, site), []
      )
      # A file outside the site that exists on every machine this runs on.
      outside = Path(__file__).resolve()
      self.assertTrue(outside.is_file())
      self.assertEqual(
        check_site_anchors.check(f'<a href="{outside}">x</a>', page, site),
        [f"link {outside} leads to no file of the site"],
      )

  def test_duplicate_id(self):
    repeated = GOOD.replace('id="article-i"', 'id="terms"')
    self.assertEqual(
      check_site_anchors.check(repeated), ["duplicate id 'terms'"]
    )

  def test_leaked_list_marker(self):
    for marker in (
      '- <a id="x"></a><strong>§4</strong>',
      '- <a id="x"></a><strong>第四款</strong>',
    ):
      with self.subTest(marker=marker):
        leaked = GOOD.replace(
          "</ul>\n</body>",
          f"</ul>\n<p>Silence is not consent. {marker} Text.</p>\n</body>",
        )
        self.assertEqual(len(check_site_anchors.check(leaked)), 1)


if __name__ == "__main__":
  unittest.main()
