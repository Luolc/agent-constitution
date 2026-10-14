"""Tests for constitution_render.

A valid tree renders as expected in both languages, in full and for
agents.
"""

import unittest

import build_constitution as bc
import constitution_languages as cl
from test_constitution_fixture import (
  A,
  ALPHA_ONE,
  B,
  BETA,
  BETA_ONE,
  EN,
  EN_AGENT,
  plain,
  Tree,
  ZH,
  ZH_AGENT,
)


class RenderTest(Tree):
  def test_valid_tree_renders_both_languages(self):
    errors, _, outputs = bc.build(self.root)
    self.assertEqual(errors, [])
    self.assertEqual(
      outputs, {"en": EN, "zh": ZH, "en.agent": EN_AGENT, "zh.agent": ZH_AGENT}
    )

  def test_write_then_check(self):
    self.assertEqual(bc.main(["--root", str(self.root)]), 0)
    self.assertEqual(
      (self.root / "constitution/constitution.zh.md").read_bytes(), ZH.encode()
    )
    self.assertEqual(
      (self.root / "constitution/constitution.zh.agent.md").read_bytes(),
      ZH_AGENT.encode(),
    )
    self.assertEqual(bc.main(["--root", str(self.root), "--check"]), 0)

  def test_preamble_heading_is_marked(self):
    """A heading in the front text has the asterisk in the full version.

    The version for agents leaves out the front text, so a heading in it has
    the asterisk in the full version, and the header explains it.
    """
    self.edit(
      "constitution/index.toml",
      "The preamble cites",
      "## Preamble\n\nThe preamble cites",
    )
    self.edit(
      "constitution/index.toml", "序言引用{@beta}", "## 序言\n\n序言引用{@beta}"
    )
    errors, _, outputs = bc.build(self.root)
    self.assertEqual(errors, [])
    star = '<sup class="omitted">*</sup>'
    self.assertIn(f"\n## Preamble{star}\n\nThe preamble cites", outputs["en"])
    self.assertIn(f"\n## 序言{star}\n\n序言引用", outputs["zh"])
    for lang in ("en", "zh"):
      self.assertIn(cl.LANGUAGES[lang].legend + "\n", outputs[lang])
      self.assertNotIn(star, outputs[lang + bc.AGENT])

  def test_provisions_for_maintainers(self):
    """Provisions marked for maintainers keep only their numbers for agents.

    A section and an item marked for maintainers keep their text, with an
    asterisk, in the full version; the version for agents keeps only their
    numbers. The others are as without the mark.
    """
    self.edit(
      f"{B}/01a-beta-two.toml",
      'status = "draft"',
      'status = "draft"\naudience = "maintainers"',
    )
    self.edit(
      f"{A}/01-alpha-one.toml",
      'number = "2"\nstatus = "draft"',
      'number = "2"\nstatus = "draft"\naudience = "maintainers"',
    )
    errors, _, outputs = bc.build(self.root)
    self.assertEqual(errors, [])
    star = '<sup class="omitted">*</sup>'
    self.assertIn(
      f'\n-   <a id="beta-two"></a>**§1a**{star} Between:\n', outputs["en"]
    )
    self.assertIn(
      f'\n    -   <a id="alpha-one.machines"></a>(**2**){star} Machines;',
      outputs["en"],
    )
    self.assertIn(
      f'\n-   <a id="beta-two"></a>**第一款之一**{star} 之间：\n', outputs["zh"]
    )
    self.assertIn(
      f'\n    -   <a id="alpha-one.machines"></a>(**二**){star} 机器；',
      outputs["zh"],
    )
    self.assertIn(
      "Language: English. \\* Background for people: where this constitution "
      "comes from, how it"
      " is organized and how it is amended. The agent version,"
      " `constitution/constitution.en.agent.md`, omits these parts.\n",
      outputs["en"],
    )
    self.assertIn(
      "语言：中文。\\* 写给人看的背景：本宪法的由来、结构和修改方式。agent 版"
      " `constitution/constitution.zh.agent.md` 不收录这些部分。\n",
      outputs["zh"],
    )
    self.assertEqual(
      outputs["en"].replace(star, "").replace(cl.LANGUAGES["en"].legend, ""), EN
    )
    self.assertEqual(
      outputs["zh"].replace(star, "").replace(cl.LANGUAGES["zh"].legend, ""), ZH
    )
    self.assertEqual(
      outputs["en.agent"],
      EN_AGENT.replace(
        "    -   (**2**) Machines; {braces} and {@Nope} stay.\n        More.\n",
        "    -   (**2**) (see the full text)\n",
      ).replace(
        "-   **§1a** Between:\n    1. first\n\n       more\n    2. second\n",
        "-   **§1a** (see the full text)\n",
      ),
    )
    self.assertEqual(
      outputs["zh.agent"],
      ZH_AGENT.replace(
        "    -   (**二**) 机器；{braces} 与 {@Nope} 不变。\n        更多。\n",
        "    -   (**二**) (见完整版)\n",
      ).replace(
        "-   **第一款之一** 之间：\n    1. 第一\n\n       更多\n    2. 第二\n",
        "-   **第一款之一** (见完整版)\n",
      ),
    )

  def test_repealed_placeholders(self):
    self.activate()
    self.edit(
      f"{B}/01-beta-one.toml",
      'status = "active"',
      'status = "repealed"\nrepealed = "2026-11"',
    )
    self.edit(
      f"{A}/01-alpha-one.toml",
      'number = "2"\nstatus = "active"',
      'number = "2"\nstatus = "repealed"\nrepealed = "2026-11"',
    )
    self.write(
      "constitution/articles/03-gamma/article.toml",
      BETA.replace('"beta"', '"gamma"').replace(
        "[title]", 'repealed = "2026-12"\n\n[title]'
      ),
    )
    # A citation of a repealed section would be an error; move it.
    self.edit(f"{A}/01-alpha-one.toml", "{@beta-one}", "{@beta-two}", 2)
    self.edit(
      "rules/rule.toml",
      "{@constitution:beta-one}",
      "{@constitution:beta-two}",
      2,
    )
    errors, _, outputs = bc.build(self.root)
    self.assertEqual(errors, [])
    self.assertIn(
      '\n    -   <a id="alpha-one.machines"></a>(**2**) (repealed November '
      "2026)\n",
      outputs["en"],
    )
    self.assertIn(
      '\n-   <a id="beta-one"></a>**§1** (repealed November 2026)\n',
      outputs["en"],
    )
    self.assertIn(
      '\n## <a id="gamma"></a>Article III (repealed December 2026)\n\n[',
      outputs["en"],
    )
    self.assertIn(
      '\n    -   <a id="alpha-one.machines"></a>(**二**) (2026 年 11 月废止)\n',
      outputs["zh"],
    )
    self.assertIn(
      '\n-   <a id="beta-one"></a>**第一款** (2026 年 11 月废止)\n',
      outputs["zh"],
    )
    self.assertIn(
      '\n## <a id="gamma"></a>第三条 (2026 年 12 月废止)\n\n[', outputs["zh"]
    )

  def test_inserted_item(self):
    self.edit(
      f"{A}/01-alpha-one.toml",
      'id = "machines"\nnumber = "2"',
      'id = "machines"\nnumber = "1a"',
    )
    self.edit(
      f"{B}/01-beta-one.toml", "{@alpha-one.money}", "{@alpha-one.machines}", 2
    )
    errors, _, outputs = bc.build(self.root)
    self.assertEqual(errors, [])
    self.assertIn(
      '\n    -   <a id="alpha-one.machines"></a>(**1a**) Machines;',
      outputs["en"],
    )
    self.assertIn(
      "**§1** Four, see [Art. I §1(1a)][alpha-one.machines] and", outputs["en"]
    )
    self.assertIn(
      '\n    -   <a id="alpha-one.machines"></a>(**一之一**) 机器；',
      outputs["zh"],
    )
    self.assertIn(
      "**第一款** 四，见[第一条第一款第一项之一][alpha-one.machines]和",
      outputs["zh"],
    )

  def test_section_of_several_paragraphs_before_the_next(self):
    """A section's later paragraphs stay in it and out of the next one.

    A nested list and a further paragraph sit at the section's content
    column, and a blank line keeps the next section's line out of that
    paragraph: Python-Markdown nests only what is indented four spaces
    and joins a list line to the paragraph just above it.
    """
    self.edit(f"{B}/01a-beta-two.toml", "2. second\n", "2. second\n\nAfter.\n")
    self.edit(f"{B}/01a-beta-two.toml", "2. 第二\n", "2. 第二\n\n之后。\n")
    self.write(
      f"{B}/02-beta-three.toml",
      'id = "beta-three"\nstatus = "draft"\n\n[content]\nen = "Next."\nzh = '
      '"下一款。"\n',
    )
    errors, _, outputs = bc.build(self.root)
    self.assertEqual(errors, [])
    self.assertIn(
      "**§1a** Between:\n    1. first\n\n       more\n    2. second\n\n"
      '    After.\n\n-   <a id="beta-three"></a>**§2** Next.\n',
      outputs["en"],
    )
    self.assertIn(
      "**第一款之一** 之间：\n    1. 第一\n\n       更多\n    2. 第二\n\n"
      '    之后。\n\n-   <a id="beta-three"></a>**第二款** 下一款。\n',
      outputs["zh"],
    )

  def test_active_title_names_the_edition(self):
    self.activate()
    errors, _, outputs = bc.build(self.root)
    self.assertEqual(errors, [])
    self.assertIn("\n# Test Constitution (2026-10 edition)\n", outputs["en"])
    self.assertIn("\n# 测试宪法 (2026-10 版)\n", outputs["zh"])
    self.assertIn(" Revision: 1. ", outputs["en.agent"])
    self.assertIn("修订号：1。", outputs["zh"])

  def test_draft_title_names_its_edition(self):
    """A draft names the edition it prepares and marks its revision draft."""
    errors, _, outputs = bc.build(self.root)
    self.assertEqual(errors, [])
    for name, title, revision in [
      ("en", "Test Constitution (2026-10 edition)", " Revision: 0.2 (draft). "),
      ("en.agent", "Test Constitution (2026-10 edition)", " 0.2 (draft). "),
      ("zh", "测试宪法 (2026-10 版)", "修订号：0.2 (草案)。"),
      ("zh.agent", "测试宪法 (2026-10 版)", "修订号：0.2 (草案)。"),
    ]:
      self.assertIn(f"\n# {title}\n", outputs[name])
      self.assertIn(revision, outputs[name])

  def test_agent_front_follows_the_status(self):
    """The version for agents opens with the build's note on the status.

    A draft is not in force; once active, every agent must follow it.
    """
    draft = bc.build(self.root)[2]
    self.activate()
    active = bc.build(self.root)[2]
    cases = [
      ("en", "a draft not yet in force", "every agent must follow it"),
      ("zh", "本宪法现为草案，尚未生效", "每个 agent 都必须遵守它"),
    ]
    for lang, in_draft, in_force in cases:
      with self.subTest(lang=lang):
        self.assertIn(in_draft, draft[lang + bc.AGENT])
        self.assertNotIn(in_force, draft[lang + bc.AGENT])
        self.assertIn(in_force, active[lang + bc.AGENT])
        self.assertNotIn(in_draft, active[lang + bc.AGENT])
        self.assertNotIn(
          "preamble cites", draft[lang + bc.AGENT] + active[lang + bc.AGENT]
        )
        self.assertNotIn(
          "序言引用", draft[lang + bc.AGENT] + active[lang + bc.AGENT]
        )

  def test_adjacent_section_citations_render_short(self):
    self.write(
      f"{B}/02-beta-three.toml",
      'id = "beta-three"\nstatus = "draft"\n\n'
      '[content]\nen = "Three."\nzh = "三。"\n',
    )
    cites = [
      ("{@beta-one} and {@beta-two}", "{@beta-one}和{@beta-two}"),
      ("{@beta-one} to {@beta-three}", "{@beta-one}至{@beta-three}"),
      (
        "{@beta-one}, {@beta-two} or {@beta-three}",
        "{@beta-one}、{@beta-two}或{@beta-three}",
      ),
      ("{@alpha-one} and {@beta-one}", "{@alpha-one}和{@beta-one}"),
      ("{@beta-one} and then {@beta-two}", "{@beta-one}和后来的{@beta-two}"),
      ("{@beta} and {@beta-one}", "{@beta}和{@beta-one}"),
      ("{@beta-one} and {@beta-two.x}", "{@beta-one}和{@beta-two.x}"),
      ("{@alpha-one.money} and {@beta-one}", "{@alpha-one.money}和{@beta-one}"),
    ]
    self.edit(
      f"{B}/01a-beta-two.toml",
      'status = "draft"',
      'status = "draft"\nitems = [{ id = "x",'
      ' number = "1", status = "draft", content = { en = "X.", zh = "X。" } }]',
    )
    en = "\n".join(f"Cite {e}." for e, _ in cites)
    zh = "\n".join(f"引用{z}。" for _, z in cites)
    self.edit(f"{A}/01-alpha-one.toml", "One, see {@beta-one}.", en)
    self.edit(f"{A}/01-alpha-one.toml", "一，见{@beta-one}。", zh)
    errors, _, outputs = bc.build(self.root)
    self.assertEqual(errors, [])
    self.assertIn(
      "Cite [Art. II §1][beta-one] and [§1a][beta-two].\n", outputs["en"]
    )
    self.assertIn(
      "引用[第二条第一款][beta-one]和[第一款之一][beta-two]。\n", outputs["zh"]
    )
    outputs = {lang: plain(text) for lang, text in outputs.items()}
    self.assertIn(
      "-   §1 Cite Art. II §1 and §1a.\n"
      "    Cite Art. II §1 to §2.\n"
      "    Cite Art. II §1, §1a or §2.\n"
      "    Cite Art. I §1 and Art. II §1.\n"
      "    Cite Art. II §1 and then Art. II §1a.\n"
      "    Cite Art. II and Art. II §1.\n"
      "    Cite Art. II §1 and Art. II §1a(1).\n"
      "    Cite Art. I §1(1) and Art. II §1.\n",
      outputs["en"],
    )
    self.assertIn(
      "-   第一款 引用第二条第一款和第一款之一。\n"
      "    引用第二条第一款至第二款。\n"
      "    引用第二条第一款、第一款之一或第二款。\n"
      "    引用第一条第一款和第二条第一款。\n"
      "    引用第二条第一款和后来的第二条第一款之一。\n"
      "    引用第二条和第二条第一款。\n"
      "    引用第二条第一款和第二条第一款之一第一项。\n"
      "    引用第一条第一款第一项和第二条第一款。\n",
      outputs["zh"],
    )

  def test_adjacent_item_citations_render_short(self):
    three = (
      '[[items]]\nid = "people"\nnumber = "3"\nstatus = '
      '"draft"\n\n[items.content]\nen = "People."\nzh = "人。"\n'
    )
    self.write(f"{A}/01-alpha-one.toml", ALPHA_ONE + "\n" + three)
    self.write(
      f"{B}/01-beta-one.toml",
      BETA_ONE.replace(
        "[content]",
        'items = [{ id = "one", number = "1",'
        ' status = "draft", content = { en = "One.", zh = "一。" } '
        "}]\n\n[content]",
      ),
    )
    cites = [
      (
        "{@alpha-one.money} or {@alpha-one.machines}",
        "{@alpha-one.money}或{@alpha-one.machines}",
      ),
      (
        "{@alpha-one.money} to {@alpha-one.people}",
        "{@alpha-one.money}至{@alpha-one.people}",
      ),
      (
        "{@alpha-one.money}, {@alpha-one.machines} and {@alpha-one.people}",
        "{@alpha-one.money}、{@alpha-one.machines}和{@alpha-one.people}",
      ),
      (
        "{@alpha-one.money} or {@beta-one.one}",
        "{@alpha-one.money}或{@beta-one.one}",
      ),
      (
        "{@alpha-one.money} or else {@alpha-one.machines}",
        "{@alpha-one.money}或者{@alpha-one.machines}",
      ),
      (
        "{@alpha-one} or {@alpha-one.money}",
        "{@alpha-one}或{@alpha-one.money}",
      ),
      (
        "{@alpha-one.money} or {@alpha-one}",
        "{@alpha-one.money}或{@alpha-one}",
      ),
    ]
    en = "\n".join(f"Cite {e}." for e, _ in cites)
    zh = "\n".join(f"引用{z}。" for _, z in cites)
    self.edit(
      f"{B}/01-beta-one.toml", "Four, see {@alpha-one.money} and {@alpha}.", en
    )
    self.edit(
      f"{B}/01-beta-one.toml", "四，见{@alpha-one.money}和{@alpha}。", zh
    )
    errors, _, outputs = bc.build(self.root)
    self.assertEqual(errors, [])
    self.assertIn(
      "Cite [Art. I §1(1)][alpha-one.money] or [(2)][alpha-one.machines].\n",
      outputs["en"],
    )
    self.assertIn(
      "引用[第一条第一款第一项][alpha-one.money]或[第二项][alpha-one.machines]。\n",
      outputs["zh"],
    )
    outputs = {lang: plain(text) for lang, text in outputs.items()}
    self.assertIn(
      "-   §1 Cite Art. I §1(1) or (2).\n"
      "    Cite Art. I §1(1) to (3).\n"
      "    Cite Art. I §1(1), (2) and (3).\n"
      "    Cite Art. I §1(1) or Art. II §1(1).\n"
      "    Cite Art. I §1(1) or else Art. I §1(2).\n"
      "    Cite Art. I §1 or Art. I §1(1).\n"
      "    Cite Art. I §1(1) or Art. I §1.\n",
      outputs["en"],
    )
    self.assertIn(
      "-   第一款 引用第一条第一款第一项或第二项。\n"
      "    引用第一条第一款第一项至第三项。\n"
      "    引用第一条第一款第一项、第二项和第三项。\n"
      "    引用第一条第一款第一项或第二条第一款第一项。\n"
      "    引用第一条第一款第一项或者第一条第一款第二项。\n"
      "    引用第一条第一款或第一条第一款第一项。\n"
      "    引用第一条第一款第一项或第一条第一款。\n",
      outputs["zh"],
    )

  def test_numerals(self):
    self.assertEqual(
      [cl.roman(n) for n in (1, 4, 9, 14, 40)], ["I", "IV", "IX", "XIV", "XL"]
    )
    self.assertEqual(
      [cl.zh_number(n) for n in (1, 10, 11, 20, 23)],
      ["一", "十", "十一", "二十", "二十三"],
    )
    en, zh = cl.LANGUAGES["en"], cl.LANGUAGES["zh"]
    self.assertEqual(en.section_number(12, ""), "§12")
    self.assertEqual(zh.section_number(12, "a"), "第十二款之一")
    self.assertEqual(en.item_ref(3, "b"), "(3b)")
    self.assertEqual(zh.item_ref(3, "b"), "第三项之二")
    self.assertEqual(zh.item_marker(12, ""), "(**十二**) ")


if __name__ == "__main__":
  unittest.main()
