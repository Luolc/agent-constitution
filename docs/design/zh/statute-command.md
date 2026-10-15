<!-- en-sha256: ef6da3c0e6697df7df14624770767b2fe2e920c53d1484e38ac31201c1d002b4 -->
# statute 命令

这是[规定体系](rule-system.md) 的一部分，那里的状态和术语也适用于这里。

## <a id="compilation-and-the-site"></a>编译与网站

**在本地**，编译器读取宪法 (agent 版) 和一份规定清单及其锁文件，写出：

- 一份 `AGENTS.md`，开头写明它依据的宪法的仓库、版和修订号，然后是启用的规定，沿着 parent 树放在嵌套的块里，树每深一层，标题就降一级，下面有启用规定的每个领域是最上一级标题，领域有 applicability 时紧跟在标题后面；根没有启用的子树 (对 one-of 来说：它的子节点一个都没有启用) 不产生任何内容，因为没有它，其中的规定一条也启用不了；依赖的节点在发出依赖的规定所在的最近一级领域 (规定在子领域下时就是子领域) 之外时，渲染成一个链接。默认每条规定只收它的要求，让文件尽量小；规定清单可以加上理由或核验方法 ([规定清单与锁文件](rule-sets.md#rule-sets-and-the-lock-file))，审查方则无论如何都会拿到它们。文件暂时不拆。等用它的某个仓库编译出的文件用 `count-tokens` 量过之后，再按量出的长度定一个阈值，超过阈值的文件沿领域拆开；
- 启用的规定需要的 skill 目录；
- 要安装的检查二进制文件的版本，以及启用规定的外部命令。

`AGENTS.md` 只收录 `uses` 含 `instructions` 的启用规定 ([规定用在哪里](rule-config.md#where-a-rule-is-used))；参数实现之后，每个占位符都换成它的取值 ([参数](rule-config.md#parameters))。

`statute compile` 把 `AGENTS.md` 写在 `statute.toml` 旁边，用 `--output` 可以写到别的文件，另外两样写在它旁边 (见下)；锁文件必须与 `statute lock` 此刻会写出的内容一致，否则不写。`AGENTS.md` 依次包含：一行说明文件是生成的注释；一行写明宪法的版、修订号和规定源；规定清单所用语言的宪法 agent 版，即宪法构建写出的 `constitution/<edition>/generated/constitution.<lang>.agent.md`，原样收入；最后是标题「规定」及启用的规定。在公开仓库里，它最后还多一行，给找不到指向全文的软链的 agent 看，全文由私有层编译而成 (见[公开仓库里的私有规定](rule-sources.md#private-rules-in-a-public-repository))：「If `.statute/AGENTS.full.md` exists, read it and follow it: it is this file with the repository's private rules added.」(中文是「如果 `.statute/AGENTS.full.md` 存在，读它并照做：它是本文件加上本仓库的私有规定。」)。每个 domain 是二级标题，子领域是三级标题 (见[子领域](rule-sources.md#subdomains))，树每深一层标题降一级，到六级为止，更深的层级停在六级；一个子节点都没启用的 one-of 不出现。第一方规定源的领域排在最前，接着是其他规定源的领域，按规定源的名字排，最后是仓库自己的。节点的标题是它的标题和 statute 显示的引用 (`R42`)，前面放锚点，由 `rule-` 加上这个引用组成，引用里的冒号换成连字符：`rule-R42`、`rule-L12`、`rule-team-7`，所以不同规定源的锚点不会重复。覆盖别的规定的规定 ([覆盖](rule-sources.md#overrides)) 站在被覆盖的那条规定的位置上，覆盖链上每条规定的锚点都放在它的标题前面，所以指向 `#rule-R2` 的链接会落到它上面；链上每条规定的子节点都放在它下面。标题下面有一行写明它覆盖了什么：`Overrides R2.`，在一条链上则是 `Overrides team:7, which overrides R2.` (中文是 `覆盖 team:7，后者覆盖 R2。`)。对被覆盖的规定的依赖和引用，都渲染成站在它位置上的那条规定。依赖的节点在发出依赖的规定所在的最近一级 domain 或子领域之外时，写一行「依赖：」并链接到那个锚点；在它之内的依赖不写出来。规定有该语言的译文就用译文，没有就用原文。领域有 applicability ([规定文件格式](rule-format.md#rule-file-format)) 时，它作为一段文字放在领域标题下面，与标题用同一种语言，其中的引用按规定正文的方式渲染。规定正文里的引用会渲染出来 ([ID](source-formats.md#ids))：`{@rule:<id>}` 写成被引节点的标题，与它的标题行一样选语言，节点在文件里就链接到它的锚点，不在就在后面加「(规定 R42)」；`{@constitution:<id>}` 写成该条款在这种语言里的标签，与 agent 版一样是纯文本，例如 `第四条第一款第二项`。标签按这一版宪法的源算出：条和款的编号取自文件名，项的编号取自 `number`，算法与宪法构建相同 ([编译](compilation.md))；与宪法构建不同的是，连着的几处引用不缩写 (`第四条第一款、第四条第二款`)。引用了不存在的节点或条款时命令失败，并指出是哪条规定的哪一部分。规定文件和锁文件里的哈希都不变。

在 `AGENTS.md` 所在的目录里，`statute compile` 还会写：

- 启用的规定需要的每个 skill ([规定与 skill](rule-format.md#rules-and-skills))：把规定所在规定源里的 `skills/<name>/` 整个复制到 `.agents/skills/<name>/` (需要的两个 skill 同名、来自两个规定源时，命令失败)，加一个文件 `.statute-generated`，再建一个指向这份副本的软链 `.claude/skills/<name>`，目标写成相对路径 `../../.agents/skills/<name>`。每个 skill 各有一个软链，不给整个目录建一个，也不只链 `SKILL.md`；Claude Code 只从 `.claude/skills/` 读仓库的 skill (2026-10-14 读取)。只有 Unix 系统上才建软链：在别的系统上，规定清单需要 skill 时命令失败。skill 里只能有目录和普通文件；
- `statute-checks.toml`：编译它的 statute 的版本，写成 `statute = "<version>"`，规定用到的内置检查就在这个二进制里；然后每条 `check = "command"` 的启用规定一张 `[[commands]]` 表，按 ID 排序，写明它的 `rule` (`"R42"`)、`run` 和 `tools`。这个文件随 statute 的版本变化。它放在仓库顶层，不放进 `.statute/`：它要提交，CI 要读，改动它时要和生成它的 `AGENTS.md` 放在一起审。

每个文件开头都有一行注释，说明是 statute 写的 (`AGENTS.md` 里是 `<!-- ... -->`，`statute-checks.toml` 里是 `# ...`)。命令动手写之前，先查一遍要写的每个位置；只要有一处放着不是它写的东西 (没有那行开头的文件、没有 `.statute-generated` 的 skill 目录、不是它所建软链的 `.claude/skills/<name>`)，就什么都不写，命令失败。要写 skill 时，`.agents/skills` 和 `.claude/skills` 必须是普通目录，或者不存在：仓库把整个 `.claude/skills` 链到 `../.agents/skills` 时命令失败，并指出这个软链，因为 statute 是每个 skill 各建一个软链。带 `.statute-generated`、但规定清单已经不需要的 skill 目录会被删掉，连同它的软链；`.agents/skills/` 和 `.claude/skills/` 里的其他东西都不动。同样的输入编译出逐字节相同的结果，软链也一样。这些文件不是一次性写完的：中途出错 (比如磁盘满了) 可能留下一部分产物，再运行一次命令就能补完。写入顺序保证了这一点：skill 的 `.statute-generated` 先于它的文件写，软链先于它的目录删，`AGENTS.md` 最后写。

写出的 `AGENTS.md` 旁边 (不给 `--output` 时就是仓库顶层) 有 `CLAUDE.md` 时，不论是普通文件还是指向 `AGENTS.md` 的软链，每次 `statute compile` 都给出 warning，有没有私有层都一样。warning 问要不要删掉它，并用几句短话说明原因：没有 `CLAUDE.md` 时 Claude Code 自己会读 `AGENTS.md` ([规定清单与锁文件](rule-sets.md#rule-sets-and-the-lock-file)，monorepo 一段)，所以不需要 `CLAUDE.md` 指向它；有私有输出时，`CLAUDE.local.md` 用全文代替 `AGENTS.md` 给 Claude Code 读，Claude Code 还会再读 `CLAUDE.md`，它的内容就叠在规定之上，指向 `AGENTS.md` 的软链会让公开规定读两遍 ([公开仓库里的私有规定](rule-sources.md#private-rules-in-a-public-repository))，没有私有层时 warning 不说这一句；迁移时 ([statute-migrate](../../../skills/statute-migrate/SKILL.md)，第 e 步)，普通文件的 `CLAUDE.md` 改名为旧版副本，指向 `AGENTS.md` 的软链不改名，只问用户要不要删。statute 既不删它，也不改名。尚未实现：warning 仍说迁移时两种都会改名。

编译、检查和升级规定清单由同一个命令完成 ([命令与第一版](#the-command-and-the-first-version))。宪法的编译器暂时仍用 Python ([编译](compilation.md))，以后再并入这个命令。

**公开网站**列出每条规定，让人们能从完整列表里挑选，列表的筛选默认不显示已被取代和已废止的规定；被取代的规定保留自己的页面，指向它的链接仍然有效。本仓的规定在网站上也显示成 `R42`。列表和规定详情页显示什么，见[静态网站](static-site.md)。

## <a id="the-command-and-the-first-version"></a>命令与第一版

**命令。** 规定体系的命令用 Rust 写 (见[为什么用 Rust](#why-rust))，在 `crates/statute` 这个 crate 里逐步实现 ([文件结构](file-structure.md))。第一版就是这一个 crate。`statute` 这个名字已经用一个占位版本在 crates.io 上占住 ([发布与版本](amendments-and-releases.md#releases-and-versions))；第一次正式发布前仍可能改名，改了就要用新名字另行发布。规定清单是 `statute.toml`，锁文件是 `statute.lock`，命令有：

- `init`：写一份新的规定清单；加 `--global` 时写全局配置 ([全局配置与 git 规定源](global-configuration.md))；
- `source add`：在规定清单、它的私有层或全局配置里登记一个规定源 ([添加规定源](global-configuration.md#adding-a-source))；
- `add` 和 `remove`：在规定清单里启用一条规定或去掉它，并更新锁文件；加 `--private` 时改的是私有规定清单 (见[公开仓库里的私有规定](rule-sources.md#private-rules-in-a-public-repository))；
- `lock`：把规定清单解析成锁文件；
- `compile`：写出编译输出 ([编译与网站](#compilation-and-the-site))；加 `--private-only` 时只写私有输出，供新 worktree 里的 git 钩子用，不联网，也从不失败 ([公开仓库里的私有规定](rule-sources.md#private-rules-in-a-public-repository)，新的 worktree 一段)；
- `check`：按 domain 列出启用的规定供审查，脚本能检查的附上结果 ([检查](rule-checks.md#checks))；
- `upgrade`：把规定源和宪法移到更新的版本 ([规定清单与锁文件](rule-sets.md#rule-sets-and-the-lock-file))；
- `explain`：说明一条规定为什么被启用、哪些规定依赖它；
- `site`：为规定清单能用到的规定生成一个本地网站 ([本地网站](#the-local-site))；
- `export`：把一个目录里的宪法和规定以 JSON 打印出来，供读取它们的程序使用 ([导出](#export))；
- `constitution build`：从源文件构建所在仓库的宪法，或者检查它 (见下文)；
- `validate <dir>`：单独检查一个规定目录，当作一个规定源，不需要规定清单 (见下文)。

**第一版**只做仓库级。用户级是第二步，它的指令文件在用户的家目录里，目前由私有 overlay 安装。本仓的规定已经迁到新格式，每条把原来的文字 ID 留作 `slug`。最先用新格式写的规定里，有关于怎样写设计文档的那几条，正好拿实际工作把整套流程试一遍。

**目前已实现**：`init`、`lock`、`explain`、`add`、`remove`、`upgrade`、`compile` 和 `check` (内置检查有规定 43、46、50、58、61 和 71，以及被 43 和 46 取代的 3 和 7) 八个命令，以及 `site`、`constitution build` 和 `validate`，支持磁盘上的规定源和 git 规定源，`[sources]` 写几个就读几个；还有 `export`。仓库自己的规定 `L12` 从 `.statute/rules/` 或 `local-rules` 指定的目录读 (见[本地规定的目录](rule-config.md#the-directory-of-local-rules))；那个目录是某个规定源的规定目录时，比如 `local-rules = "rules"` 加上 `R = { path = "." }`，这些规定归那个规定源，本地引用会报错并提示写成 `R12`。规定在 `requires`、`conflicts` 和 `{@rule:...}` 里引用别的规定源的规定时，写作 `R12` 或 `team:7`，用的是规定清单 `[sources]` 里的名字：规定源自己的 `[sources]` 表 ([规定源与引用](rule-sets.md#sources-and-references)) 还不读。[规定源](rule-sources.md) 里已实现的是 `public`，连同 `requires`、`conflicts`、`overrides`、`parent` 和 `{@rule:...}` 里哪些规定可以引用哪些，以及把 `public` 与 GitHub 核对的检查 ([检查](rule-checks.md#checks))。覆盖已经实现，连同 `scope` 和级别；子领域、规定或领域指向另一个规定源的 `parent`，以及放在 git 目录里的私有层连同它的锁文件、它的输出、`.statute/.gitignore` 和 `compile --private-only`，也都已实现。全局配置、`init --global`、git 规定源和 `source add` 已经实现 ([全局配置与 git 规定源](global-configuration.md))，只是 `upgrade` 还不显示一条规定改动的文字。[规定配置](rule-config.md) 里，规定文件的 `default`、`uses` 和 `check = "none"` 已经读取并检查，连同[规定不挂在规定下面](rule-config.md#no-rule-under-a-rule) 的树形，`compile` 写什么、`check` 列什么也按 `uses` 来；规定清单的默认开关也已实现，连同 `domains`、`exclude`、`local-rules` 和规定源条目的 `defaults`，以及 `add`、`remove`、`explain` (包括 `--defaults`) 和锁文件对它们的处理。`default` 在规定文件需要它的地方必填，`check = "review"` 报错。参数也已实现，取值写进锁文件、`upgrade`、`explain <name>` 和两份输出。还没有实现的是：one-of 的渲染，以及公开网站上的标记；本地网站的标记已经实现。继承规定清单和按路径分段还没有实现。

**`init`** 在当前目录写一份 `statute.toml`：宪法取自哪个规定源、哪一版，`public = true` 或 `public = false`，空的 `rules`，以及给了 `--source` 时的一个磁盘上的规定源，即第一方规定源 `R`。`public` 取 `--public` 或 `--private` 指定的值 (两者最多给一个)，没有指定时取 `gh` 显示的这个仓库的情况，见[公开与否](rule-sources.md#public-or-not)。`--source <dir>` 指定规定源的目录，相对于当前目录。不给它时，`init` 不写 `[sources]`，`R` 取全局配置里的或内置默认的那个 ([一个名字指哪个规定源](global-configuration.md#which-source-a-name-means))，读它所跟踪分支的最新提交。`--edition` 指定哪一版；不给时，`init` 取规定源 `constitution/` 下唯一一个含 `index.toml` 的目录，一个都没有或多于一个时报错，并列出找到的那些。写之前它先解析一遍这份规定清单，解析失败就什么都不写。它不写 `statute.lock`：没有启用任何规定，也就没有要锁定的东西；第一次 `add` 会写出锁文件，已有的 `statute.lock` 会被替换。已有 `statute.toml` 时原样保留，`init` 以退出码 1 退出。

**`upgrade`** 把解析结果与 `statute.lock` 比较，打印变了什么 ([规定清单与锁文件](rule-sets.md#rule-sets-and-the-lock-file))，没有变化就说锁文件已是最新，然后像 `lock` 一样写锁文件。没有 `statute.lock` 时没有可比的东西，以退出码 1 退出。加上 `--replace-superseded` 时，它先在 `rules` 和 `packs` 里把每条被取代的规定换成它的后继规定，顺着取代链找到最后一条，文件其余部分保持原样；后继规定已经列出的，只去掉旧的那一项。取代链走到一条有多个后继规定的规定时，这一项保持不动，把这几条后继规定列出来，由用户用 `add` 和 `remove` 自己选。已废止的规定留给用户自己去掉。改过的规定清单解析不通过时，两个文件都不改。

**`validate <dir>`** 把一个目录里的规定文件当作第一方规定源读，连同 `params.toml` 里的参数，做所有不需要规定清单的检查 ([检查程序](checkers.md#checkers))：每个节点和参数的格式、规定树、引用的目标、译文、引用和占位符，以及默认值。每条发现单独一行写到标准错误；有错误时以状态 1 退出，没有问题时向标准输出打一行，写明节点数。本仓把 `statute validate rules` 作为 commit 钩子运行。

**`constitution build`** 在宪法上做的事与 `uv run constitution-build` 相同：检查 `constitution/` 下唯一一版的源文件，为每种语言把完整版和 agent 版写到这一版的 `generated/` 里，与那个命令写出的逐字节相同；加 `--check` 时什么都不写，已提交的文件与重新构建的结果不同就失败；加 `--check --base <git-ref>` 时还要与那个提交比较编号和修订号；`--root <dir>` 让它在当前目录以外的检出上运行。它做的检查是[检查程序](checkers.md#checkers) 里第 1 至第 7 项中关于宪法的部分，报错文字也相同。它不读 `rules/`：那个命令对规定的检查，包括规定对宪法的引用和 base 提交里已有的规定，这里都没有实现。生成文件里写的仍是 `uv run constitution-build`，CI、提交钩子和本设计的其他部分也仍然运行它；换成 `statute constitution build`、删掉 Python 的命令，都还没有做。

**切换。** 目前 agent 读的是私有 overlay 的用户级指令文件。这些文件将加一条：仓库里有 `statute.lock` 的，以这个仓库的编译输出为准，用户级指令文件不再适用。它们管着的所有仓库一次切换：等它们的每一部分都成了规定，并且[从手写的指南切换过来](rule-sources.md#cutover-from-the-hand-written-guide) 的几个条件都成立；不先切某一个主题。在那之前，编译输出照常构建、用来对比，但不代替它们给 agent 读。

## <a id="the-local-site"></a>本地网站

`statute site` 把规定清单能用到的规定生成一个网站，写到 `statute.toml` 旁边的 `.statute/site/`，在浏览器里直接从磁盘打开；它从不发布，也不是[公开网站](static-site.md)。它读 `statute.toml`，有私有规定清单时也读，两层都在内存里解析，每个 git 规定源读锁文件记录的那个提交 ([git 规定源](global-configuration.md#git-sources))；它不写锁文件。最后打印 `index.html` 的路径。

- **文件。** 纯 HTML，加一份内嵌的样式表：没有脚本，不用 Hugo，不引入 statute 现有依赖之外的依赖。每次运行都重新生成：先删掉旧的 `.statute/site/`，但只在这个目录里有 statute 写的 `.gitignore` (见下) 时才删，否则什么都不写并报错。除了 `.statute/.gitignore`，`.statute/site/` 之外什么都不写。
- **让 git 忽略。** statute 写 `.statute/site/.gitignore`，内容是 `*`，这样 git 忽略那里的每个文件，包括它自己，它也标明这个目录归 statute。statute 还写 `.statute/.gitignore`，其中列出 `/site/`，每个往 `.statute/` 写生成文件的命令都这样做 (见[公开仓库里的私有规定](rule-sources.md#private-rules-in-a-public-repository))。在 git 仓库里，它先用 `git ls-files` 问 `.statute/site/`，git 跟踪着那里的某个文件时，什么都不写并报错，写明是哪个文件和解决办法 `git rm -r --cached .statute/site`。git 回答不了时 (起不来或出错)，也什么都不写，报错并带上 git 的错误信息。不在 git 仓库里时没有东西会被提交，也就不用问。
- **首页**按四类规定源 ([四类规定源](rule-sources.md#four-kinds-of-source)) 分节：全局公开、全局私有、本仓公开、本仓私有。每节列出它的规定源，每个规定源列出它的领域树，包括每个节点，不论启用与否：节点的引用、标题和状态，以及这份规定清单是启用它、覆盖它还是没选它。按规定源类别筛选、隐藏这份规定清单没有启用的规定，以及显示默认隐藏的已被取代和已废止的节点，都只靠复选框和样式表实现。
- **规定源属于哪一类**看它的两个属性：仓库自己的规定 (`L`) 和 `scope = "local"` 的规定源是本仓的，其余都是全局的，包括全局配置里的；公开与否按规定源自己的声明，`L` 按规定清单的声明。在 `.` 的规定源，比如本仓的 `R = { path = "." }`，是第一方规定源，所以是全局公开的。`local-rules` 指向它的规定目录时，它的规定同时也是仓库自己的规定：它只在全局公开下列一次，本仓公开那一节写明这一点并链接过去；否则仓库自己的规定列在它所属类别的本仓那一节。尚未实现：不论 `local-rules` 指向哪里，网站仍把在 `.` 的规定源当作仓库自己的规定。每个节点在首页和它自己的页面上都标出所属规定源的类别：用页面的语言写出类别名，底色是四种颜色之一。节点自己的页面上，标记还写出规定源的名字，例如 `全局私有 · me`。
- **每个节点一页**，`<source>/<id>.html`：陈述、理由、验证方式和适用范围；`requires`、`conflicts`、`overrides` 和取代它的规定；上溯的父节点路径和子节点。领域的页面就是它下面节点的列表。对规定的引用链接到那条规定的页面，对宪法的引用写成条款的标签，和编译出的 `AGENTS.md` 一样。页面用 `[compile] lang` 指定的语言，节点的类别也是；状态按规定文件里的写法显示，比如 `draft` 或 `trial`。
- **[规定配置](rule-config.md) 的标记。** 每个领域和每条规定，在索引和自己的页面上，都写明默认开启还是默认关闭；one-of 的选项写明是不是 one-of 的默认选项。规定的页面写明它用在哪里：`AGENTS.md`、审查，或两者都有；索引有两个筛选，分别隐藏这两种规定。占位符显示这份规定清单给参数的取值 (这个人有自己的取值时用这个人自己的)，加上标记，鼠标悬停时显示参数名，并链接到它所在规定源的参数页 `<source>/parameters.html`。参数页列出每个参数的标题、名字、类型、范围和默认值，它的取值和取值从哪里来，以及用到它的启用规定。

## <a id="export"></a>导出

`statute export [<dir>]` 把一个目录 (默认是 `.`) 里的宪法和规定打印成一个 JSON 值，供读取它们的程序使用，本仓的网站就是这样用的 ([静态网站](static-site.md)，「规定」)。它不读规定清单：这个目录是一个规定源，里面有 `constitution/` 或 `rules/`，或者两者都有。

- `constitution`：没有 `constitution/` 时为 `null`；否则是它唯一那一版的 `index.toml` 里的 `edition`、`revision`、`status` 和 `languages` (一版都没有或不止一版时导出失败)，以及按正文顺序排列的 `provisions`：每个条、款、项给出引用它时写的 `id`、它的 `parent` (条为 `null`) 和按语言给出的 `labels`，写法与编译出的 `AGENTS.md` 相同。
- `nodes`：`rules/` 里的每个节点，按 ID 排序，给出它的 `file` 和它的 `fields`，即文件里原样写的各字段，正文也在内。程序拿到的是文件里原样写的值；statute 以后认识的新字段，不改导出也能传过去；默认值由程序自己补。只有一个值是 statute 自己算出来的，免得每个程序再算一遍：`on-by-default`，即规定源的默认值是否启用这个节点 ([默认开关](rule-config.md#default-switches))，已被取代或已废止的规定总是否；取决于别的规定源里的节点时为 `null`。

打印之前，statute 像检查任何规定源一样检查这些规定 ([依赖、one-of 与规定包](rule-sets.md#dependencies-one-ofs-and-packs))；检查不通过时，它只打印错误，以退出码 1 退出。依据旧原文翻译的译文过不了这项检查，所以节点列出的每份译文都跟随当前原文。JSON 是唯一的格式，所以这个命令没有选择格式的选项。

## <a id="why-rust"></a>为什么用 Rust

速度不是主要原因。几百次文件检查和正则表达式匹配，用 Python 也只要一两秒；真正花时间和钱的是审查方调用模型。选 Rust 的原因是：

- 检查作为一个二进制文件发布，不需要 Python 或别的运行时，所以用任何语言的下游仓库都能用；
- 并行运行检查更容易写对；
- [limae](https://github.com/Luolc/limae) 是一个以 Python wheel 发布、从 pre-commit 运行的 Rust linter，已经通过这种方式用到了下游仓库，所以这条分发途径已经试过。
