<!-- en-sha256: 789ebb5fa460bb0f3276fc0fa91873c0a2153703e6ad67c38ac4fc8d4575de4b -->
# 规定检查与审查

这是[规定体系](rule-system.md) 的一部分，那里的状态和术语也适用于这里。

## <a id="checks"></a>检查

大多数规定没法用脚本判定：脚本能判定的规定，通常早已是 CI 检查或 lint。所以规定主要靠审查来落实，检查为审查服务：它判定脚本能判定的那少数规定，其余的交给审查方。

编写检查不用任何专门的语言，声明式的也不用。一小组内置的检查原语 (「这个文件存在」「这个 TOML 文件里这个键是这个值」) 会不断需要再加一种情况、再加「或」、再加「除非」，直到它自己成了一门没人读得顺的语言。检查就是代码，分两种，没有第三种。

**我们的规定：Rust。** 本仓规定的检查放在命令的 crate 里 ([命令与第一版](statute-command.md#the-command-and-the-first-version))，每条规定一个模块，以规定的 ID 命名 (规定 42 的模块是 `r42.rs`)，就像 ruff 和 clippy 把每条 lint 实现为一个函数。规定的正文和元数据留在 `rules/42.toml`，写规定的人和写检查的人各改各的文件；模块的测试用例跟着检查放在 crate 里。常用的操作，例如从 TOML 文件读一个键，是同一个 crate 里普通的辅助函数；一个操作第三次重复时才抽成辅助函数，不提前 (`simplicity`)。这个 crate 构建成一个二进制文件，所以下游用户装一个二进制文件就有了全部检查，不用部署任何脚本。这个 crate 发布到 crates.io，有自己的版本，发布它本身就是一次发版；规定随仓库的 tag 发版 ([发布与版本](amendments-and-releases.md#releases-and-versions))。

需要读 GitHub 状态 (pull request 或 CI run) 的检查，在仓库里运行机器上的 `gh` 去读，`gh api` 从仓库的 GitHub remote 找到仓库。statute 不读、不存、也不打印任何 token，凭证留在 `gh` 自己的配置里。`PATH` 上没有 `gh`、没有登录、仓库不在 GitHub 上、调用失败，以及调用 60 秒内没有返回 (按下文超时的命令那样停掉)，都和缺工具一样算无法判定。这类检查看的对象从仓库本身得出：pull request 是 GitHub 与 `HEAD` 关联的那些 (把它合入默认分支的那一个；没有的话，是包含它的那些未关闭和已合并的 pull request)，发布是 `HEAD` 上的 tag。还没推送的 commit 没有 pull request，没有 tag 的 `HEAD` 没有发布，两者都算无法判定。

一条规定被取代时，它的后继规定得到一个新模块，从旧模块复制过来，按需修改。旧模块留在代码里，标为被取代，除非专门要求，否则不运行。

**下游编写的规定：外部命令。** 存放在别的规定源里的规定，把它的检查声明为一条命令 (`check = "command"`，另写 `run` 和 `tools`)，用什么语言都可以。脚本放在那个规定源自己的仓库里，用它自己的工具链运行；规定写明它需要的工具 (例如 uv 或 Go)。约定如下：

- 退出码 0：满足规定；1：违反规定；其他任何值：检查无法判定；
- 标准输出：JSON，为每一处发现给出位置和原因，写成对象 `{"location": ..., "reason": ...}` 的列表。

命令在规定清单所在的目录里用 `sh -c` 运行；检查一个变更时，`STATUTE_BASE` 里是变更起点的 git ref。`tools` 里的工具不在 `PATH` 上、命令 10 分钟后还没退出或还没关闭输出 (它启动的进程一并停掉)、输出超过 1 MiB、退出码为 1 但输出不是上述列表，都算无法判定。

允许规定没有自动检查 (`check = "none"`)；这样的规定在 `uses` 含 `review` 时交给审查方 ([规定用在哪里](rule-config.md#where-a-rule-is-used))。

## <a id="outcomes-and-the-review-loop"></a>结果与审查回路

检查有三种结果：通过、违反和无法判定。缺工具、超时和脚本崩溃都算无法判定，从不算违反，所以缺了某个工具链不会让检查变红。

审查从 `statute check` 开始：审查方运行它，拿到一份已启用规定的清单，脚本判定了的少数规定带着结果，其余的待审，每条都附上理由和核验方法，这两部分编译出的 `AGENTS.md` 默认不收 ([编译与网站](statute-command.md#compilation-and-the-site))。审查方按 domain、按规定逐项过这份清单：待审的规定、无法判定的结果和违反的结果。是由一个审查方全部接手，还是由几个审查方各接一组，以及怎样分组，在规定清单里设定。`uses` 把待审的规定缩小到标了 `review` 的那些 ([规定用在哪里](rule-config.md#where-a-rule-is-used))。

第一版里，`statute check` 列出仓库里 `uses` 含 `review` 的已启用规定供审查，有检查的就运行它的检查；带 `--base <ref>` 时，检查变更 `git diff <ref>...HEAD`。它把规定按顶层 domain 分组，子领域的规定归在它所属的 domain 下，domain 和每组里的规定都按 ID 排序，每组以 domain 的引用和标题作标题 (`## R13 (Security)`)。每条规定一行，写明结果 (`pass`、`violation`、`inconclusive`，没有自动检查、`uses` 含 `review` 的规定写 `review`)、引用和标题，以及一句说明，发现列在下面；无法判定或待审的规定后面紧跟它的理由和核验方法，其中的引用与编译出的 `AGENTS.md` 一样渲染，只是被引的规定写成标题加引用 (`标题 (规定 R42)`)，因为审查方是离开那个文件读它的。在各领域之前，`## statute.toml` 一节把规定清单的 `public` 与仓库在 GitHub 上的可见性比对 (见[公开与否](rule-sources.md#public-or-not))，经 `gh repo view --json visibility` 查询：仓库在 GitHub 上显示为 `PUBLIC` 而清单写着 `public = false`，即为违反；`gh` 给不出答案，算无法判定；其余情况都通过。有规定或规定清单被违反时退出码为 1，否则为 0。内置检查只覆盖规定的一部分时，什么也没发现也算无法判定，不算通过，因为规定的其余部分仍要审查方看。内置检查属于第一方规定源的规定；别的规定源里写了 `check = "builtin"` 的规定算无法判定，并附一句说明 statute 没有它的内置检查，所以本地的 `L50` 不会运行规定 50 的检查。各条内置检查如下：

- 规定 46 (instruction-files-apart) 用 `check-instruction-changes` 的路径表区分指令文件、生成文件和其他文件，变更里既有指令文件又有其他文件的即为违反。statute 自己的文件按规定清单所在的目录认，不论清单在仓库的哪一层：`statute.toml`、`statute.lock` 和 `.statute/` 下的所有文件都是指令文件。锁文件算指令文件，是因为它钉住每个 git 规定源的 commit，`statute upgrade` 只改写锁文件就能把规定源升到更新的规定，而 `statute compile --check` 照样通过。statute 在规定清单旁编出的 `AGENTS.md`，只在等于 statute 现在从公开层编出的结果时算生成文件，这正是规定 47 对生成文件的要求；锁文件只在 statute 编译得了 (这要求它等于 `statute lock` 此刻会写出的内容)，并且它的 TOML 与 base (取 merge base) 相比只在 `statute` 和 `commands` 上不同时，才算生成文件，这两项是 statute 按自己的版本和规定的检查写的；手改过、没有重编、改了这两项以外的内容，或者 statute 编译不了时，都算指令文件，所以手改的文件与其他文件混在一起即为违反，不论仓库的 CI 跑不跑 `statute compile --check`。0.1.18 之前的 statute 在规定清单旁写的 `statute-checks.toml`，现在由 statute 删掉，算生成文件。只改了生成文件的变更 (例如为 statute 的新版本重编、重新锁定) 不算混合。说明里列出变更涉及的生成文件，供规定 47 的审查方核对。脚本在这里有两处不同：在本仓，它按本仓规定 L31 始终把仓库顶层的 `AGENTS.md` 算作生成文件，仓库顶层的 `statute.lock` 只要 TOML 只在这两项上不同也算，因为本仓的 CI 跑 `statute compile --check`；它也只认仓库顶层的 statute 文件。路径表没有列出的 agent 工具的文件、路径表以外的指令源文件，以及仓库在自己的 `AGENTS.md` 里声明的路径，交给审查方；没有 `--base` 就没有可检查的变更；
- 规定 280 (instruction-files-apart-but-migration) 取代规定 46，运行同一项检查，只多它唯一的例外：在规定清单旁新增 `statute.toml`、而 base 的任何目录里都没有 `statute.toml` 的变更是迁移变更，可以把指令文件和其他文件放在一起。这时结果是无法判定，其他文件逐个列出交给审查方，由审查方核对它们只是把 `statute compile --check` 加进仓库在 base 上已有的 commit hook 和 CI。其余变更都按规定 46 判定；
- 规定 43 (credential-designated-place) 带 `--redact` 运行机器上的 gitleaks，扫描变更里的提交或全部历史，有命中即为违反，命中只显示位置和 gitleaks 的规则名；gitleaks 识别不出的凭证，以及仓库以外的地方，交给审查方；没有 gitleaks 时算无法判定；
- 规定 50 (commit-hook-credential-scan) 读仓库顶层的 `.pre-commit-config.yaml` (不在 git 里时读规定清单所在目录的)，有 hook 在 commit 阶段对每次提交、不带文件过滤、带 `--staged` 和 `--redact` 运行 gitleaks，即为通过；gitleaks 自己发布的 hook，无论用它现在的还是以前的仓库名，`rev` 为 v8.1.1 或更新时按它自己的 entry 计。statute 看不到命令的 hook (`rev` 更旧或读不出的 gitleaks 自带 hook、别的仓库的 hook、运行脚本的本地 hook)、不带 `--redact` 或带文件过滤的 gitleaks hook、别的扫描器、别的 hook 框架的配置，以及 statute 读不了的 YAML，算无法判定。根本没有 hook 配置，或者配置里 commit 阶段的 hook 都看得到命令、却没有一个扫描暂存的改动，即为违反；
- 规定 58 (release-first-green-commit) 把 `HEAD` 上的 tag 当作发布，把默认分支当作主分支，把包含默认分支所要求检查 (分支保护或 ruleset 要求的) 的 workflow 当作 CI；这个 workflow 为 `HEAD` 推送到默认分支而跑的 run 在第 1 次 attempt 就以成功结束的，即为通过，否则即为违反。`HEAD` 上没有 tag、没有要求的检查、没有这样的 run (推送不触发 CI，或者 commit 不在该分支上)，以及 run 还没跑完时，算无法判定；
- 规定 61 (verdict-names-version) 读 `HEAD` 所属 pull request 上的 verdict：正文以 `Verdict:` 开头的评论或 review，以及批准或要求修改的 review。verdict 里没有其 pull request 中某个 commit 的完整 SHA，即为违反，review 还以 GitHub 为它记下的 commit 写明版本；在别处给出的 verdict，以及写明的 SHA 是不是审查过的版本，交给审查方；
- 规定 71 (no-automatic-merge) 读 `HEAD` 所属 pull request 的时间线，只要某个 pull request 开过 auto-merge 就是违反，之后关掉了或已经由它合并了也一样；其他自动合并的方式，以及是谁开的，交给审查方。

规定 43、46 和 280 取代的规定 3、7 和 46 保留各自的检查，只在规定清单仍然启用它们时运行。

`statute check` 由审查方运行，也可以在 CI 里运行。它不作为提交钩子：有的检查要扫描全部历史，或者经网络访问代码托管平台，可能很慢，而提交钩子只放几秒内跑完的检查。

对交给它的每条规定，审查方判断规定是满足还是违反。违反的要修正。审查方的判断与检查的结论不同时，就是检查错了：

- 误报：检查标记了一处其实满足规定的内容；
- 漏报：检查放过了一处违反规定的内容，是在审查别的结果或在后来的工作中发现的。

两种情况都会引出一项提案：修正这条规定的检查。修正检查不改变什么算合规，所以这条规定保留 ID，修订号加一 (`wrong-checkers.fixing`)。人和 agent 都可以依照 `rule-evolution` 提出修改规定。规定就是这样自我改进的：脚本能跑的少数检查先判定能判定的，其余的由 agent 逐条审查，agent 得出的结论再回到检查里。

**谁来判断。** 编译、升级、判断冲突和兼容范围，凡是脚本能判定的，都由确定性的脚本完成。脚本判断不了的，交给 agent，走上面的三种结果和审查回路。这些判断不需要人参与；只有宪法本来就要求人批准的地方，人才出现。这与 `automation-first.decide-and-do` 一致。

## <a id="trial-rules"></a>试行规定

agent 做某项工作时，发现没有任何规定、skill 或惯例覆盖它，就开始把惯例写下来。第一个版本是试行规定 (`status = "trial"`)：用在实际工作中，试行期间可以随意修改，稳定之后改为 `active`。本仓自己的规定、skill 和惯例，也打算按同样的方式产生。

**起步。** 宪法还是草案时，每条规定都是 `draft`。宪法现行之后，新规定从 `trial` 开始，宪法还是草案时写下的规定原地从 `draft` 转为 `trial` ([ID 与修订号](rule-format.md#ids-and-revisions))。

**状态的含义。** `trial` 与 `active` 的区别，就像 linter 里实验性检查与稳定检查的区别：`active` 的规定已经稳定，可以直接拿来用；`trial` 的规定还在实验，还可能改。`active` 说的是规定已经稳定，不是说有人审过它。领域和 one-of 的状态只说这个节点本身，即它的标题、applicability 和它放了哪些规定；领域可以是 `active`，而其中有的规定还是 `trial`。

**转正。** 一条试行规定什么时候已经稳定、可以改为 `active`，由委托人判断，条件以 R289 为准 (`rules/289.toml`)。多数规定没法用确定性的检查判定，所以规定主要用在审查里：审查方先跑现有的检查，读规定的验证办法，再按领域里的规定评判这次改动。试行规定在审查里表现如何，用过多少次、有没有被用错，是委托人判断的依据；使用次数本身不会让一条规定转正。转正只改状态，不改什么算合规，所以原地修改，修订号加一 ([ID 与修订号](rule-format.md#ids-and-revisions))。
