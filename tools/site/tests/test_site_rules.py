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

from constitution_site import site_rules


class SiteRulesTest(unittest.TestCase):
  def setUp(self):
    tmp = tempfile.TemporaryDirectory()
    self.addCleanup(tmp.cleanup)
    self.root = Path(tmp.name)
    site_fixture.write(self.root)
    self.data = self.load()

  def nodes(self, lang):
    return {n["id"]: n for n in self.data[lang]["nodes"]}

  def test_tree_order(self):
    order = [(n["id"], n["depth"]) for n in self.data["en"]["nodes"]]
    # The one-of's option, rule 3, counts no level for the one-of, which
    # heads no group.
    self.assertEqual(order, [(1, 0), (2, 1), (3, 1), (4, 1), (5, 1), (6, 1)])
    nodes = self.nodes("en")
    self.assertEqual([g["id"] for g in nodes[3]["groups"]], [1])
    self.assertEqual([g["kind"] for g in nodes[3]["groups"]], ["domain"])
    self.assertEqual([g["id"] for g in nodes[6]["groups"]], [1])
    self.assertEqual(
      nodes[2]["children"],
      [
        {
          "id": 3,
          "label": "R3",
          "title": "Rule 3",
          "titleHtml": "Rule 3",
          "key": "3",
        }
      ],
    )

  def test_domain_option(self):
    """An option that is a domain heads its rules; the one-of heads none."""
    site_fixture.domain_option(self.root)
    nodes = {n["id"]: n for n in self.load()["en"]["nodes"]}
    self.assertEqual(
      [(g["id"], g["kind"]) for g in nodes[3]["groups"]],
      [(1, "domain"), (7, "domain")],
    )
    self.assertEqual((nodes[7]["depth"], nodes[3]["depth"]), (1, 2))
    self.assertEqual((nodes[3]["domain"], nodes[3]["boxed"]), ("7", False))
    self.assertEqual([o["id"] for o in nodes[2]["options"]], [7])
    self.assertTrue(nodes[7]["optionDefault"])

  def test_subdomain(self):
    """A domain under a domain heads the rules below it, under its own."""
    site_fixture.subdomain(self.root)
    nodes = {n["id"]: n for n in self.load()["en"]["nodes"]}
    self.assertEqual((nodes[2]["kind"], nodes[2]["depth"]), ("domain", 1))
    self.assertEqual(
      [(g["id"], g["kind"]) for g in nodes[3]["groups"]],
      [(1, "domain"), (2, "domain")],
    )
    self.assertEqual([c["id"] for c in nodes[1]["children"]], [2, 4, 5, 6])
    self.assertEqual([c["id"] for c in nodes[2]["children"]], [3])

  def test_other_sources(self):
    """A rule under another source's domain, or overriding its rule."""
    for name, old, new in (
      ("rules/6.toml", "parent = 1", 'parent = "team:7"'),
      ("rules/4.toml", 'parent = 1\ndefault = "off"', 'overrides = "team:8"'),
    ):
      path = self.root / name
      path.write_text(path.read_text().replace(old, new), encoding="utf-8")
    data = self.load()["en"]["nodes"]
    order = [(n["id"], n["depth"]) for n in data]
    self.assertEqual(order, [(1, 0), (2, 1), (3, 1), (5, 1), (4, 0), (6, 0)])
    nodes = {n["id"]: n for n in data}
    self.assertEqual(nodes[6]["parent"], {"ref": "team:7"})
    self.assertEqual(nodes[6]["groups"], [])
    self.assertEqual(
      (nodes[4]["parent"], nodes[4]["overrides"]), (None, {"ref": "team:8"})
    )
    self.assertIsNone(nodes[5]["overrides"])

  def read(self, path=None):
    read = site_rules.load(path or self.root)
    assert read is not None
    return read

  def load(self):
    src, nodes, params = self.read()
    assert src is not None
    return site_rules.site_data(src, nodes, params=params)

  def test_superseded_is_not_under_provisions(self):
    nodes = self.nodes("en")
    self.assertEqual(
      nodes[4]["supersededBy"],
      [
        {
          "id": 5,
          "label": "R5",
          "title": "Rule 5",
          "titleHtml": "Rule 5",
          "key": "5",
        },
        {
          "id": 6,
          "label": "R6",
          "title": "Rule 6",
          "titleHtml": "Rule 6",
          "key": "6",
        },
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
    # An untranslated node is shown in its original.
    for node_id in (4, 5):
      with self.subTest(node_id):
        self.assertEqual(zh[node_id]["shown"], "en")
        self.assertEqual(zh[node_id]["translations"], [])
        self.assertEqual(zh[node_id]["title"], f"Rule {node_id}")
    self.assertEqual(zh[5]["supersededBy"], [])
    self.assertEqual(
      zh[3]["parent"],
      {
        "id": 2,
        "label": "R2",
        "title": "组 2",
        "titleHtml": "组 2",
        "key": "2",
      },
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
      "Rule six, see [Art. I §1(1)](../../#one-a.x).",
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
    _, nodes, _ = self.read()
    rule = next(n for n in nodes if n.id == 5)
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
      self.assertEqual(nodes[3]["groups"][0]["label"], "R1")
    self.assertEqual(site_rules.rule_label(12), "R12")

  def test_authority_keeps_the_narrowest(self):
    """Rule 3 names `one` and `one-a`, under it; rule 5 names `one` alone."""
    nodes = self.nodes("en")
    self.assertEqual([a["ref"] for a in nodes[3]["authority"]], ["one-a"])
    self.assertEqual([a["ref"] for a in nodes[5]["authority"]], ["one"])
    cited = self.data["en"]["citedBy"]
    self.assertEqual([r["id"] for r in cited["one-a"]], [3])
    self.assertNotIn(3, [r["id"] for r in cited["one"]])

  def test_cited_within_a_provision(self):
    """A provision with one under it counts the rules citing either.

    The article, and its section with the item rule 6 cites; not the
    item, with nothing under it.
    """
    within = self.data["zh"]["citedWithin"]
    self.assertEqual(list(within), ["one", "one-a"])
    self.assertEqual([r["id"] for r in within["one-a"]], [3, 6])
    self.assertEqual(
      within["one"],
      [
        {
          "id": 3,
          "label": "R3",
          "title": "规定 3",
          "titleHtml": "规定 3",
          "key": "3",
        },
        {
          "id": 5,
          "label": "R5",
          "title": "Rule 5",
          "titleHtml": "Rule 5",
          "key": "5",
        },
        {
          "id": 6,
          "label": "R6",
          "title": "Rule 6",
          "titleHtml": "Rule 6",
          "key": "6",
        },
      ],
    )

  def test_conflicts_on_both_sides(self):
    nodes = self.nodes("en")
    self.assertEqual(
      nodes[5]["conflicts"],
      [
        {
          "id": 3,
          "label": "R3",
          "title": "Rule 3",
          "titleHtml": "Rule 3",
          "key": "3",
        },
        {"ref": "team:7"},
      ],
    )
    self.assertEqual(
      nodes[3]["conflicts"],
      [
        {
          "id": 5,
          "label": "R5",
          "title": "Rule 5",
          "titleHtml": "Rule 5",
          "key": "5",
        }
      ],
    )
    self.assertEqual(
      nodes[5]["requires"],
      [
        {
          "id": 6,
          "label": "R6",
          "title": "Rule 6",
          "titleHtml": "Rule 6",
          "key": "6",
        }
      ],
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
      ("check", "none"),
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
    src, nodes, params = self.read()
    assert src is not None
    demo = self.root / "demo"
    if not demo.exists():
      site_fixture.write_other(demo)
    other = site_rules.Source("demo", "global-private", *self.read(demo)[1:])
    return site_rules.site_data(src, nodes, [other], enabled, params)

  def test_one_source(self):
    """Alone, this repo's nodes have no source heading and no `enabled`."""
    self.assertEqual(
      self.data["en"]["sources"], [{"name": "R", "quadrant": "global-public"}]
    )
    nodes = self.nodes("en")
    self.assertEqual([g["key"] for g in nodes[3]["groups"]], ["1"])
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
      ["R1", "R2", "R3", "R4", "R5", "R6", "demo:1", "demo:2", "demo:4"],
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
      [g["key"] for g in nodes["R3"]["groups"]], ["source-R", "1"]
    )
    self.assertEqual(
      demo["requires"],
      [
        {
          "id": 3,
          "label": "R3",
          "title": "Rule 3",
          "titleHtml": "Rule 3",
          "key": "3",
        }
      ],
    )
    self.assertEqual(
      demo["text"]["statement"],
      "After [rule R3](../3/), see [rule demo:2](../demo-2/).",
    )
    self.assertEqual(
      nodes["demo:1"]["applicability"], "For every repository; see rule R9."
    )
    self.assertEqual(
      [(label, nodes[label].get("enabled")) for label in ("R1", "R3", "R5")]
      + [("demo:2", demo.get("enabled"))],
      [("R1", None), ("R3", True), ("R5", False), ("demo:2", True)],
    )
    self.assertEqual(
      [r["label"] for r in data["citedBy"]["one-a"]], ["R3", "demo:2", "demo:4"]
    )
    # Under this repo's domain: no group of its own, filtered by that one.
    under = nodes["demo:4"]
    self.assertEqual(
      (under["depth"], [g["key"] for g in under["groups"]], under["domain"]),
      (0, ["source-demo"], "1"),
    )
    self.assertEqual((demo["domain"], nodes["R3"]["domain"]), ("demo-1", "1"))
    # A conflict across sources shows on both sides.
    self.assertEqual([c["label"] for c in under["conflicts"]], ["R5"])
    self.assertEqual(
      [c.get("label", c.get("ref")) for c in nodes["R5"]["conflicts"]],
      ["R3", "demo:4", "team:7"],
    )

  def test_params_are_read_past(self):
    """params.toml is no node, in this repo or another source."""
    params = (
      '[[parameter]]\nname = "language.docs"\ntype = "language"\n'
      'scope = "repo"\ndefault = "en"\n\n'
      '[parameter.title]\nen = "Language of documents"\n'
    )
    (self.root / "rules/params.toml").write_text(params, encoding="utf-8")
    site_fixture.write_other(self.root / "demo")
    (self.root / "demo/rules/params.toml").write_text(params, encoding="utf-8")
    data = self.other()["en"]
    self.assertEqual(
      [n["label"] for n in data["nodes"]],
      ["R1", "R2", "R3", "R4", "R5", "R6", "demo:1", "demo:2", "demo:4"],
    )

  def test_defaults_and_uses(self):
    """Each node's switch, the one-of's default option and a rule's uses."""
    nodes = self.nodes("en")
    self.assertEqual(
      [(i, n.get("default"), n["onByDefault"]) for i, n in nodes.items()],
      [
        (1, "off", False),
        (2, None, False),
        (3, None, False),
        (4, "off", False),
        (5, "off", False),
        (6, "off", False),
      ],
    )
    self.assertEqual(
      nodes[2]["defaultOption"],
      {
        "id": 3,
        "label": "R3",
        "title": "Rule 3",
        "titleHtml": "Rule 3",
        "key": "3",
      },
    )
    self.assertEqual(
      [(i, n.get("optionDefault")) for i, n in nodes.items()],
      [(1, None), (2, None), (3, True), (4, None), (5, None), (6, None)],
    )
    self.assertEqual(nodes[3]["uses"], ["instructions"])
    self.assertNotIn("uses", nodes[1])
    # The domain on, and rule 6 in it: rule 3 comes in as the one-of's
    # default; rule 4, on but superseded, does not.
    for name, old, new in (
      ("rules/1.toml", 'default = "off"', 'default = "on"'),
      ("rules/4.toml", 'default = "off"', 'default = "on"'),
      (
        "rules/6.toml",
        'default = "off"',
        'default = "on"\nuses = ["instructions", "review"]',
      ),
    ):
      path = self.root / name
      text = path.read_text(encoding="utf-8")
      self.assertIn(old, text)
      path.write_text(text.replace(old, new, 1), encoding="utf-8")
    nodes = {n["id"]: n for n in self.load()["en"]["nodes"]}
    self.assertEqual(
      [(i, n["onByDefault"]) for i, n in nodes.items()],
      [(1, True), (2, True), (3, True), (4, False), (5, False), (6, True)],
    )
    self.assertEqual(nodes[6]["uses"], ["instructions", "review"])

  def test_params(self):
    """A placeholder shows its default, marked; each source its page."""
    site_fixture.params(self.root)
    data = self.load()
    for lang, docs in (("en", "Chinese"), ("zh", "Chinese")):
      nodes = {n["id"]: n for n in data[lang]["nodes"]}
      # Rule 6 is untranslated: its English text names the language in
      # English on both languages' pages.
      self.assertEqual(
        nodes[6]["text"]["statement"],
        'Write in <a class="param-value" href="../params/#language.docs"'
        f' title="language.docs">{docs}</a>; label <code>x <a'
        ' class="param-value" href="../params/#work-items.label-done"'
        ' title="work-items.label-done">done &amp; dusted</a></code>.',
      )
      self.assertEqual(nodes[6]["title"], "Rule 6")
    pages = data["zh"]["params"]
    self.assertEqual(
      [(p["key"], p["source"]) for p in pages], [("params", "R")]
    )
    docs, label = pages[0]["params"]
    self.assertEqual(
      docs,
      {
        "name": "language.docs",
        "type": "language",
        "values": None,
        "scope": "repo",
        "default": "zh",
        "shown": "中文",
        "title": "Language of documents",
        "lang": "en",
        "rules": [
          {
            "id": 6,
            "label": "R6",
            "title": "Rule 6",
            "titleHtml": "Rule 6",
            "key": "6",
          }
        ],
      },
    )
    self.assertEqual((label["title"], label["lang"]), ("完成项的标签", "zh"))
    self.assertEqual(self.data["en"]["params"], [])

  def test_params_of_two_sources(self):
    """A rule of another source names its own parameter or this repo's."""
    site_fixture.params(self.root)
    demo = self.root / "demo"
    site_fixture.write_other(demo)
    (demo / "rules/params.toml").write_text(
      site_fixture.PARAMS.split("[[parameter]]")[0]
      + "[[parameter]]"
      + site_fixture.PARAMS.split("[[parameter]]")[2],
      encoding="utf-8",
    )
    rule = demo / "rules/2.toml"
    rule.write_text(
      rule.read_text(encoding="utf-8").replace(
        "see {@rule:2}.",
        "see {@rule:2}, in {$language.docs}, `{$work-items.label-done}`.",
      ),
      encoding="utf-8",
    )
    data = self.other()["en"]
    text = {n["label"]: n for n in data["nodes"]}["demo:2"]["text"]
    self.assertIn('href="../params/#language.docs"', text["statement"])
    self.assertIn(
      'href="../demo-params/#work-items.label-done"', text["statement"]
    )
    pages = {p["key"]: p for p in data["params"]}
    self.assertEqual(list(pages), ["params", "demo-params"])
    rules = {
      (key, q["name"]): [r["label"] for r in q["rules"]]
      for key, page in pages.items()
      for q in page["params"]
    }
    self.assertEqual(
      rules,
      {
        ("params", "language.docs"): ["R6", "demo:2"],
        ("params", "work-items.label-done"): ["R6"],
        ("demo-params", "work-items.label-done"): ["demo:2"],
      },
    )

  def test_a_title_shows_the_value_alone(self):
    """A placeholder in a title: the value in the title, marked apart."""
    site_fixture.params(self.root)
    rule = self.root / "rules/5.toml"
    text = rule.read_text(encoding="utf-8")
    rule.write_text(
      text.replace('en = "Rule 5"', 'en = "Rule in {$language.docs}"'),
      encoding="utf-8",
    )
    nodes = {n["id"]: n for n in self.load()["en"]["nodes"]}
    self.assertEqual(nodes[5]["title"], "Rule in Chinese")
    self.assertEqual(
      nodes[5]["titleMarked"],
      'Rule in <a class="param-value" href="../params/#language.docs"'
      ' title="language.docs">Chinese</a>',
    )
    self.assertEqual(nodes[3]["conflicts"][0]["title"], "Rule in Chinese")

  def test_a_value_in_a_title_is_escaped(self):
    """A string with `<` in a title is text in the list, not HTML."""
    site_fixture.params(self.root)
    params = self.root / "rules/params.toml"
    params.write_text(
      params.read_text(encoding="utf-8").replace("done & dusted", "a <b> c"),
      encoding="utf-8",
    )
    rule = self.root / "rules/5.toml"
    text = rule.read_text(encoding="utf-8")
    rule.write_text(
      text.replace('en = "Rule 5"', 'en = "Label {$work-items.label-done}"'),
      encoding="utf-8",
    )
    node = {n["id"]: n for n in self.load()["en"]["nodes"]}[5]
    # As written for a template that escapes it; as HTML in a link.
    self.assertEqual(node["title"], "Label a <b> c")
    self.assertEqual(
      node["titleInLink"],
      'Label <span class="param-value" title="work-items.label-done">'
      "a &lt;b&gt; c</span>",
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
