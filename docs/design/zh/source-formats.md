<!-- en-sha256: b9b33ce2e5abfa775373f91a47b2a621feb92f3f82f89471ea5c4874efd2dc16 -->
# 源文件格式

所有源文件都是 TOML，用标准库的 `tomllib` 读取。读者看到的每一段文字都是一张表：键是宪法 `index.toml` 里列出的语言代码，值是该语言的文字，所以一个文件里并排放着一条条文的所有语言。规定的文字只需要原文那一种语言，译文靠哈希跟踪 ([规定文件格式](rule-format.md#rule-file-format))。多行文字用字面量字符串 (`'''...'''`)；构建时去掉开头引号之后、结尾引号之前的换行。

## <a id="ids"></a>ID

宪法的每条条文 (条、款、项) 都有一个 ID：一个或多个小写单词，以连字符相连，`^[a-z0-9]+(-[a-z0-9]+)*$`，例如 `leaks` 或 `must-ask`。ID 是一个键，就像 BibTeX 的键，由人一次选定、永不更改；它说明这条条文讲什么，而不是它排在哪里。

ID 有两个命名空间。在宪法里，条和款的 ID 在整部宪法内唯一，项的 ID 只在所在的款内唯一，所以项可以起很短的名字 (`capital`)。在规定里，ID 是一个数字，指本规定源 parent 树上的一个节点：一条规定、一个 one-of 或一个领域 ([ID 与修订号](rule-format.md#ids-and-revisions))。ID 永不重复使用。自宪法的文本第一次获批准起，宪法的条文只会被废止，不会被删除，所以源文件里保留着此后用过的每一个 ID，构建会拒绝重复的 ID。宪法还是草案时，修订直接删除过条文；构建看不到这些 ID，所以在这里列出，以后哪一次草案修订再删除条文，也把它加进来：

- 修订号 0.5：条 `interpretation`；
- 修订号 0.7：条 `roles` 及其款 `relay-role`、`job-role` 和 `worker-role`；项 `terms.deployment`、`terms.job`、`terms.layer`、`terms.project` 和 `terms.skill`；
- 修订号 0.9：款 `third-party-personal-info`；
- 修订号 0.11：款 `asking-once`；
- 修订号 0.14：项 `terms.authoritative-interpretation`、`terms.edition`、`terms.hard-rule` 和 `terms.undetermined`；
- 修订号 0.16：项 `must-ask.downtime`、`must-ask.machines`、`must-ask.money`、`must-ask.releases` 和 `must-ask.risk-controls`；
- 修订号 0.18：项 `must-ask.productive-resources`，改名为 `must-ask.means-of-production`；
- 修订号 0.24：条 `protection-scope`，它的款 `protected-text` 和 `ordinary-tooling` 移入 `amendment-procedure`；
- 修订号 0.28：款 `response-priority` 及其项 `response-priority.safety-incidents` (其文字现为款 `safety-incidents`)、`response-priority.productivity-gains` 和 `response-priority.minor-improvements`；
- 修订号 0.30：款 `checks-everywhere`，由 `decidable-provisions` 取代；
- 修订号 0.33：款 `rules-refine`，其文字现为项 `division-of-instructions.rules`；
- 修订号 0.35：款 `role-definitions`；
- 修订号 0.36：款 `compliance`。

早先扁平源文件中以日期开头的 ID (`20261012-...`) 也已作废。规定从不删除，而是被取代或废止 ([ID 与修订号](rule-format.md#ids-and-revisions))。

文字里用 `{@...}` 引用；除此之外文字中没有任何标记，其他花括号原样输出。宪法只引用自己，不写命名空间：条或款写它的 ID (`{@safety}`、`{@leaks}`)，项写所在款的 ID、一个点和项的 ID (`{@must-ask.capital}`)，始终写这个完整路径，即使在同一款内也一样。规定总要写明命名空间：`{@constitution:leaks}`、`{@constitution:must-ask.capital}`、`{@rule:3}`；冒号说明是哪一部文本，点说明在其中的哪个位置。构建把对宪法的引用渲染成该条文在这种语言里的编号 (见[编号](constitution.md#numbering))；生成的文本只显示编号，从不显示 ID。源文件以外长期保存的记录 (案例、pull request、报告) 可以按 ID 或按编号引用条文；按编号引用要写明是哪一版，同一版内编号永不改变 (见[编号](constitution.md#numbering))。内容延续到下一版的条文，ID 也一并延续。
