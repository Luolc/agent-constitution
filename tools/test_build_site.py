"""Tests for build_site's page and check_site_anchors: a built constitution
becomes a Hugo page in both languages, and the site check passes a page
whose links all have targets and fails a broken link or a leaked list
marker. Hugo itself runs in CI, not here."""

import unittest
from pathlib import Path

import build_site
import check_site_anchors

ROOT = Path(__file__).resolve().parent.parent

GOOD = """<html><body>
<nav><a href="#%e5%ba%8f%e8%a8%80">序言</a></nav>
<h2 id="序言">序言</h2>
<h2 id="article-i"><a id="general-provisions"></a>Article I. General provisions</h2>
<ul>
<li><a id="terms"></a><strong>§1</strong> In this text, see <a href="#terms.principal">(1)</a>:
<ul><li><a id="terms.principal"></a>(<strong>1</strong>) Principal, under <a href="#general-provisions">Art. I</a>.</li></ul>
</li>
</ul>
</body></html>"""


class PageTest(unittest.TestCase):
    def test_built_constitution_in_each_language(self):
        for lang, (_, label) in build_site.PAGES.items():
            with self.subTest(lang=lang):
                md = (ROOT / f"constitution/constitution.{lang}.md").read_text(encoding="utf-8")
                title = md.split("\n")[2]
                out = build_site.page(md, label, "0.6")
                self.assertTrue(out.startswith(f'+++\ntitle = "{title[2:]}"\n+++\n<div class="constitution">\n'))
                self.assertIn(f'{title}\n\n<p class="revision">{label} 0.6</p>\n', out)
                self.assertNotIn("tools/build_constitution.py", out)
                self.assertTrue(out.endswith("\n</div>\n"))
                self.assertIn(" {#general-provisions}\n", out)
                self.assertNotIn("## <a id=", out)

    def test_other_first_line(self):
        with self.assertRaises(ValueError):
            build_site.page("# Title\n\nText.\n", "Revision", "0.6")


class CheckTest(unittest.TestCase):
    def test_every_link_has_its_target(self):
        self.assertEqual(check_site_anchors.check(GOOD), [])

    def test_broken_link(self):
        broken = GOOD.replace('href="#terms.principal"', 'href="#terms.missing"')
        self.assertEqual(check_site_anchors.check(broken), ["link #terms.missing has no target on the page"])

    def test_leaked_list_marker(self):
        for marker in ("- <a id=\"x\"></a><strong>§4</strong>", "- <a id=\"x\"></a><strong>第四款</strong>"):
            with self.subTest(marker=marker):
                leaked = GOOD.replace("</ul>\n</body>", f"</ul>\n<p>Silence is not consent. {marker} Text.</p>\n</body>")
                self.assertEqual(len(check_site_anchors.check(leaked)), 1)


if __name__ == "__main__":
    unittest.main()
