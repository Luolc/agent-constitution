# Skills

`skills/<name>/SKILL.md` in the usual skill layout. Skills are not compiled
into the instruction file; they are installed as they are where the
harnesses look for skills. How they are installed is not decided yet (see
the [opening](README.md) of this design); the overlay's apply step under [Compilation](compilation.md)
is the current plan. A skill says how to do something; what must be
done stays in a rule, and the skill cites the rule ID. The instruction-only
check counts everything under `skills/` as instruction files, since a
skill directs the agents that read it.
