# statute

`statute` will compile a constitution and the rules a repository chooses
into the instruction files coding agents read. It is under development:
so far it reads a rule set, `statute.toml`, and one source of rules on
disk, and writes the lock file, `statute.lock`, `AGENTS.md` and, next
to it, the skills, checks and hook configuration the rules need; what it
does is listed in
[The tool and the first version](https://github.com/Luolc/agent-constitution/blob/main/docs/design/rule-system.md#the-tool-and-the-first-version). The release on crates.io, 0.0.1,
is a placeholder that only prints a notice. The project lives at
<https://agent-constitution.luolc.com/>, and its source at
<https://github.com/Luolc/agent-constitution>.
