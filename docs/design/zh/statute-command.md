<!-- en-sha256: b6d12d1af9a1ef56a183ea31f593db3c929459aaf4613eb227e35150692b3df6 -->
# statute 命令

这是[规定体系](rule-system.md) 的一部分，那里的状态和术语也适用于这里。

## <a id="compilation-and-the-site"></a>编译与网站

**在本地**，编译器读取宪法 (agent 版) 和一份规定清单及其锁文件，写出：

- 一份 `AGENTS.md`，开头写明它依据的宪法的仓库、版和修订号，然后是启用的规定，沿着 parent 树放在嵌套的块里，树每深一层，标题就降一级，下面有启用规定的每个领域是最上一级标题，领域有 applicability 时紧跟在标题后面；根没有启用的子树 (对 one-of 来说：它的子节点一个都没有启用) 不产生任何内容，因为没有它，其中的规定一条也启用不了；依赖的节点在发出依赖的规定所在的最近一级领域 (规定在子领域下时就是子领域) 之外时，渲染成一个链接。默认每条规定只收它的要求，让文件尽量小；规定清单可以加上理由或核验方法 ([规定清单与锁文件](rule-sets.md#rule-sets-and-the-lock-file))，审查方则无论如何都会拿到它们。文件暂时不拆。等用它的某个仓库编译出的文件用 `count-tokens` 量过之后，再按量出的长度定一个阈值，超过阈值的文件沿领域拆开；
- 启用的规定需要的 skill 目录；
- 要安装的检查二进制文件的版本，以及启用规定的外部命令。

`statute compile` 把 `AGENTS.md` 写在 `statute.toml` 旁边，用 `--output` 可以写到别的文件，另外两样写在它旁边 (见下)；锁文件必须与 `statute lock` 此刻会写出的内容一致，否则不写。`AGENTS.md` 依次包含：一行说明文件是生成的注释；一行写明宪法的版、修订号和规定源；规定清单所用语言的宪法 agent 版，即宪法构建写出的 `constitution/<edition>/generated/constitution.<lang>.agent.md`，原样收入；最后是标题「规定」及启用的规定。在公开仓库里，它最后还多一行，告诉 agent 有 `AGENTS.private.md` 时接着读它，并附上链接；私有层就编译到这个文件 (见[公开仓库里的私有规定](rule-sources.md#private-rules-in-a-public-repository)；暂定，尚未实现)。每个 domain 是二级标题，子领域是三级标题 (见[子领域](rule-sources.md#subdomains)；尚未实现)，树每深一层标题降一级，到六级为止，更深的层级停在六级；一个子节点都没启用的 one-of 不出现。节点的标题是它的标题和 statute 显示的引用 (`R42`)，前面放锚点，由 `rule-` 加上这个引用组成，引用里的冒号换成连字符：`rule-R42`、`rule-L12`、`rule-team-7`，所以不同规定源的锚点不会重复。依赖的节点在发出依赖的规定所在的最近一级 domain 或子领域之外时，写一行「依赖：」并链接到那个锚点；在它之内的依赖不写出来。规定有该语言的译文就用译文，没有就用原文。领域有 applicability ([规定文件格式](rule-format.md#rule-file-format)) 时，它作为一段文字放在领域标题下面，与标题用同一种语言，其中的引用按规定正文的方式渲染。规定正文里的引用会渲染出来 ([ID](source-formats.md#ids))：`{@rule:<id>}` 写成被引节点的标题，与它的标题行一样选语言，节点在文件里就链接到它的锚点，不在就在后面加「(规定 R42)」；`{@constitution:<id>}` 写成该条款在这种语言里的标签，与 agent 版一样是纯文本，例如 `第四条第一款第二项`。标签按这一版宪法的源算出：条和款的编号取自文件名，项的编号取自 `number`，算法与宪法构建相同 ([编译](compilation.md))；与宪法构建不同的是，连着的几处引用不缩写 (`第四条第一款、第四条第二款`)。引用了不存在的节点或条款时命令失败，并指出是哪条规定的哪一部分。规定文件和锁文件里的哈希都不变。

在 `AGENTS.md` 所在的目录里，`statute compile` 还会写：

- 启用的规定需要的每个 skill ([规定与 skill](rule-format.md#rules-and-skills))：把规定源里的 `skills/<name>/` 整个复制到 `.agents/skills/<name>/`，加一个文件 `.statute-generated`，再建一个指向这份副本的软链 `.claude/skills/<name>`，目标写成相对路径 `../../.agents/skills/<name>`。每个 skill 各有一个软链，不给整个目录建一个，也不只链 `SKILL.md`；Claude Code 只从 `.claude/skills/` 读仓库的 skill (2026-10-14 读取)。只有 Unix 系统上才建软链：在别的系统上，规定清单需要 skill 时命令失败。skill 里只能有目录和普通文件；
- `statute-checks.toml`：编译它的 statute 的版本，写成 `statute = "<version>"`，规定用到的内置检查就在这个二进制里；然后每条 `check = "command"` 的启用规定一张 `[[commands]]` 表，按 ID 排序，写明它的 `rule` (`"R42"`)、`run` 和 `tools`。这个文件随 statute 的版本变化。

每个文件开头都有一行注释，说明是 statute 写的 (`AGENTS.md` 里是 `<!-- ... -->`，`statute-checks.toml` 里是 `# ...`)。命令动手写之前，先查一遍要写的每个位置；只要有一处放着不是它写的东西 (没有那行开头的文件、没有 `.statute-generated` 的 skill 目录、不是它所建软链的 `.claude/skills/<name>`)，就什么都不写，命令失败。要写 skill 时，`.agents/skills` 和 `.claude/skills` 必须是普通目录，或者不存在：仓库把整个 `.claude/skills` 链到 `../.agents/skills` 时命令失败，并指出这个软链，因为 statute 是每个 skill 各建一个软链。带 `.statute-generated`、但规定清单已经不需要的 skill 目录会被删掉，连同它的软链；`.agents/skills/` 和 `.claude/skills/` 里的其他东西都不动。同样的输入编译出逐字节相同的结果，软链也一样。这些文件不是一次性写完的：中途出错 (比如磁盘满了) 可能留下一部分产物，再运行一次命令就能补完。写入顺序保证了这一点：skill 的 `.statute-generated` 先于它的文件写，软链先于它的目录删，`AGENTS.md` 最后写。

编译、检查和升级规定清单由同一个命令完成 ([命令与第一版](#the-command-and-the-first-version))。宪法的编译器暂时仍用 Python ([编译](compilation.md))，以后再并入这个命令。

**公开网站**列出每条规定，让人们能从完整列表里挑选，列表的筛选默认不显示已被取代和已废止的规定；被取代的规定保留自己的页面，指向它的链接仍然有效。本仓的规定在网站上也显示成 `R42`。列表和规定详情页显示什么，见[静态网站](static-site.md)。

## <a id="the-command-and-the-first-version"></a>命令与第一版

**命令。** 规定体系的命令用 Rust 写 (见[为什么用 Rust](#why-rust))，在 `crates/statute` 这个 crate 里逐步实现 ([文件结构](file-structure.md))。第一版就是这一个 crate。`statute` 这个名字已经用一个占位版本在 crates.io 上占住 ([发布与版本](amendments-and-releases.md#releases-and-versions))；第一次正式发布前仍可能改名，改了就要用新名字另行发布。规定清单是 `statute.toml`，锁文件是 `statute.lock`，命令有：

- `init`：写一份新的规定清单；
- `add` 和 `remove`：在规定清单里启用一条规定或去掉它，并更新锁文件；
- `lock`：把规定清单解析成锁文件；
- `compile`：写出编译输出 ([编译与网站](#compilation-and-the-site))；
- `check`：按 domain 列出启用的规定供审查，脚本能检查的附上结果 ([检查](rule-checks.md#checks))；
- `upgrade`：把规定源和宪法移到更新的版本 ([规定清单与锁文件](rule-sets.md#rule-sets-and-the-lock-file))；
- `explain`：说明一条规定为什么被启用、哪些规定依赖它。

**第一版**只做仓库级。用户级是第二步，它的指令文件在用户的家目录里，目前由私有 overlay 安装。本仓的规定已经迁到新格式，每条把原来的文字 ID 留作 `slug`。最先用新格式写的规定里，有关于怎样写设计文档的那几条，正好拿实际工作把整套流程试一遍。

**目前已实现**：`init`、`lock`、`explain`、`add`、`remove`、`upgrade`、`compile` 和 `check` (内置检查有规定 43、46、50、58、61 和 71，以及被 43 和 46 取代的 3 和 7) 八个命令，只支持一个磁盘上的规定源。继承规定清单和按路径分段都还没有实现；git 规定源、多个规定源，以及在 `requires` 或 `conflicts` 里对别的规定源的引用 (`"R12"` 或 `"team:7"`)，会报告为暂不支持。三种引用都按上文的写法读取和显示，但只有一个规定源时，规定清单只能用这个规定源里的规定：第一方规定源就是规定清单所在的目录时，本地规定会报错并提示写成 `R12`；第一方规定源在别处时，本地规定会报告为暂不支持，因为那时本地规定就成了第二个规定源。[规定源](rule-sources.md) 一部分也没有实现：`public` 和 `scope`、`statute.private.toml`、覆盖和子领域。

**`init`** 在当前目录写一份 `statute.toml`：宪法取自哪个规定源、哪一版，空的 `rules`，以及一个磁盘上的规定源，即第一方规定源 `R`。`--source <dir>` 指定规定源的目录，相对于当前目录，默认是 `.`。`--edition` 指定哪一版；不给时，`init` 取规定源 `constitution/` 下唯一一个含 `index.toml` 的目录，一个都没有或多于一个时报错，并列出找到的那些。写之前它先解析一遍这份规定清单，解析失败就什么都不写。它不写 `statute.lock`：没有启用任何规定，也就没有要锁定的东西；第一次 `add` 会写出锁文件，已有的 `statute.lock` 会被替换。已有 `statute.toml` 时原样保留，`init` 以退出码 1 退出。

**`upgrade`** 把解析结果与 `statute.lock` 比较，打印变了什么 ([规定清单与锁文件](rule-sets.md#rule-sets-and-the-lock-file))，没有变化就说锁文件已是最新，然后像 `lock` 一样写锁文件。没有 `statute.lock` 时没有可比的东西，以退出码 1 退出。加上 `--replace-superseded` 时，它先在 `rules` 和 `packs` 里把每条被取代的规定换成它的后继规定，顺着取代链找到最后一条，文件其余部分保持原样；后继规定已经列出的，只去掉旧的那一项。取代链走到一条有多个后继规定的规定时，这一项保持不动，把这几条后继规定列出来，由用户用 `add` 和 `remove` 自己选。已废止的规定留给用户自己去掉。改过的规定清单解析不通过时，两个文件都不改。

**切换。** 目前 agent 读的是私有 overlay 的用户级指令文件。这些文件将加一条：仓库里有 `statute.lock` 的，以这个仓库的编译输出为准，用户级指令文件不再适用。它们管着的所有仓库一次切换：等它们的每一部分都成了规定，并且[从手写的指南切换过来](rule-sources.md#cutover-from-the-hand-written-guide) 的几个条件都成立；不先切某一个主题。在那之前，编译输出照常构建、用来对比，但不代替它们给 agent 读。

## <a id="why-rust"></a>为什么用 Rust

速度不是主要原因。几百次文件检查和正则表达式匹配，用 Python 也只要一两秒；真正花时间和钱的是审查方调用模型。选 Rust 的原因是：

- 检查作为一个二进制文件发布，不需要 Python 或别的运行时，所以用任何语言的下游仓库都能用；
- 并行运行检查更容易写对；
- [limae](https://github.com/Luolc/limae) 是一个以 Python wheel 发布、从 pre-commit 运行的 Rust linter，已经通过这种方式用到了下游仓库，所以这条分发途径已经试过。
