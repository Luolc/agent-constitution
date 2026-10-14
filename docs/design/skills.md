# Skills

`skills/<name>/SKILL.md` in the usual skill layout. Skills are not compiled
into the instruction file; they are installed as they are where the
harnesses look for skills. In a repository, `statute compile` installs
the skills the enabled rules need, and a skill declares in its front
matter which rules it implements
([Rule system](rule-system.md#rules-and-skills)); where and how they
are written is in
[Compilation and the site](rule-system.md#compilation-and-the-site).
How skills are installed at the user level is not decided yet (see
the [opening](README.md) of this design); the overlay's apply step under [Compilation](compilation.md)
is the current plan. A skill says how to do something; what must be
done stays in a rule, and the skill cites the rule ID. The instruction-only
check counts everything under `skills/` as instruction files, since a
skill directs the agents that read it.
