<!-- en-sha256: 9ff7937dfc652c5c48cf27bf4e850b23d6d12fdf828c3cdfa0f3a5db371f6d63 -->
# Skill

`skills/<name>/SKILL.md`，采用通常的 skill 目录结构。skill 不编译进指令文件；它们原样安装到各 harness 查找 skill 的位置。怎样安装还没有定 (见本设计的[开篇](README.md))；目前的计划是 overlay 的 apply 步骤，见[编译](compilation.md)。skill 说明怎样做某件事；必须做什么仍写在规定里，skill 引用规定的 ID。指令文件单独变更的检查把 `skills/` 下的一切都算作指令文件，因为 skill 是给读它的 agent 的指令。
