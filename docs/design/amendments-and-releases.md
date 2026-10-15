# Amendments and releases

## Approving an amendment

Built so that the person spends as little time as possible on it, which is
what `purpose` asks for. The person reads what the constitution
says and nothing
else: tools, checks and fixtures are ordinary changes (see [File structure](file-structure.md)).

While the constitution is a draft, an amendment pull request is merged
once agent review has passed, and the person reads it afterwards on a
preview of the site; the description is still written as below, so that
the reading is quick. Only a change to an approved edition waits for the
person's yes on a named head commit before it is merged. How the repo
keeps the edition in force and a draft of the next one is settled by
`text-and-sources` and `draft-and-edition`.

- An amendment pull request changes only `constitution/`: the sources and
  the generated files rebuilt from them.
- Its description opens with the diff of the generated text in the
  language the person reads, then the same diff in each other language,
  collapsed. The generated diff is what agents will read,
  numbers included; the person does not need to read the sources.
- Agent review is finished before the person is asked: CI's constitution
  check is green, and the reviewer has checked that the texts in all
  languages say the same thing. A person approving the change approves the
  texts in all languages on the strength of that review.
- The description ends with the open questions, each clear and easy to
  answer and with a recommended answer (`asking`).
- For an approved edition, the person's yes names the head commit it
  approves, and the merge lands exactly that commit (`independent-review`).
- `proposing-amendments` asks for the generated diff in the
  language the approving person reads, and `amendment-languages`
  for the texts in all languages to change together; the steps above are how this repo
  meets them. If the head changes afterwards, the person is
  asked again with only the diff since the approved head.

## Releases and versions

- Versions are tags `v<major>.<minor>.<patch>` on `main`. A constitution
  amendment or a rule change that alters behavior is at least a minor bump;
  a change to tools that alters output is at least a minor bump; the rest is
  a patch.
- A release tag goes only on a `main` commit whose push-triggered `check` run
  succeeded on its first attempt. A rerun that turned green does not
  qualify; tag a later commit that passed the first time.
- Pushing a release tag makes a version available. By default a release
  is an item of `must-ask`, added by rule 32 (`release-must-ask`), and needs a
  person's yes; a project may move it under
  `relaxations.project-overrides`.
- The crates under `crates/` are published to crates.io, each with its
  own version in its `Cargo.toml`; publishing one is a release too.
  `statute` 0.0.1 was a placeholder that held the name.
- A crate's version is independent of the repo's, so its tag carries the
  crate's name: pushing `statute-v<version>` releases `statute` at that
  version, under the same rules as a release tag above.
  `.github/workflows/release.yml` publishes it. The workflow holds no
  crates.io token: crates.io trusts this repo's `release.yml` running in
  the GitHub environment `release` (Trusted Publishing), and the job trades
  its OIDC token for a short-lived crates.io token. The environment admits
  only tags `statute-v*`.
- Before publishing, a job of its own, outside the environment and
  without the OIDC token, runs `check-release-tag`; the job that publishes
  runs only when it passed, so no tool of the check can get the token. The
  check takes three steps in order, each only once the one before it
  passed, and stops at the first that fails: the tag is `statute-v` followed by the version in
  `crates/statute/Cargo.toml`, and a manifest without a version to read
  fails; the tagged commit is on `main`; that commit's push-triggered run
  of `ci.yml` completed with `success` on its first attempt. So the tag
  goes on after that run has finished; when it was pushed earlier, the
  check fails, and the workflow is rerun once the CI run has passed.
- The overlay pins a tag and its commit SHA and refuses to build if they
  disagree. It moves to a new version by a change in the overlay, reviewed
  there.
