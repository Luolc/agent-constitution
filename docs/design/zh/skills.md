<!-- en-sha256: 930a6462d758782b2e2b59040d91f278ffe1dc8a6180b1189d133a97cd4007b6 -->
# Skill

`skills/<name>/SKILL.md`，采用通常的 skill 目录结构。skill 不编译进指令文件；它们原样安装到各 harness 查找 skill 的位置。在仓库里，`statute compile` 安装启用的规定需要的 skill，skill 在 front matter 里声明它落实哪些规定 ([规定体系](rule-system.md#rules-and-skills))；写到哪里、怎么写，见[编译与网站](rule-system.md#compilation-and-the-site)。用户层的 skill 怎样安装还没有定 (见本设计的[开篇](README.md))；目前的计划是 overlay 的 apply 步骤，见[编译](compilation.md)。skill 说明怎样做某件事；必须做什么仍写在规定里，skill 引用规定的 ID。指令文件单独变更的检查把 `skills/` 下的一切都算作指令文件，因为 skill 是给读它的 agent 的指令。
