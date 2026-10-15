<!-- en-sha256: c5c31bae1daad8eaf3ab406e731fb52de0a75a81997b1edf2c56991f80d5cf99 -->
# statute 命令

这是[规定体系](rule-system.md) 的一部分，那里的状态和术语也适用于这里。

## <a id="compilation-and-the-site"></a>编译与网站

**在本地**，编译器读取宪法 (agent 版) 和一份规定清单及其锁文件，写出：

- 一份 `AGENTS.md`，收入宪法的 agent 版，然后是启用的规定，排成一个平铺的列表，每条规定一项，只有它的引用和要求 (见下)。文件暂时不拆。等用它的某个仓库编译出的文件用 `count-tokens` 量过之后，再按量出的长度定一个阈值，超过阈值的文件沿领域拆开；
- 启用的规定需要的 skill 目录；
- 要安装的检查二进制文件的版本，以及启用规定的外部命令。

`AGENTS.md` 只收录 `uses` 含 `instructions` 的启用规定 ([规定用在哪里](rule-config.md#where-a-rule-is-used))；参数实现之后，每个占位符都换成它的取值 ([参数](rule-config.md#parameters))。

`statute compile` 把 `AGENTS.md` 写在 `statute.toml` 旁边，用 `--output` 可以写到别的文件，另外两样写在它旁边 (见下)；锁文件必须与 `statute lock` 此刻会写出的内容一致，否则不写。`AGENTS.md` 依次包含：一行说明文件是生成的注释，`compile` 靠它区分自己写的文件和手写的文件；规定清单所用语言的宪法 agent 版，即宪法构建写出的 `constitution/<edition>/generated/constitution.<lang>.agent.md`，原样收入；最后是标题「规定」及启用的规定。文件依据的宪法的版、修订号和规定源记在 `statute.lock` 里，不写进文件。

`AGENTS.md` 是给 agent 读的，私有全文 `.statute/AGENTS.full.md` 也是，两者构建方式相同，永远是一个平铺的列表。每条启用的规定是一项，写成 `- <引用> <要求>`，例如 `- R22 Keep a credential out of every message.`：先是 statute 显示的这条规定的引用 (`R42`、`L12`、`team:7`)，一个空格，然后是它的要求；要求有多行时，第一行之后的每行缩进两个空格，接着这一项写。别的一概不写：没有规定的标题、理由和核验方法，没有领域、子领域和 one-of，既不写它们的标题，也不写 applicability，也没有锚点和链接。节点之间的关系和设置，即 `requires`、`conflicts`、one-of、`overrides`、默认开关、`uses` 和 `parent`，都是元信息：statute 在锁定和编译时检查它们，规定清单编译通过之后，它们的作用就尽到了，不写进文件。标题、理由、核验方法和领域的 applicability 是给人看的，放在网站上 (见下)；审查方从 `statute check` 拿到规定的理由和核验方法。引用保留在每一项的开头，审查发现和引用都靠它指明是哪条规定。

各项按树的先序排，节点在它的子节点之前，相关的规定因此挨在一起：第一方规定源的领域排在最前，接着是其他规定源的领域，按规定源的名字排，最后是仓库自己的；同一个节点下面，先是第一方规定源的子节点，接着是其他规定源的，按名字排，最后是仓库自己的，同一个规定源的按 ID 排；领域下面先排它的规定和 one-of，再排它的子领域，各按这个顺序，这样领域自己的规定挨在一起。两个网站在领域下也这样排，每个领域归在一个标题下 ([本地网站](#the-local-site)、[静态网站](static-site.md))。one-of 给出它启用的选项里的规定，选项是领域时就是这个领域自己的规定；没有启用规定的子树不产生任何内容。覆盖别的规定的规定 ([覆盖](rule-sources.md#overrides)) 站在被覆盖的那条规定的位置上，被覆盖的规定不进文件。规定有该语言的译文就用译文，没有就用原文。要求里的引用会渲染出来 ([ID](source-formats.md#ids))：`{@rule:<id>}` 写成被引规定的引用 `R42`，不论它在不在文件里；被引的规定被覆盖时，写成站在它位置上的那条规定；`{@constitution:<id>}` 写成该条款在这种语言里的标签，与 agent 版一样是纯文本，例如 `第四条第一款第二项`。标签按这一版宪法的源算出：条和款的编号取自文件名，项的编号取自 `number`，算法与宪法构建相同 ([编译](compilation.md))；与宪法构建不同的是，连着的几处引用不缩写 (`第四条第一款、第四条第二款`)。引用了不存在的节点或条款时命令失败，并指出是哪条规定的哪一部分。规定文件和锁文件里的哈希都不变。

在 `AGENTS.md` 所在的目录里，`statute compile` 还会写：

- 启用的规定需要的每个 skill ([规定与 skill](rule-format.md#rules-and-skills))：把规定所在规定源里的 `skills/<name>/` (本仓库自己的规定取仓库顶层的 `skills/`；但若某个规定源就是这个目录，如 `R = { path = "." }`，这些 skill 归该规定源，本仓库自己的规定没有 skill) 整个复制到 `.agents/skills/<name>/` (需要的两个 skill 同名、来自两个规定源时，命令失败)，加一个文件 `.statute-generated`，再建一个指向这份副本的软链 `.claude/skills/<name>`，目标写成相对路径 `../../.agents/skills/<name>`。每个 skill 各有一个软链，不给整个目录建一个，也不只链 `SKILL.md`；Claude Code 只从 `.claude/skills/` 读仓库的 skill (2026-10-14 读取)。只有 Unix 系统上才建软链：在别的系统上，规定清单需要 skill 时命令失败。skill 里只能有目录和普通文件；
- `statute-checks.toml`：编译它的 statute 的版本，写成 `statute = "<version>"`，规定用到的内置检查就在这个二进制里；然后每条 `check = "command"` 的启用规定一张 `[[commands]]` 表，按 ID 排序，写明它的 `rule` (`"R42"`)、`run` 和 `tools`。这个文件随 statute 的版本变化。它放在仓库顶层，不放进 `.statute/`：它要提交，CI 要读，改动它时要和生成它的 `AGENTS.md` 放在一起审。

每个文件开头都有一行注释，说明是 statute 写的 (`AGENTS.md` 里是 `<!-- ... -->`，`statute-checks.toml` 里是 `# ...`)。命令动手写之前，先查一遍要写的每个位置；只要有一处放着不是它写的东西 (没有那行开头的文件、没有 `.statute-generated` 的 skill 目录、不是它所建软链的 `.claude/skills/<name>`)，就什么都不写，命令失败。要写 skill 时，`.agents/skills` 和 `.claude/skills` 必须是普通目录，或者不存在：仓库把整个 `.claude/skills` 链到 `../.agents/skills` 时命令失败，并指出这个软链，因为 statute 是每个 skill 各建一个软链。带 `.statute-generated`、但规定清单已经不需要的 skill 目录会被删掉，连同它的软链；`.agents/skills/` 和 `.claude/skills/` 里的其他东西都不动。同样的输入编译出逐字节相同的结果，软链也一样。这些文件不是一次性写完的：中途出错 (比如磁盘满了) 可能留下一部分产物，再运行一次命令就能补完。写入顺序保证了这一点：skill 的 `.statute-generated` 先于它的文件写，软链先于它的目录删，`AGENTS.md` 最后写。

`statute compile --check` 什么都不写。它只编译公开层，凡是 `statute compile` 会改动的地方都逐个列出并失败：内容不同或缺失的 `AGENTS.md`、`statute-checks.toml`，skill 副本里与源不同、缺失或源里没有的文件，不是 statute 写的副本或软链，以及带 `.statute-generated`、但规定清单已经不需要的 skill 目录。git 忽略的私有输出既不编译也不比较，所以私有层加载失败也不会让它失败。和 `compile` 一样，`statute.lock` 不是 `statute lock` 此刻会写出的内容时它也失败。本仓库把它作为提交钩子运行，CI 也跑这些钩子。

写出的 `AGENTS.md` 旁边 (不给 `--output` 时就是仓库顶层) 有 `CLAUDE.md` 时，不论是普通文件还是指向 `AGENTS.md` 的软链，每次 `statute compile` 都给出 warning，有没有私有层都一样。warning 问要不要删掉它，并用几句短话说明原因：没有 `CLAUDE.md` 时 Claude Code 自己会读 `AGENTS.md` ([规定清单与锁文件](rule-sets.md#rule-sets-and-the-lock-file)，monorepo 一段)，所以不需要 `CLAUDE.md` 指向它；有私有输出时，`CLAUDE.local.md` 用全文代替 `AGENTS.md` 给 Claude Code 读，Claude Code 还会再读 `CLAUDE.md`，它的内容就叠在规定之上，指向 `AGENTS.md` 的软链会让公开规定读两遍 ([公开仓库里的私有规定](rule-sources.md#private-rules-in-a-public-repository))，没有私有层时 warning 不说这一句；迁移时 ([statute-migrate](../../../skills/statute-migrate/SKILL.md)，第 e 步)，普通文件的 `CLAUDE.md` 改名为旧版副本，指向 `AGENTS.md` 的软链不改名，只问用户要不要删。statute 既不删它，也不改名。尚未实现：warning 仍说迁移时两种都会改名。

编译、检查和升级规定清单由同一个命令完成 ([命令与第一版](#the-command-and-the-first-version))。宪法也由这个命令构建，即 `statute constitution build`。

**公开网站**列出每条规定，让人们能从完整列表里挑选，列表的筛选默认不显示已被取代和已废止的规定；被取代的规定保留自己的页面，指向它的链接仍然有效。本仓的规定在网站上也显示成 `R42`。列表和规定详情页显示什么，见[静态网站](static-site.md)。

## <a id="the-command-and-the-first-version"></a>命令与第一版

**命令。** 规定体系的命令用 Rust 写 (见[为什么用 Rust](#why-rust))，在 `crates/statute` 这个 crate 里逐步实现 ([文件结构](file-structure.md))。第一版就是这一个 crate。它的名字是 `statute`：crates.io 上的 0.0.1 是占住这个名字的占位版本，这个命令从 0.1.0 起 ([发布与版本](amendments-and-releases.md#releases-and-versions))。规定清单是 `statute.toml`，锁文件是 `statute.lock`，命令有：

- `init`：写一份新的规定清单；加 `--global` 时写全局配置 ([全局配置与 git 规定源](global-configuration.md))；
- `source add`：在规定清单、它的私有层或全局配置里登记一个规定源 ([添加规定源](global-configuration.md#adding-a-source))；
- `add` 和 `remove`：在规定清单里启用一条规定或去掉它，并更新锁文件；加 `--private` 时改的是私有规定清单 (见[公开仓库里的私有规定](rule-sources.md#private-rules-in-a-public-repository))；
- `lock`：把规定清单解析成锁文件；
- `compile`：写出编译输出 ([编译与网站](#compilation-and-the-site))；加 `--private-only` 时只写私有输出，供新 worktree 里的 git 钩子用，不联网，也从不失败 ([公开仓库里的私有规定](rule-sources.md#private-rules-in-a-public-repository)，新的 worktree 一段)；加 `--check` 时什么都不写，提交的输出与重新编译的结果不同就失败；
- `check`：按 domain 列出启用的规定供审查，脚本能检查的附上结果 ([检查](rule-checks.md#checks))；
- `upgrade`：把规定源和宪法移到更新的版本 ([规定清单与锁文件](rule-sets.md#rule-sets-and-the-lock-file))；
- `explain`：说明一条规定为什么被启用、哪些规定依赖它；
- `site`：为规定清单能用到的规定生成一个本地网站 ([本地网站](#the-local-site))；
- `export`：把一个目录里的宪法和规定以 JSON 打印出来，供读取它们的程序使用 ([导出](#export))；
- `constitution build`：从源文件构建所在仓库的宪法，或者检查它 (见下文)；
- `validate <dir>`：单独检查一个规定目录，当作一个规定源，不需要规定清单 (见下文)。

**第一版**只做仓库级。用户级是第二步，它的指令文件在用户的家目录里，目前由私有 overlay 安装。本仓的规定已经迁到新格式，每条把原来的文字 ID 留作 `slug`。最先用新格式写的规定里，有关于怎样写设计文档的那几条，正好拿实际工作把整套流程试一遍。

**目前已实现**：`init`、`lock`、`explain`、`add`、`remove`、`upgrade`、`compile` 和 `check` (内置检查有规定 43、46、50、58、61 和 71，以及被 43 和 46 取代的 3 和 7) 八个命令，以及 `site`、`constitution build` 和 `validate`，支持磁盘上的规定源和 git 规定源，`[sources]` 写几个就读几个；还有 `export`。仓库自己的规定 `L12` 从 `.statute/rules/` 或 `local-rules` 指定的目录读 (见[本地规定的目录](rule-config.md#the-directory-of-local-rules))；那个目录是某个规定源的规定目录时，比如 `local-rules = "rules"` 加上 `R = { path = "." }`，这些规定归那个规定源，本地引用会报错并提示写成 `R12`。规定在 `requires`、`conflicts` 和 `{@rule:...}` 里引用别的规定源的规定时，写作 `R12` 或 `team:7`，用的是规定清单 `[sources]` 里的名字：规定源自己的 `[sources]` 表 ([规定源与引用](rule-sets.md#sources-and-references)) 还不读。[规定源](rule-sources.md) 里已实现的是 `public`，连同 `requires`、`conflicts`、`overrides`、`parent` 和 `{@rule:...}` 里哪些规定可以引用哪些，以及把 `public` 与 GitHub 核对的检查 ([检查](rule-checks.md#checks))。覆盖已经实现，连同 `scope` 和级别；子领域、规定或领域指向另一个规定源的 `parent`，以及放在 git 目录里的私有层连同它的锁文件、它的输出、`.statute/.gitignore` 和 `compile --private-only`，也都已实现。全局配置、`init --global`、git 规定源和 `source add` 已经实现 ([全局配置与 git 规定源](global-configuration.md))，只是 `upgrade` 还不显示一条规定改动的文字。[规定配置](rule-config.md) 里，规定文件的 `default`、`uses` 和 `check = "none"` 已经读取并检查，连同[规定不挂在规定下面](rule-config.md#no-rule-under-a-rule) 的树形，`compile` 写什么、`check` 列什么也按 `uses` 来；规定清单的默认开关也已实现，连同 `domains`、`exclude`、`local-rules` 和规定源条目的 `defaults`，以及 `add`、`remove`、`explain` (包括 `--defaults`) 和锁文件对它们的处理。`default` 在规定文件需要它的地方必填，`check = "review"` 报错。参数也已实现，取值写进锁文件、`upgrade`、`explain <name>` 和两份输出。两个网站的标记、公开网站的参数页、编译出的 `AGENTS.md` 里 one-of 的位置，以及网站上 one-of 的框，也都已经实现。继承规定清单和按路径分段还没有实现。

**`init`** 在当前目录写一份 `statute.toml`：宪法取自哪个规定源、哪一版，`public = true` 或 `public = false`，空的 `rules`，以及给了 `--source` 时的一个磁盘上的规定源，即第一方规定源 `R`。`public` 取 `--public` 或 `--private` 指定的值 (两者最多给一个)，没有指定时取 `gh` 显示的这个仓库的情况，见[公开与否](rule-sources.md#public-or-not)。`--source <dir>` 指定规定源的目录，相对于当前目录。不给它时，`init` 不写 `[sources]`，`R` 取全局配置里的或内置默认的那个 ([一个名字指哪个规定源](global-configuration.md#which-source-a-name-means))，读它所跟踪分支的最新提交。`--edition` 指定哪一版；不给时，`init` 取规定源 `constitution/` 下唯一一个含 `index.toml` 的目录，一个都没有或多于一个时报错，并列出找到的那些。写之前它先解析一遍这份规定清单，解析失败就什么都不写。它不写 `statute.lock`：没有启用任何规定，也就没有要锁定的东西；第一次 `add` 会写出锁文件，已有的 `statute.lock` 会被替换。已有 `statute.toml` 时原样保留，`init` 以退出码 1 退出。

**`upgrade`** 把解析结果与 `statute.lock` 比较，打印变了什么 ([规定清单与锁文件](rule-sets.md#rule-sets-and-the-lock-file))，没有变化就说锁文件已是最新，然后像 `lock` 一样写锁文件。没有 `statute.lock` 时没有可比的东西，以退出码 1 退出。加上 `--replace-superseded` 时，它先在 `rules` 和 `packs` 里把每条被取代的规定换成它的后继规定，顺着取代链找到最后一条，文件其余部分保持原样；后继规定已经列出的，只去掉旧的那一项。取代链走到一条有多个后继规定的规定时，这一项保持不动，把这几条后继规定列出来，由用户用 `add` 和 `remove` 自己选。已废止的规定留给用户自己去掉。改过的规定清单解析不通过时，两个文件都不改。

**`validate <dir>`** 把一个目录里的规定文件当作第一方规定源读，连同 `params.toml` 里的参数，做所有不需要规定清单的检查 ([检查程序](checkers.md#checkers))：每个节点和参数的格式、规定树、引用的目标、译文、引用和占位符，以及默认值。每条发现单独一行写到标准错误；有错误时以状态 1 退出，没有问题时向标准输出打一行，写明节点数。`<dir>` 旁边有 `constitution/` 时，它还读这部宪法，检查宪法对规定的要求：每个节点的 status 与 `index.toml` 的一致，每处 `{@constitution:...}` 引用和每个 `authority` 条目都成立，`authority` 同时写了某个条款和它的上级时给警告；宪法读不进来时只报一条错误，宪法自身的错误留给 `constitution build` 报。加 `--base <git-ref>` 时还做第 7 项检查里规定的那部分：比较编号时，base 里的每个节点都还在，旧格式的规定以 slug 为其名字的节点存在，已废止或已被取代的仍然如此；第 7 项跳过比较的情况下它也跳过，并打出同样的提示；`<dir>` 旁边没有 `constitution/` 时报错。这些发现写成 `rules/7.toml:12: error: <message>`。本仓把 `statute validate rules` 作为 commit 钩子运行，CI 里还加上 `--base` 运行。

**`constitution build`** 检查 `constitution/` 下唯一一版的源文件，为每种语言把完整版和 agent 版写到这一版的 `generated/` 里，文件第一行写明生成它的命令 `cargo run -p statute -- constitution build`；加 `--check` 时什么都不写，已提交的文件与重新构建的结果不同就失败；加 `--check --base <git-ref>` 时还要与那个提交比较编号和修订号；`--root <dir>` 让它在当前目录以外的检出上运行。它做的检查是[检查程序](checkers.md#checkers) 里第 1 至第 7 项中关于宪法的部分。它不读 `rules/`：规定对宪法的说法和 base 提交里已有的规定，归 `validate` 检查 (见上文)。本仓把 `constitution build --check` 作为 commit 钩子运行，CI 里还加上 `--base` 运行。

**切换。** 目前 agent 读的是私有 overlay 的用户级指令文件。这些文件将加一条：仓库里有 `statute.lock` 的，以这个仓库的编译输出为准，用户级指令文件不再适用。它们管着的所有仓库一次切换：等它们的每一部分都成了规定，并且[从手写的指南切换过来](rule-sources.md#cutover-from-the-hand-written-guide) 的几个条件都成立；不先切某一个主题。在那之前，编译输出照常构建、用来对比，但不代替它们给 agent 读。

## <a id="the-local-site"></a>本地网站

`statute site` 把规定清单能用到的规定生成一个网站，写到 `statute.toml` 旁边的 `.statute/site/`，在浏览器里直接从磁盘打开；它从不发布，也不是[公开网站](static-site.md)。它读 `statute.toml`，有私有规定清单时也读，两层都在内存里解析，每个 git 规定源读锁文件记录的那个提交 ([git 规定源](global-configuration.md#git-sources))；它不写锁文件。最后打印 `index.html` 的路径。

- **文件。** 纯 HTML，加一份内嵌的样式表，照[前端](frontend.md) 写，用的是公开网站那套 token，图标也内联在页面里；首页另外内嵌一段很短的脚本，没有它页面照样能用 ([前端](frontend.md#in-the-code))。不用 Hugo，不引入 statute 现有依赖之外的依赖。每次运行都重新生成：先删掉旧的 `.statute/site/`，但只在这个目录里有 statute 写的 `.gitignore` (见下) 时才删，否则什么都不写并报错。除了 `.statute/.gitignore`，`.statute/site/` 之外什么都不写。
- **让 git 忽略。** statute 写 `.statute/site/.gitignore`，内容是 `*`，这样 git 忽略那里的每个文件，包括它自己，它也标明这个目录归 statute。statute 还写 `.statute/.gitignore`，其中列出 `/site/`，每个往 `.statute/` 写生成文件的命令都这样做 (见[公开仓库里的私有规定](rule-sources.md#private-rules-in-a-public-repository))。在 git 仓库里，它先用 `git ls-files` 问 `.statute/site/`，git 跟踪着那里的某个文件时，什么都不写并报错，写明是哪个文件和解决办法 `git rm -r --cached .statute/site`。git 回答不了时 (起不来或出错)，也什么都不写，报错并带上 git 的错误信息。不在 git 仓库里时没有东西会被提交，也就不用问。
- **首页**按四类规定源 ([四类规定源](rule-sources.md#four-kinds-of-source)) 分节：全局公开、全局私有、本仓公开、本仓私有。每节列出它的规定源，每个规定源把它的规定排成一张规定表 ([前端](frontend.md#components))，不论启用与否，领域下先排规定和 one-of、再排子领域 (见[编译与网站](#compilation-and-the-site))：每条规定一行，有引用、标题、标记，以及这份规定清单是启用它、覆盖它还是没选它，上面是写出规定源和领域的分组行；one-of 是一个框，装着它的选项，默认的那个加了标记。全部规定都在这一页上。每条规定那一行里还有它的陈述，在标题下面，字小一号、颜色弱一档，用页面的语言；没有这种语言的用规定自己的语言，并标出没有译文；占位符的填法和节点页一样 (见下)。筛选旁边有一个开关，可以显示标题和陈述、只显示标题，或者只显示陈述 (陈述占标题的位置)，默认是第一种；它靠单选按钮和样式表实现。脚本把选择写进地址，`view=title` 或 `view=statement`，默认的不写，这样前进后退能回到它，并记下来留给下次打开的首页。显示哪种以地址为准，地址里没有 `view` 就显示默认的；只有首页在这样的地址上打开时，才显示记下的那种，并把它写进这个地址。筛选器每个字段一个复选框菜单：状态 (默认不选已被取代和已废止)、默认开关、用途、领域、规定源类别，以及这份规定清单是否启用这条规定。样式表让勾选立即生效，所以没有脚本筛选器也能用，只是这时领域的复选框只管直接在这个领域里的规定；脚本勾上或取消一个领域时连同它下面的领域一起改，把筛选写进地址，并加上「全选」和「不选」、按行内文字的搜索、「清除筛选」和显示的规定条数。上方的图例可以折叠，保持读者留下的样子。
- **规定源属于哪一类**看它的两个属性：仓库自己的规定 (`L`) 和 `scope = "local"` 的规定源是本仓的，其余都是全局的，包括全局配置里的；公开与否按规定源自己的声明，`L` 按规定清单的声明。在 `.` 的规定源，比如本仓的 `R = { path = "." }`，是第一方规定源，所以是全局公开的。`local-rules` 指向它的规定目录时，它的规定同时也是仓库自己的规定：它只在全局公开下列一次，本仓公开那一节写明这一点并链接过去；否则仓库自己的规定列在它所属类别的本仓那一节。尚未实现：不论 `local-rules` 指向哪里，网站仍把在 `.` 的规定源当作仓库自己的规定。每个节点在首页和它自己的页面上都标出所属规定源的类别：用页面的语言写出类别名，底色是四种颜色之一。节点自己的页面上，标记还写出规定源的名字，例如 `全局私有 · me`。
- **每个节点一页**，`<source>/<id>.html`：陈述、理由、验证方式和适用范围；`requires`、`conflicts`、`overrides` 和取代它的规定；上溯的父节点路径和子节点。另有上级不同的节点用同一个标题时，指向这个节点的链接加上它上级的标题，首页的领域菜单也一样 ([前端](frontend.md#components))。领域的页面就是它下面节点的列表。对规定的引用链接到那条规定的页面，对宪法的引用写成条款的标签，和编译出的 `AGENTS.md` 一样。状态按规定文件里的写法显示，比如 `draft` 或 `trial`。
- **两种语言。** 页面有英文和中文两份：`.statute/site/` 顶层用 `[compile] lang` 指定的语言，因为这个站点属于仓库，不随读的人变；另一种语言放在 `zh/` 下 (那种语言是中文时放在 `en/` 下)，下面的路径相同。每一页都链接到自己的另一种语言版本 ([前端](frontend.md#components) 的语言切换)。statute 加上的文字，包括节点和规定源的类别，用页面的语言。节点的标题和正文，规定文件里有页面语言的就用页面语言，没有的用节点自己的语言，并标上那种语言的 `lang`；这时节点页上有一个 tag，说明它没有译文。这样的正文引用宪法时用它自己语言的条款标签，引用的规定也用这种语言的标题。
- **[规定配置](rule-config.md) 的标记。** 每个领域和每条规定，在索引和自己的页面上，都写明默认开启还是默认关闭；one-of 的选项写明是不是 one-of 的默认选项。规定的页面写明它用在哪里：`AGENTS.md`、审查，或两者都有；索引有两个筛选，分别隐藏这两种规定。占位符显示这份规定清单给参数的取值 (这个人有自己的取值时用这个人自己的)，加上标记，鼠标悬停时显示参数名，并链接到它所在规定源的参数页 `<source>/parameters.html`。参数页列出每个参数的标题、名字、类型、范围和默认值，它的取值和取值从哪里来，以及用到它的启用规定。

## <a id="export"></a>导出

`statute export [<dir>]` 把一个目录 (默认是 `.`) 里的宪法和规定打印成一个 JSON 值，供读取它们的程序使用，本仓的网站就是这样用的 ([静态网站](static-site.md)，「规定」)。它不读规定清单：这个目录是一个规定源，里面有 `constitution/` 或 `rules/`，或者两者都有。

- `constitution`：没有 `constitution/` 时为 `null`；否则是它唯一那一版的 `index.toml` 里的 `edition`、`revision`、`status` 和 `languages` (一版都没有或不止一版时导出失败)，以及按正文顺序排列的 `provisions`：每个条、款、项给出引用它时写的 `id`、它的 `parent` (条为 `null`) 和按语言给出的 `labels`，写法与编译出的 `AGENTS.md` 相同。
- `nodes`：`rules/` 里的每个节点，按 ID 排序，给出它的 `file` 和它的 `fields`，即文件里原样写的各字段，正文也在内。程序拿到的是文件里原样写的值；statute 以后认识的新字段，不改导出也能传过去；默认值由程序自己补。只有一个值是 statute 自己算出来的，免得每个程序再算一遍：`on-by-default`，即规定源的默认值是否启用这个节点 ([默认开关](rule-config.md#default-switches))，已被取代或已废止的规定总是否；取决于别的规定源里的节点时为 `null`。
- `params`：`rules/params.toml` 里的参数 ([参数](rule-config.md#parameters))，按文件里的顺序，每个给出 `name`、`type`、`values` (`choice` 的可选值，其他类型为 `null`)、`scope`、`default`、各语言的 `title`，以及 `shown`：输出里写出的默认值，按该规定源的标题用到的每种语言各给一份 (语言写成它的名称，「Chinese」「中文」)；没有这个文件时为空。

打印之前，statute 像检查任何规定源一样检查这些规定 ([依赖、one-of 与规定包](rule-sets.md#dependencies-one-ofs-and-packs))；检查不通过时，它只打印错误，以退出码 1 退出。依据旧原文翻译的译文过不了这项检查，所以节点列出的每份译文都跟随当前原文。JSON 是唯一的格式，所以这个命令没有选择格式的选项。

## <a id="why-rust"></a>为什么用 Rust

速度不是主要原因。几百次文件检查和正则表达式匹配，用 Python 也只要一两秒；真正花时间和钱的是审查方调用模型。选 Rust 的原因是：

- 检查作为一个二进制文件发布，不需要 Python 或别的运行时，所以用任何语言的下游仓库都能用；
- 并行运行检查更容易写对；
- [limae](https://github.com/Luolc/limae) 是一个以 Python wheel 发布、从 pre-commit 运行的 Rust linter，已经通过这种方式用到了下游仓库，所以这条分发途径已经试过。
