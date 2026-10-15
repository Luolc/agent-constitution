# statute

`statute` is a command that will compile a constitution and the rules a
repository chooses into the instruction files coding agents read. It is
under development: so far it reads a rule set, `statute.toml`, and one
source of rules on disk, and writes the lock file, `statute.lock`,
`AGENTS.md` and, next to it, the skills and checks the rules need; what
it does is listed in
[The command and the first version](https://github.com/Luolc/agent-constitution/blob/main/docs/design/statute-command.md#the-command-and-the-first-version). On crates.io, 0.0.1 is
a placeholder that held the name and only prints a notice; the command
starts with 0.1.0. The project lives at
<https://agent-constitution.luolc.com/>, and its source at
<https://github.com/Luolc/agent-constitution>.
