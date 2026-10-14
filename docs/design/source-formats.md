# Source formats

All sources are TOML, read with the standard library's `tomllib`. Every
text a reader sees is a table whose keys are the language codes listed in
the constitution's `index.toml` and whose values are the text in that language,
so one file holds all languages of one provision side by side. Multi-line
text is a literal string (`'''...'''`); the build drops the newline after
the opening quotes and before the closing ones.

## IDs

Every provision of the constitution (article, section, list item) and
every rule has an ID: one or more lowercase words joined by hyphens,
`^[a-z0-9]+(-[a-z0-9]+)*$`, for example `leaks` or `credential-storage`.
The ID is a key, like a BibTeX key, chosen by a person once and never
changed; it says what the provision is about, not where it stands.

There are two namespaces. In the constitution, article and section IDs
are unique across the whole constitution, and a list item's ID only
within its section, so items can have short names (`capital`). In the
rules, a rule ID is unique among the rules, the private overlay's rules
included. The same word may be used once in each namespace. An ID is
never reused. From the first approved revision on, a provision of the
constitution is only ever repealed, never deleted, so the sources hold
every ID used since then and the build rejects a second one. While the
constitution was a draft, revisions deleted provisions outright; the
build does not see those IDs, so they are listed here, and a draft
revision that deletes another adds it:

- revision 0.5: the article `interpretation`;
- revision 0.7: the article `roles` and its sections `relay-role`,
  `job-role` and `worker-role`; the items `terms.deployment`, `terms.job`,
  `terms.layer`, `terms.project` and `terms.skill`;
- revision 0.9: the section `third-party-personal-info`;
- revision 0.11: the section `asking-once`;
- revision 0.14: the items `terms.authoritative-interpretation`,
  `terms.edition`, `terms.hard-rule` and `terms.undetermined`;
- revision 0.16: the items `must-ask.downtime`, `must-ask.machines`,
  `must-ask.money`, `must-ask.releases` and `must-ask.risk-controls`;
- revision 0.18: the item `must-ask.productive-resources`, renamed
  `must-ask.means-of-production`;
- revision 0.24: the article `protection-scope`, whose sections
  `protected-text` and `ordinary-tooling` moved into
  `amendment-procedure`;
- revision 0.28: the section `response-priority` and its items
  `response-priority.safety-incidents` (its text is now the section
  `safety-incidents`), `response-priority.productivity-gains` and
  `response-priority.minor-improvements`;
- revision 0.30: the section `checks-everywhere`, replaced by
  `decidable-provisions`;
- revision 0.33: the section `rules-refine`, whose text is now the item
  `division-of-instructions.rules`;
- revision 0.35: the section `role-definitions`;
- revision 0.36: the section `compliance`.

The date-prefixed IDs of the earlier flat sources (`20261012-...`) are
retired as well. A rule may be deleted, and its ID then goes into
`rules/retired-ids.txt` (see [Rules](rules.md#rules)).

Text cites with `{@...}`; nothing else in the text is markup, and any
other braces are output as is. The constitution cites only itself,
without a namespace: an article or section by its ID (`{@safety}`,
`{@leaks}`), a list item by its section's ID, a dot and the item's ID
(`{@must-ask.capital}`), always this full path, even inside the same
section. A rule always names the namespace: `{@constitution:leaks}`,
`{@constitution:must-ask.capital}`, `{@rule:credential-storage}`; the colon
says which body of text, the dot where in it. The build renders a
citation of the constitution as the number the provision has in that
language (see [Numbering](constitution.md#numbering)); the generated text shows numbers only, never IDs. A
long-lived record outside the sources (a case, a pull request, a report)
may cite a provision by ID or by number; a citation by number names the
edition, within which a number never changes (see [Numbering](constitution.md#numbering)). An ID
carries over from one edition to the next for every provision whose
content carries over.
