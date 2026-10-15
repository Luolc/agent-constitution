"""Tests for site-rules, the data of the rules' pages.

On the made-up checkout of site_fixture: the order along the parent tree,
which rules the lists show, the language each node is shown in, the
citations as links, conflicts on both sides, the labels, the narrowest
provisions of an `authority`, and the rules under each provision and
within each article.
"""

from dataclasses import replace
import json
from pathlib import Path
import tempfile
import unittest

import site_fixture

from constitution_compiler.model import Errors
from constitution_compiler.read import load
from constitution_site import site_rules


class SiteRulesTest(unittest.TestCase):
  def setUp(self):
    tmp = tempfile.TemporaryDirectory()
    self.addCleanup(tmp.cleanup)
    self.root = Path(tmp.name)
    site_fixture.write(self.root)
    errors = Errors()
    src = load(self.root, errors)
    assert src is not None, errors
    self.data = site_rules.site_data(src)

  def nodes(self, lang):
    return {n["id"]: n for n in self.data[lang]["nodes"]}

  def test_tree_order(self):
    order = [(n["id"], n["depth"]) for n in self.data["en"]["nodes"]]
    self.assertEqual(order, [(1, 0), (2, 1), (3, 2), (4, 1), (5, 1), (6, 2)])
    nodes = self.nodes("en")
    self.assertEqual([g["id"] for g in nodes[3]["groups"]], [1, 2])
    self.assertEqual(
      [g["kind"] for g in nodes[3]["groups"]], ["domain", "one-of"]
    )
    self.assertEqual([g["id"] for g in nodes[6]["groups"]], [1])
    self.assertEqual(
      nodes[5]["children"],
      [{"id": 6, "label": "R6", "title": "Rule 6", "key": "6"}],
    )

  def test_subdomain(self):
    """A domain under a domain heads the rules below it, under its own."""
    (self.root / "rules/2.toml").write_text(
      site_fixture.group(2, "domain", 1), encoding="utf-8"
    )
    nodes = {n["id"]: n for n in self.load()["en"]["nodes"]}
    self.assertEqual((nodes[2]["kind"], nodes[2]["depth"]), ("domain", 1))
    self.assertEqual(
      [(g["id"], g["kind"]) for g in nodes[3]["groups"]],
      [(1, "domain"), (2, "domain")],
    )
    self.assertEqual([c["id"] for c in nodes[1]["children"]], [2, 4, 5])
    self.assertEqual([c["id"] for c in nodes[2]["children"]], [3])

  def test_other_sources(self):
    """A rule under another source's domain, or overriding its rule."""
    for name, old, new in (
      ("rules/6.toml", "parent = 5", 'parent = "team:7"'),
      ("rules/4.toml", "parent = 1", 'overrides = "team:8"'),
    ):
      path = self.root / name
      path.write_text(path.read_text().replace(old, new), encoding="utf-8")
    data = self.load()["en"]["nodes"]
    order = [(n["id"], n["depth"]) for n in data]
    self.assertEqual(order, [(1, 0), (2, 1), (3, 2), (5, 1), (4, 0), (6, 0)])
    nodes = {n["id"]: n for n in data}
    self.assertEqual(nodes[6]["parent"], {"ref": "team:7"})
    self.assertEqual(nodes[6]["groups"], [])
    self.assertEqual(
      (nodes[4]["parent"], nodes[4]["overrides"]), (None, {"ref": "team:8"})
    )
    self.assertIsNone(nodes[5]["overrides"])

  def load(self):
    errors = Errors()
    src = load(self.root, errors)
    assert src is not None, errors
    return site_rules.site_data(src)

  def test_superseded_is_not_under_provisions(self):
    nodes = self.nodes("en")
    self.assertEqual(
      nodes[4]["supersededBy"],
      [
        {"id": 5, "label": "R5", "title": "Rule 5", "key": "5"},
        {"id": 6, "label": "R6", "title": "Rule 6", "key": "6"},
      ],
    )
    self.assertEqual(nodes[4]["text"]["statement"], "Old.")
    # The superseded rule 4 also names `one`; only 5 is under it.
    cited = self.data["en"]["citedBy"]
    self.assertEqual([r["id"] for r in cited["one"]], [5])
    self.assertEqual([r["id"] for r in cited["one-a.x"]], [6])

  def test_shown_language(self):
    zh = self.nodes("zh")
    self.assertEqual((zh[3]["shown"], zh[3]["title"]), ("zh", "规定 3"))
    self.assertEqual(zh[3]["translations"], ["zh"])
    self.assertEqual(zh[1]["title"], "组 1")
    # A translation of an older original is shown as none is.
    for node_id in (4, 5):
      with self.subTest(node_id):
        self.assertEqual(zh[node_id]["shown"], "en")
        self.assertEqual(zh[node_id]["translations"], [])
        self.assertEqual(zh[node_id]["title"], f"Rule {node_id}")
    self.assertEqual(zh[5]["supersededBy"], [])
    self.assertEqual(
      zh[3]["parent"], {"id": 2, "label": "R2", "title": "组 2", "key": "2"}
    )

  def test_citations_are_links(self):
    en, zh = self.nodes("en"), self.nodes("zh")
    self.assertEqual(
      en[3]["text"]["statement"],
      "See [Art. I §1](../../#one-a) and [rule 6](../6/).",
    )
    self.assertEqual(
      zh[3]["text"]["statement"],
      "见[第一条第一款](../../#one-a)和[规定 6](../6/)。",
    )
    # An English text on a Chinese page keeps English labels.
    self.assertEqual(
      zh[6]["text"]["statement"],
      "Under five, see [Art. I §1(1)](../../#one-a.x).",
    )
    self.assertEqual(
      zh[6]["authority"], [{"ref": "one-a.x", "label": "第一条第一款第一项"}]
    )

  def test_applicability(self):
    en, zh = self.nodes("en"), self.nodes("zh")
    self.assertEqual(
      en[1]["applicability"], "For every repository; see [rule 3](../3/)."
    )
    self.assertEqual(zh[1]["applicability"], "所有仓库，见[规定 3](../3/)。")
    self.assertIsNone(en[2]["applicability"])

  def test_listed_only_in_force(self):
    errors = Errors()
    src = load(self.root, errors)
    assert src is not None, errors
    rule = next(n for n in src.nodes if n.id == 5)
    self.assertTrue(site_rules.listed(rule))
    for status in ("superseded", "repealed"):
      with self.subTest(status):
        self.assertFalse(site_rules.listed(replace(rule, status=status)))

  def test_label(self):
    for lang in ("en", "zh"):
      nodes = self.nodes(lang)
      self.assertEqual(
        [n["label"] for n in nodes.values()],
        ["R1", "R2", "R3", "R4", "R5", "R6"],
      )
      self.assertEqual(nodes[3]["groups"][1]["label"], "R2")
    self.assertEqual(site_rules.rule_label(12), "R12")

  def test_authority_keeps_the_narrowest(self):
    """Rule 3 names `one` and `one-a`, under it; rule 5 names `one` alone."""
    nodes = self.nodes("en")
    self.assertEqual([a["ref"] for a in nodes[3]["authority"]], ["one-a"])
    self.assertEqual([a["ref"] for a in nodes[5]["authority"]], ["one"])
    cited = self.data["en"]["citedBy"]
    self.assertEqual([r["id"] for r in cited["one-a"]], [3])
    self.assertNotIn(3, [r["id"] for r in cited["one"]])

  def test_cited_within_an_article(self):
    """An article counts the rules that cite it or a provision under it."""
    within = self.data["zh"]["citedWithin"]
    self.assertEqual(list(within), ["one"])
    self.assertEqual(
      within["one"],
      [
        {"id": 3, "label": "R3", "title": "规定 3", "key": "3"},
        {"id": 5, "label": "R5", "title": "Rule 5", "key": "5"},
        {"id": 6, "label": "R6", "title": "Rule 6", "key": "6"},
      ],
    )

  def test_conflicts_on_both_sides(self):
    nodes = self.nodes("en")
    self.assertEqual(
      nodes[5]["conflicts"],
      [
        {"id": 3, "label": "R3", "title": "Rule 3", "key": "3"},
        {"ref": "team:7"},
      ],
    )
    self.assertEqual(
      nodes[3]["conflicts"],
      [{"id": 5, "label": "R5", "title": "Rule 5", "key": "5"}],
    )
    self.assertEqual(
      nodes[5]["requires"],
      [{"id": 3, "label": "R3", "title": "Rule 3", "key": "3"}],
    )
    self.assertEqual(nodes[6]["conflicts"], [])

  def test_metadata(self):
    rule = self.nodes("en")[3]
    for field, value in (
      ("revision", 2),
      ("kind", "rule"),
      ("status", "active"),
      ("lang", "en"),
      ("level", "project"),
      ("constitution", ">=1"),
      ("check", "review"),
    ):
      self.assertEqual(rule[field], value)
    self.assertEqual(self.nodes("en")[1]["text"], {})

  def test_main_writes_the_file(self):
    out = self.root / "web/data/rules.json"
    self.assertEqual(
      site_rules.main(["--root", str(self.root), "--out", str(out)]), 0
    )
    self.assertEqual(json.loads(out.read_text(encoding="utf-8")), self.data)
    (self.root / "rules/6.toml").write_text("id = 6\n", encoding="utf-8")
    self.assertEqual(
      site_rules.main(["--root", str(self.root), "--out", str(out)]), 1
    )

  def other(self, enabled=None):
    errors = Errors()
    src = load(self.root, errors)
    assert src is not None, errors
    demo = self.root / "demo"
    site_fixture.write_other(demo)
    other = site_rules.load_source(demo, "demo", "global-private", src, errors)
    self.assertEqual(list(errors), [])
    return site_rules.site_data(src, [other], enabled)

  def test_one_source(self):
    """Alone, this repo's nodes have no source heading and no `enabled`."""
    self.assertEqual(
      self.data["en"]["sources"], [{"name": "R", "quadrant": "global-public"}]
    )
    nodes = self.nodes("en")
    self.assertEqual([g["key"] for g in nodes[3]["groups"]], ["1", "2"])
    self.assertNotIn("enabled", nodes[3])
    self.assertEqual(
      (nodes[3]["key"], nodes[3]["source"], nodes[3]["quadrant"]),
      ("3", "R", "global-public"),
    )

  def test_other_source(self):
    """A second source follows, its nodes linked across the two."""
    data = self.other(enabled={"R3", "demo:2"})["en"]
    self.assertEqual(
      data["sources"],
      [
        {"name": "R", "quadrant": "global-public"},
        {"name": "demo", "quadrant": "global-private"},
      ],
    )
    self.assertEqual(
      [n["label"] for n in data["nodes"]],
      ["R1", "R2", "R3", "R4", "R5", "R6", "demo:1", "demo:2"],
    )
    nodes = {n["label"]: n for n in data["nodes"]}
    demo = nodes["demo:2"]
    self.assertEqual(
      (demo["key"], demo["source"], demo["quadrant"]),
      ("demo-2", "demo", "global-private"),
    )
    self.assertEqual(
      [(g["key"], g["kind"]) for g in demo["groups"]],
      [("source-demo", "source"), ("demo-1", "domain")],
    )
    self.assertEqual(
      [g["key"] for g in nodes["R3"]["groups"]], ["source-R", "1", "2"]
    )
    self.assertEqual(
      demo["requires"],
      [{"id": 3, "label": "R3", "title": "Rule 3", "key": "3"}],
    )
    self.assertEqual(
      demo["text"]["statement"],
      "After [rule R3](../3/), see [rule demo:2](../demo-2/).",
    )
    self.assertEqual(
      nodes["demo:1"]["applicability"], "For every repository; see rule 3."
    )
    self.assertEqual(
      [(label, nodes[label].get("enabled")) for label in ("R1", "R3", "R5")]
      + [("demo:2", demo.get("enabled"))],
      [("R1", None), ("R3", True), ("R5", False), ("demo:2", True)],
    )
    self.assertEqual(
      [r["label"] for r in data["citedBy"]["one-a"]], ["R3", "demo:2"]
    )

  def test_main_with_another_source(self):
    out = self.root / "web/data/rules.json"
    site_fixture.write_other(self.root / "demo")
    lock = self.root / "statute.lock"
    lock.write_text('[[rules]]\nrule = "demo:2"\n', encoding="utf-8")
    args = ["--root", str(self.root), "--out", str(out)]
    args += ["--source", f"demo:global-private:{self.root / 'demo'}"]
    self.assertEqual(site_rules.main([*args, "--enabled", str(lock)]), 0)
    self.assertEqual(
      json.loads(out.read_text(encoding="utf-8")),
      self.other(enabled={"demo:2"}),
    )


if __name__ == "__main__":
  unittest.main()
