<!-- en-sha256: 53ef07b9d5317b5781c23896f5ec0edf382c1e726729be57006733e739e0ddcb -->
# 规定格式

这是[规定体系](rule-system.md) 的一部分，那里的状态和术语也适用于这里。

## <a id="what-one-rule-holds"></a>一条规定包含什么

一条规定只要求一件事，而且是否违反可以判定：给定一个仓库、一次变更或一份记录，有人能回答「这条规定有没有被违反？」。它就像 linter 里的一条 lint 规则，范围窄到可以检查，这正是 `decidable-provisions` 对每条下位法条文的要求。

它的正文分三部分：要求 (statement)，用一句命令句说该做什么；理由 (reason)；核验方法 (verification)，即怎样判断这条规定有没有被违反。每部分都很短。需要写更多的规定，通常是把几条规定写成了一条，应当拆开。

## <a id="ids-and-revisions"></a>ID 与修订号

规定的 ID 是一个数字，按顺序分配，永不重复使用。预计规定会增长得很快，而且主要由 agent 编写和检查，所以人不会逐条细读；用数字可以免去为起名争论，措辞改变时也从不需要改 ID。领域和 one-of 的 ID 与规定取自同一个序列，所以一个数字在它的规定源里只指一个节点。

规定永不删除。一次修改保不保留 ID，取决于一个问题：它有没有改变什么算合规？

- 没有改变，就原地修改这条规定，修订号加一。改写措辞、增加一个例子、修复检查里的一个 bug：原来合规的仍然合规，原来违反的仍然违反。
- 改变了，修改后的规定就取一个新 ID。旧规定标为被取代，写明取代它的规定 (拆成几条时写明这几条)，并永久留在规定源里。只有同一规定源的规定才能取代它 (见[规定源与引用](rule-sets.md#sources-and-references))。

所以一条规定的 ID 始终代表同一项要求。修订号是每条规定各自的一个数字，从 1 起算；它与宪法的修订号无关。规定的状态是 `draft`、`trial`、`active`、`superseded` 或 `repealed` 之一。宪法还是草案时，每条规定都是 `draft`；宪法现行之后，新规定从 `trial` 开始，也就是先在实际工作中试用，稳定之后再改为 `active` (两者的含义、由谁来改，见[试行规定](rule-checks.md#trial-rules))。`draft` 只留给还不打算用于实际工作的规定，例如 [检查程序](checkers.md) 里的金丝雀规定。规定从 `draft` 转为 `trial`，只改状态，不改什么算合规，所以和上面说的同类改动一样，原地修改，修订号加一。撤销而没有替代的规定为废止，它与被取代的规定一样，保留自己的 ID 和文件。

只改规定译文的改动，修订号不变：修订号跟着原文和文字以外的字段走，改译文两样都不动 (见[规定文件格式](#rule-file-format) 里的「语言」)。锁文件除了记每条规定文件的 SHA-256，还记去掉译文之后的规定的 SHA-256，statute 靠它分清这两种改动 (见[规定清单与锁文件](rule-sets.md#rule-sets-and-the-lock-file))。`statute validate --base` 用的也是这个哈希：哈希与基准的不同的节点，修订号要比基准的大 (见[检查程序](checkers.md#checkers))。

## <a id="rule-file-format"></a>规定文件格式

parent 树上的每个节点 (一条规定、一个 one-of 或一个领域) 是一个 TOML 文件 `rules/<id>.toml`，全部平铺在一个目录里：`rules/42.toml`，不补零，因为 ID 会一直增长，补到几位迟早都不够。树的结构只记在 `parent` 字段里，从不靠目录表达，所以把一条规定挪到另一个 parent 下只改一行，不搬文件。工具一律按数字给文件排序，不按文件名。目录里还可以有 `params.toml`，即规定源的参数 (见[声明参数](rule-config.md#declaring-a-parameter))；它不是节点。

一条规定，这里的例子带了中文译文：

```toml
id = 42
slug = "uv-sync-locked-in-ci"
revision = 1
kind = "rule"
status = "trial"
lang = "en"
parent = 7
requires = [12]
authority = ["simplicity"]
constitution = ">=1"
level = "project"
check = "builtin"
default = "on"
uses = ["instructions", "review"]

[title]
en = "Sync from the lock file first in CI"
zh = "CI 里先按锁文件同步依赖"

[text.statement]
en = "In CI, run `uv sync --locked` before any step that runs Python."
zh = "在 CI 里，先运行 `uv sync --locked`，再运行任何执行 Python 的步骤。"

[text.reason]
en = '''
A job that resolves its dependencies anew can test other versions than the
ones the lock file records, and a stale lock file goes unnoticed.
'''
zh = '''
重新解析依赖的 job 测到的版本可能与锁文件记录的不同，锁文件过期了也发现不了。
'''

[text.verification]
en = '''
Violated when a CI job runs Python before a `uv sync --locked` step, or
runs `uv sync` without `--locked`.
'''
zh = '''
CI 的 job 在 `uv sync --locked` 这一步之前运行了 Python，或者运行 `uv sync` 时没带 `--locked`，即为违反。
'''

[translations]
zh = "<SHA-256 of the original text>"
```

一个 one-of 和一个领域，格式相同：

```toml
id = 5
slug = "python-package-manager"
revision = 1
kind = "one-of"
status = "active"
lang = "en"
parent = 3
default = 6

[title]
en = "Python package manager"
```

```toml
id = 3
slug = "python"
revision = 1
kind = "domain"
status = "active"
lang = "en"
default = "off"

[title]
en = "Python"

[applicability]
en = '''
How Python code is written, tested and packaged. Enable it in a
repository that holds Python code; a repository with none does not need
it.
'''
```

各字段：

- `id`：从 1 起的整数，在本规定源的规定、one-of 和领域之间唯一，永不重复使用。文件名就是 ID。
- `slug` (可选)：一个可读的短名，由小写单词以连字符连接，在本规定源内唯一。它只用于网址，以及让人更容易认出这个文件；任何引用都不用它。本仓的规定迁到这个格式时，把原来的文字 ID 放在了这里。
- `revision`：从 1 起的整数，每次原地修改加一 ([ID 与修订号](#ids-and-revisions))。
- `kind`：`rule`、`one-of` 或 `domain`。
- `status`：`draft`、`trial`、`active`、`superseded` 或 `repealed`。
- `superseded-by`：取代这条规定的、同一规定源里的规定的 ID；几条规定一起取代它时写成 ID 的列表，`superseded-by = [22, 23]`。只在 `status` 为 `superseded` 时出现。每一项都是另一条规定，不重复；列表不能为空。
- `lang`：这条规定写成时用的语言，即原文的语言，写成语言代码，如 `"en"` 或 `"zh"`。
- `parent`：同一规定源里父节点的 ID：规定的是一个领域或一个 one-of，从不是规定；one-of 的是一个领域。规定的 parent 也可以是另一个规定源的领域或子领域，写法与 `requires` 相同 (`"team:3"`)。每条规定和每个 one-of 都有 parent，带 `overrides` 的规定除外；领域没有，作为子领域时则是一个领域，在同一规定源里，或者在另一个规定源里，写法与 `requires` 相同 (见[子领域](rule-sources.md#subdomains))，作为选项时则是一个 one-of。one-of 的选项要么全是规定，要么全是领域，领域可以嵌套任意多层 (见[规定不挂在规定下面](rule-config.md#no-rule-under-a-rule))。这些 statute 都会检查。
- `overrides` (可选，只有规定有)：这条规定在启用它的规定清单里所顶替的另一个规定源的规定，写法与 `requires` 相同 (`"R200"`)；带它的规定没有 `parent` (见[覆盖](rule-sources.md#overrides))。
- `requires` (可选)：启用这条规定所需的规定或 one-of。同一规定源里的节点写成数字，别的规定源里的写成字符串：规定源的短名、一个冒号和 ID (`"team:7"`)，短名是大写字母的规定源则写成字母加 ID (`"P7"`)，短名取自本规定源的 `[sources]` 表 ([规定源与引用](rule-sets.md#sources-and-references))。
- `conflicts` (可选)：不能与这条规定一起用的规定，写法同 `requires`，只写在一边 ([依赖、one-of 与规定包](rule-sets.md#dependencies-one-ofs-and-packs))。
- `authority`：这条规定所细化的宪法条文的 ID，写法与宪法引用自身时相同，不带命名空间 (`"simplicity"`、`"must-ask.capital"`)。这是字段的值，不是正文里的 `{@...}` 引用，所以 [ID](source-formats.md#ids) 要求规定的引用带命名空间，这一条不适用于它；规定正文里的引用照旧带命名空间 (`{@constitution:simplicity}`)。
- `constitution`：这条规定兼容的宪法修订号，写成 Python 的版本说明符 (`">=1"`、`">=1,<4"`；见[从规定追溯到宪法](#tracing-rules-to-the-constitution))。它和 `authority` 都必填，但不依据宪法的规定清单读规定时不要求这两项 ([规定清单与锁文件](rule-sets.md#rule-sets-and-the-lock-file))；依据宪法的规定清单两项都要求，`statute validate` 和 `statute export` 也要求，除非规定旁边没有宪法、运行所在目录的规定清单也不依据宪法 ([命令与第一版](statute-command.md#the-command-and-the-first-version))。
- `level`：默认层级，`project` 或 `user` ([规定清单与锁文件](rule-sets.md#rule-sets-and-the-lock-file))。
- `check`：怎样检查是否合规 ([检查](rule-checks.md#checks))。`builtin`：这条规定在本仓的 crate 里有一个模块。`command`：外部命令，另由两个字段给出，`run` 是命令行，`tools` 是它需要的工具列表 (`["uv"]`)。`none`：没有自动检查；审查方是否拿到这条规定，由 `uses` 决定。`review` 是 `none` 的旧名，写了就报错。
- `default`：必填，规定是否默认开启，`on` 或 `off`；one-of 的子节点没有，带 `overrides` 的规定也没有 (见[默认开关](rule-config.md#default-switches))。
- `uses` (可选，只有规定有)：规定去哪里，由 `instructions` 和 `review` 组成的非空列表；不写就是 `["instructions"]` (见[规定用在哪里](rule-config.md#where-a-rule-is-used))。
- `[title]`：简短的标题，只占一行 (多行字符串可以以换行结尾)。
- `[applicability]` (可选，只有领域有)：一两句话，说明这个领域管什么、哪些仓库该启用它、什么情况下不必启用。它可以像规定正文一样引用规定 (`{@rule:<id>}`) 和条款 ([ID](source-formats.md#ids))。网站把它放在领域详情页的标题下面 ([静态网站](static-site.md))；编译出的 `AGENTS.md` 不收它 ([编译与网站](statute-command.md#compilation-and-the-site))。
- `[text.statement]`、`[text.reason]` 和 `[text.verification]`：正文的三部分 ([一条规定包含什么](#what-one-rule-holds))。
- `[translations]` (可选)：每种译文语言一个键，原文语言不在其中；值是这份译文所依据的原文的 SHA-256。

领域和 one-of 只有 `id`、`slug`、`revision`、`kind`、`status`、`lang`、`parent` (one-of 都有，领域只在作为子领域或选项时有)、`default` (领域上是 `on` 或 `off`，作为 one-of 的选项时没有；one-of 上是它默认启用的那个选项的 ID)、`[title]`、`[applicability]` (只有领域有，可选)，以及有译文时的 `[translations]`。它们没有正文、检查或别的元数据。它们的 `status` 只说节点本身，不说其中的规定 ([试行规定](rule-checks.md#trial-rules))。

**语言。** 每段文字，即 `[title]`、`[applicability]` 和正文三部分，都是一张以语言代码为键的表，与[源文件格式](source-formats.md) 相同。`lang` 指明的那种语言必填，其他语言都是可选的译文。所以一条规定可以只用一种语言写，用什么语言写作的人都能直接写。一份译文要给出这个节点在这种语言里的每段文字 (规定是标题和全部三部分，领域是标题和 applicability，如果有的话)，它在 `[translations]` 里的键记下它依据的是哪一版原文。这个哈希值是原文语言的 `title`、`text.statement`、`text.reason`、`text.verification` 四个值按这个顺序、以 TOML 解析器返回的形式 (即构建去掉多行字符串引号旁的换行之前，见[源文件格式](source-formats.md))、把文件里的每个 CRLF 行尾当作 LF (这样无论检出时用哪种行尾，哈希都相同；转义写出的 `\r\n` 不是行尾，原样保留)、用 UTF-8 编码，每两个之间以一个 NUL 字节 (0x00) 连接后的 SHA-256；有 applicability 的领域对 `title` 和 `applicability` 计算，连接方式相同，其他领域和 one-of 只对标题计算。原文改了、译文的哈希对不上时，检查失败并打印新的哈希值，就像 `check-design-translation` 对设计文档所做的那样。编译成某种语言时，规定有这种语言的译文就用译文，没有就用原文。

## <a id="rules-and-skills"></a>规定与 skill

规定说明必须达到什么状态；skill 说明怎样做到：步骤、模板、命令、已知的坑 (`skills`)。别人能指着说「你违反了这一条」的内容，写进规定；只是帮 agent 把事情做对的内容，写进 skill。

skill 声明自己落实哪些规定，只有当其中至少一条在规定清单里时才安装。声明写在它 front matter 的 `metadata` 映射里，键是 `statute-rules`，值是它所在规定源里的规定 ID，用空格隔开。

```yaml
---
name: example-skill
description: How to do the thing that rules 42 and 51 require.
metadata:
  statute-rules: "42 51"
---
```

[Agent Skills 规范](https://agentskills.io/specification) 规定 front matter 里只能有 `name`、`description`、`license`、`compatibility`、`metadata` 和 `allowed-tools`，`metadata` 的值只能是字符串 (2026-10-14 读取)；所以声明写成 `metadata` 下的一个字符串，不另设一个键。statute 只从块映射 (block mapping) 里读声明，即上例的写法，键单独占 `metadata:` 下面的一行；`metadata` 写成流式映射 (flow mapping，如 `metadata: {statute-rules: "42"}`) 而其中有这个键的，编译失败，报错指出是哪个 skill、哪一行。没有声明规定的 skill 永远不装；声明了规定源里没有的规定，编译失败。仓库的 `AGENTS.md` 只保留这个仓库特有的内容 (它是什么、用什么语言、目录结构、检查命令)，并指明它使用的规定清单 (`division-of-instructions.project-instructions`)。规定清单按仓库选定 ([规定清单与锁文件](rule-sets.md#rule-sets-and-the-lock-file))。

## <a id="tracing-rules-to-the-constitution"></a>从规定追溯到宪法

依据宪法的每条规定都在 `authority` 字段里引用它所细化的宪法条文 ([规定文件格式](#rule-file-format))。这个关联双向使用：

- 宪法的某条条文修改时，列出引用它的规定，并开一个任务，逐条检查它们是否仍然适用。
- 在网站上，宪法每条条文下面列出引用它的规定，读者由此看到这条条文在实践中落实成了什么。

**兼容的修订号。** 每条规定在 `constitution` 字段里声明自己兼容的宪法修订号范围，写成 Python 的版本说明符，如 `">=1"` 或 `">=1,<4"`。修订号是整数，所以版本说明符的语法只接受比较整数的那一部分：一个或几个子句，用逗号连接，每个子句是 `>=`、`<=`、`>`、`<`、`==`、`!=` 之一加一个修订号，前后可以有空格。`~=1`、`===1`、`==1.*`、`>=1.0`、单独的 `1`、空串和末尾多一个逗号都报错。最低修订号一般都写。最高修订号只在确知这条规定与更高的修订号不兼容时才写，比如宪法废除了这条规定所依赖的内容；不预防性地写上限，Python 打包生态对依赖上限的经验就是如此。宪法的某一条文被修订时，上面列出的那些规定逐条判断：仍然适用的不动；不再适用的加上限，或者在改动影响了怎样才算合规时改写并开新 ID ([ID 与修订号](#ids-and-revisions))。编译时，规定清单钉住的修订号必须落在每条已启用规定的范围内；否则构建失败，并指出是哪条规定、它的范围是什么。
