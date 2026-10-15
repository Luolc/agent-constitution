<!-- en-sha256: 9c3138db662189320bb23d41c6fb8174ab7f4844f83eb8d973badf9a1e1740bb -->
# 规定配置

状态：已定；还没有实现。已经实现的部分见[命令与第一版](statute-command.md#the-command-and-the-first-version)。这部分设计说明规定清单在规定列表之外还能设定的三样东西：仓库自己的规定放在哪个目录，规定留给下层的参数取什么值 (先做语言)，以及哪些领域和规定不必列出就会启用，由它们各自声明的开关决定；此外还有每条规定自己声明的一项：它是给干活的 agent、给审查方，还是两者都给。它以[规定清单与锁文件](rule-sets.md#rule-sets-and-the-lock-file)、[规定文件格式](rule-format.md#rule-file-format) 和[规定源](rule-sources.md) 为基础，也以用户 statute 目录里的[全局配置](global-configuration.md#the-global-configuration) `~/.statute/config.toml` 为基础：全局配置为用户的每份规定清单登记规定源，不启用任何规定。

三样东西的目的相同：采用 statute 的仓库拿到的是一小套最通用、放在哪里都不太会出错的规定，其余的按需开启；只关乎某个人或某种环境的东西，是仓库设定的一个值或开启的一条规定，而不是要它去掉的一条规定。

## <a id="contents"></a>目录

- [本地规定的目录](#the-directory-of-local-rules)
- [参数](#parameters)
- [默认开关](#default-switches)
- [规定用在哪里](#where-a-rule-is-used)
- [第一方规定的推荐默认值](#recommended-defaults-for-the-first-party-rules)
- [提议：规定不挂在规定下面](#proposed-no-rule-under-a-rule)
- [它改变什么](#what-this-changes)

## <a id="the-directory-of-local-rules"></a>本地规定的目录

现在仓库自己的规定放在 `statute.toml` 旁边的 `rules/` 里 ([规定源与引用](rule-sets.md#sources-and-references))。规定清单顶层的 `local-rules` 可以指定另一个目录，相对于 `statute.toml` 所在的目录：

```toml
local-rules = ".repo-rules"
```

- 默认是 `"rules"`，所以没写这个键的规定清单读的还是现在读的那些。
- 只有 `statute.toml` 能设定它。它描述的是仓库的布局，每台机器和 CI 都一样，所以全局配置里没有这个键；`statute.private.toml` 里也没有，因为私有层只增加。
- 这个目录就是某个磁盘上的规定源的 `rules/` 时，比如 `R = { path = "." }` 加上默认值，其中的规定归那个规定源，与现在一样：本地引用其中一条会报错，并提示写成 `R12`。

本仓是第一方规定源，以后也会放只管本仓开发的规定：这些规定约束本仓库，不是给别人用的通用规定。本仓有了规定清单之后，用 `R = { path = "." }` 指向 `rules/` 里的通用规定，用 `local-rules = ".repo-rules"` 指向自己的规定，写作 `L12`。这样它在第一方规定之外还有本地规定，四类规定源 ([四类规定源](rule-sources.md#four-kinds-of-source)) 在它这里都可能出现。[本地网站](statute-command.md#the-local-site) 把在 `.` 的规定源只列一次，归入全局公开，并在本地公开一节里说明仓库自己的规定在那里；改用 `.repo-rules/` 之后，本地公开一节直接列出仓库自己的规定。

## <a id="parameters"></a>参数

讲原则的规定把具体取值留给下面的层 (`rules/101.toml`)。有些取值在每个仓库里都是同一类选择：commit message 用什么语言、代码注释用什么语言、回复人用什么语言。这类取值由规定在正文里写一个参数，由规定清单给出取值，就像 linter 的配置给出 `line-length`。最先做的参数是语言；以后的数字参数，比如审查轮数，也照这个办法。参数不是 `adjustable` 可调值 ([规定](rules.md#rules))：参数的值由规定清单为读它输出的每个 agent 选定，可调值则是仓库或任务的文字可以在宪法许可下放宽的值。

### <a id="declaring-a-parameter"></a>声明参数

参数是规定树上的一个节点，和规定一样在 `rules/` 里占一个文件，写 `kind = "parameter"` (这里的 ID 是编的)：

```toml
id = 9001
slug = "language-commits"
revision = 1
kind = "parameter"
status = "trial"
lang = "en"
parent = 77
name = "language.commits"
type = "language"
default = "en"

[title]
en = "Language of commit messages and pull request titles"
```

- `name`：用点连起来的两个词，每个词是用连字符连起来的小写单词，在规定源的参数里唯一。第一个词是规定清单里设定它的那张表 (见下)，所以不能与规定清单已有的键重名 (`rules`、`packs`、`sources`、`compile` 等)。规定正文和规定清单写的是名字而不是 ID，这样两处读起来都直接说明了意思。
- `type`：取以下之一
  - `language`：statute 知道名称的语言代码 (目前是 `en` 和 `zh`)；
  - `string`：任意非空的文字，只占一行，两端没有空格，也不含反引号。字符串是名字，所以由规定自己把占位符放进代码片段 (`` `{$work-items.label-approved}` ``)，statute 填值时不另加标记；
  - `integer`，等第一个数字参数出现时再加。

  构建按类型检查 `default` 和规定清单设定的每个取值，不符时失败，并指出是哪个参数、哪个值。statute 不检查字符串是否指向真实存在的东西，比如 issue 跟踪系统里的某个标签。
- `default`：规定清单没有设定时得到的值。这个键在各种节点上意思相同，都是规定清单什么都不说时得到的东西：这里是一个值，在领域或规定上是开关，在 one-of 上是它的一个子节点 ([默认开关](#default-switches))。
- `parent`：一个领域，参数在网站上列在它下面。除此之外参数不属于任何领域：任何领域的规定都可以用它。
- `[title]` 说明这个值管什么，与领域的标题一样翻译，哈希记在 `[translations]` 里 ([规定文件格式](rule-format.md#rule-file-format))。

参数没有正文、检查、`requires`，也没有默认开关。它从不单独启用：有启用的规定用到它，它就在规定清单里。

参数的标题或默认值改动时原地修改，修订号加一。新的默认值确实会改变 agent 收到的要求，但只在没有设定取值的规定清单里；锁文件记下了取值，所以 `upgrade` 会显示这一变化 (见下)。

每个参数只在自己的文件里声明一次，这是在另外两个位置之间比较后选的。在用到它的每条规定里声明，默认值要每条规定写一遍，还可能写得不一致。在领域里声明，它就只能放在一个领域里，而语言是跨领域用的：commit message 在 Git 下面，回复人在协作下面。

### <a id="placeholders"></a>占位符

规定正文用 `{$<name>}` 写一个参数：

```toml
[text.statement]
en = "Write commit messages and pull request titles in {$language.commits}."
zh = "commit message 与 pull request 标题用{$language.commits}写。"
```

- 这个写法与引用的 `{@...}` 同属一类 ([ID](source-formats.md#ids))，也不会与 `{{ }}` 冲突：Hugo 模板用 `{{ }}`，而网站的数据会经过 Hugo。
- 规定只用自己规定源和第一方规定源的参数，只写名字。同一份规定清单里有两个规定源声明了同一个名字，构建失败，并指出这两个规定源。
- 占位符可以出现在规定的标题和正文的任何部分，别处都不行。写了一个不存在的参数，构建失败，与引用了不存在的规定一样。
- 每份译文含有与原文相同的占位符，次数也相同；构建像检查引用那样检查这一点。占位符从不翻译，翻译的是它周围的文字。
- `[translations]` 里的哈希按原文的写法算，包括占位符。设定取值不碰任何规定文件，所以不改变任何哈希。

### <a id="setting-a-value"></a>设定取值

规定清单在参数名第一个词那张表里设定参数：

```toml
[language]
communication = "zh"
docs = "zh"
```

- 只有 `statute.toml` 设定取值，`statute.private.toml` 一个也不设：它设的值会让全文里某条公开规定的文字与提交的 `AGENTS.md` 里的不一样，而私有层只增加 ([公开仓库里的私有规定](rule-sources.md#private-rules-in-a-public-repository))。
- 键不对应规定清单任何规定源的参数，报错；值的类型不对，或者是 statute 不知道名称的语言代码，也报错。为没有启用规定在用的参数设了值，给一个警告；这样关掉相关规定时，值可以留在原处。
- [全局配置](global-configuration.md#the-global-configuration) 里也可以有同样的表。编译规定清单时不读其中的值：`statute init` 把它们写进新的 `statute.toml`，等用户级输出实现之后，它们就是那份输出的取值 ([规定清单与锁文件](rule-sets.md#rule-sets-and-the-lock-file)，「放在哪里」一段)。这样，提交的规定清单和锁文件单独就能说明编译出的 `AGENTS.md` 里有什么，没有全局配置的 CI 也能构建出同一个文件。

规定清单得到的取值按顺序取第一个有的：它的 `statute.toml`，参数的 `default`。

**语言参数**共六个，默认都是 `en`：

| 名字 | 管什么 |
| --- | --- |
| `language.communication` | agent 写给人的话：回复、提问、汇报 |
| `language.code-comments` | 代码里的注释 |
| `language.docs` | 仓库里的文档：Markdown、设计和其他文字文件 |
| `language.commits` | commit message 与 pull request 标题 |
| `language.pull-requests` | pull request 描述、pull request 里的评论和审查 |
| `language.work-items` | issue 和其他工单 |

commit message 与 pull request 描述分成两个参数，是因为常见的做法是前者用英文，方便查历史、方便工具处理，后者用读这个仓库的人的语言。代码本身，即标识符、文件名和属于接口的字符串，没有参数：一律用英文，这一点已经定了。

**issue 跟踪系统里的名字。** 关于工单的规定会写到跟踪系统的标签和状态名，这些名字因系统、因团队而异。计划在领域 R14 下设四个 `string` 参数，默认值都是常见的名字：

| 名字 | 默认 | 管什么 |
| --- | --- | --- |
| `work-items.label-needs-decision` | `needs-user` | 等人来看或来定的工单的标签 |
| `work-items.label-approved` | `approved` | 人已批准的工单的标签 |
| `work-items.status-done` | `Done` | 以完成告终的一次尝试的状态 |
| `work-items.status-abandoned` | `Abandoned` | 被放弃的一次尝试的状态 |

参数实现之前，用到它们的规定把名字和默认值写成纯文字「(parameter `work-items.label-needs-decision`, default `needs-user`)」，实现参数的那次改动再把它换成占位符。表 `work-items` 与键 `language.work-items` 不冲突：一个是规定清单的表，一个是表 `language` 里的键。

`[compile] lang` 与语言参数分开：它指定编译出的 `AGENTS.md` 用哪种语言写，关乎 agent 读什么；参数说的是 agent 写什么。

### <a id="values-in-the-lock-file-and-the-outputs"></a>锁文件与输出里的取值

**锁文件**为启用规定用到的每个参数记一行，与规定一样，另外记下规定清单得到的取值：

```toml
[[rules]]
rule = "R9001"
revision = 1
sha256 = "<SHA-256 of rules/9001.toml>"
value = "zh"
```

规定清单里的取值或参数的默认值变了，锁文件就过期了，与规定改动时一样；`upgrade` 列出这个参数及其新旧取值。

**编译出的 `AGENTS.md`** 用取值替换占位符，不加任何标记：语言写成它在文件所用语言里的名称 (`Chinese`、`中文`)，字符串和数字照写。规定的标题和要求读起来就像取值本来就写在规定里。

**`statute explain <name>`** 说明取值、它从哪里来 (规定清单还是默认值)，以及哪些启用的规定在用它。

**公开网站**把占位符显示为它的默认值，标成一个取值，鼠标悬停时显示参数名，并链接到参数的页面：标题、名字、类型、默认值，以及用到它的规定。规定列表把参数列在它的领域下面，与规定一样。**本地网站**以同样的方式显示它那份规定清单的取值，并在参数页面上写明取值从哪里来。

## <a id="default-switches"></a>默认开关

每个领域和每条规定都声明自己是否默认开启。规定清单什么都不列，就得到开启的领域里开启的规定；它只列出与此不同的地方。

### <a id="in-the-rule-files"></a>规定文件里

每个领域和每条规定都必须写 `default`：

```toml
id = 78
kind = "domain"
default = "off"
```

```toml
id = 44
kind = "rule"
default = "on"
```

- 取 `on` 或 `off`。参数的 `default` 是它的取值 ([声明参数](#declaring-a-parameter))，有规定用到参数时，它就在规定清单里。one-of 的子节点没有 `default`；由 one-of 指定其中一个作为默认 (见下)。
- 只适用于部分仓库或部分工作方式的领域是 `off`，比如 Fleet 只适用于在多个 agent 之间分工的工作：规定清单开启它之前，它的规定一条也不进来。子领域的 `default` 在它的领域开启之后才起作用。
- 一条规定只有属于最通用、放在哪里都最不容易出错的那一类时才是 `on`。依赖某个人的偏好或环境 (共用的机器、一个审查方、某种 CI 服务、某个工具) 的规定是 `off`。
- 规定源的默认值开启的规定必须能单独解析：在这个规定源之内，`on` 规定的 parent (是规定时) 和它依赖的每条规定都是 `on`；`on` 规定所在的领域是 `on` 时，它不依赖任何 `off` 领域里的东西。构建会检查这一点。

**one-of 的默认。** 每个 one-of 都在一个必填字段里指定它的一个子节点作为默认：

```toml
id = 79
kind = "one-of"
parent = 16
default = 81
```

- `default` 是这个 one-of 一个子节点的 ID；写成别的节点，构建失败。
- one-of 的 parent 开启 (是领域时该领域开启，是规定时该规定启用)，one-of 就开启：它启用默认的子节点，除非规定清单选了另一个子节点，或者把这个 one-of 关掉 (见下)。这样仓库开启一个领域，每个 one-of 都有一个能用的选择，不用自己去选，只改它在意的那几处。
- 只有部分仓库需要的 one-of，放在一个 `off` 的领域或子领域里，由需要的仓库开启。
- 默认子节点带进来的规定也必须能单独解析，与 `on` 规定一样 (见上)。

改动规定、领域或 one-of 的 `default` 不改变什么算合规，所以是原地修改，修订号加一 ([ID 与修订号](rule-format.md#ids-and-revisions))。`upgrade` 列出因此进来或退出的规定，与列出依赖带进来的规定一样。

### <a id="in-the-rule-set"></a>规定清单里

规定清单只列出与默认值不同的地方：

```toml
rules = ["R57", "R62"]
domains = ["R78"]
exclude = ["R83", "R66"]
packs = ["R96"]
```

- `rules` 与现在一样，开启一条规定。对 `off` 的规定或 `off` 领域里的规定，开启它们靠的就是这一项；列出默认值已经开启的规定，给一个警告。列出 one-of 的一个子节点，就用它代替 one-of 的默认。
- `domains` 开启一个 `off` 的领域：其中的规定按各自的 `default`。列出一个 `on` 的领域，给一个警告。
- `exclude` 关掉一个默认开启的领域、one-of 或规定。关掉一个节点，就关掉它的整棵子树；关掉的 one-of 一个子节点也不启用。被某条启用的规定依赖的规定不能关：构建失败，并指出依赖它的那条规定。
- `packs` 与现在一样，开启整棵子树里的每条规定，不论它的 `default`；但在 one-of 处只开一个子节点：规定清单在 `rules` 里点名的那个，没有点名就是默认的那个，规定包只进入这个子节点的子树。`exclude` 可以关掉其中一部分。
- 一条规定依赖某个 one-of，而规定清单没有启用这个 one-of 的任何子节点时，带进来的是默认子节点，与 statute 补上任何缺失的依赖一样，不再报错。
- `statute add` 把领域写进 `domains`，把规定写进 `rules`；`statute remove` 从 `rules`、`domains` 或 `packs` 里去掉一项，这条规定或这个领域仍然默认开启时，把它写进 `exclude`。
- `statute.private.toml` 可以有 `rules` 和 `domains`，它们只增加；不能有 `exclude`：私有层不去掉公开层的任何规定 ([公开仓库里的私有规定](rule-sources.md#private-rules-in-a-public-repository))。

**取谁的默认值。** 规定清单取第一方规定源 `R` 的默认值，不论 `R` 从哪里来；也取它自己的 `[sources]` 写了的每个规定源的默认值，即 `statute.toml` 里写的，私有层则是 `statute.private.toml` 里写的。只有全局配置登记的规定源不带进任何默认规定；要用它的规定就逐条列出，或者在规定清单里写上这个规定源，取它的默认值。否则，往全局配置里加一个规定源，就会在每个仓库里开启规定，而这些仓库的规定清单里没有任何一处写明。规定源条目写 `defaults = false`，就不取这个规定源的任何默认值：

```toml
[sources]
team = { git = "https://example.test/our-team/rules.git", branch = "main", public = true, defaults = false }
```

`defaults` 只是规定清单里规定源条目的键：写在全局配置里会报错，因为全局配置不启用任何规定。

`statute init` 不写 `rules`、`domains` 或 `exclude`：新的规定清单就是第一方的默认值。

### <a id="defaults-in-the-lock-file-and-the-outputs"></a>锁文件与输出里的默认值

**锁文件**与现在一样记下每条启用的规定。只因默认值而启用的规定，在它那一行加 `by = "default"`，这样锁文件能说明哪些规定不是选进来的，`upgrade` 也能列出因 `default` 改动而进来或退出的规定。

**`statute explain <rule>`** 对一条规定或一个领域，说明它是否默认开启，以及规定清单为什么有它或没有它：「R44 默认开启，它的领域 R13 也开启」；「R66 默认开启，被 `exclude` 关掉」；「R78 默认关闭；`statute add R78` 开启它」。对 one-of，它指出默认的子节点，规定清单选了别的子节点时也指出选的是哪个。`statute explain --defaults` 为规定清单打出三张列表：它保留的默认开启的规定、它关掉的默认开启的规定、它开启的默认关闭的规定。

**编译出的 `AGENTS.md`** 不变：它收录启用的规定，不论它们怎么进来的。

**公开网站**在列表和详情页上把每个领域和每条规定标成默认开启或默认关闭，标出每个 one-of 的默认子节点，列表加一个只显示默认开启规定的筛选，让读者一眼看出一份新的规定清单里有什么。**本地网站**在它那份规定清单是否启用这条规定的旁边，显示同样的这些标记。

### <a id="moving-a-repository-over"></a>把仓库迁过来

仓库迁到 statute 上来，不论是从手写的指南迁，还是从默认开关出现之前写的规定清单迁，都从默认值出发，过三张列表；规定清单写好之后，`statute explain --defaults` 会把它们打出来：

1. 仓库保留的默认开启的规定：什么都不用做；
2. 它必须关掉的默认开启的规定：写进 `exclude`，每项用一行注释说明原因；
3. 它需要的默认关闭的领域和规定：写进 `domains` 和 `rules`；某个 one-of 要选默认以外的子节点时，也写进 `rules`。

默认开关出现之前写的规定清单，把用到的每条规定都列在 `rules` 和 `packs` 里；按默认值，它还可能得到从没列过的规定。默认开关上线后的第一次 `lock` 会列出它们，迁移时把每一条放进第二张列表，或者保留。指导迁移的 skill (还没有写) 按这个顺序过这三张列表。

## <a id="where-a-rule-is-used"></a>规定用在哪里

一条规定有两类读者：干活的 agent 读编译出的 `AGENTS.md`，审查方过 `statute check` 打出的清单 ([结果与审查回路](rule-checks.md#outcomes-and-the-review-loop))。不是每条规定两边都用得上。「不在 agent 之间传递凭证」(`rules/45.toml`) 是 agent 干活时必须知道的，但手里只有 diff 和 pull request 描述的审查方判断不了它有没有被违反。「描述里写的 head 就是审查的 head」这类要求则只对审查方有用。大部分规定根本不必进审查。

**规定文件里**用 `uses` 列出规定去哪里，取下面两个值的非空子集：

```toml
uses = ["instructions", "review"]
```

- `instructions`：`statute compile` 把这条规定写进编译出的 `AGENTS.md`。
- `review`：`statute check` 把这条规定连同理由和核验方法列给审查方。
- 没写 `uses` 的规定取 `["instructions"]`；要 `review` 就写出来。空列表报错，这两个以外的值也报错。领域、one-of 和参数没有 `uses`。

`uses` 与 `check` 是两回事。`check` 说的是由程序怎样检查这条规定：`builtin`、`command`，或者 `review`，即没有程序 ([规定文件格式](rule-format.md#rule-file-format))。`statute check` 运行每条有程序的启用规定的程序，不论它的 `uses`；列给审查方的只有 `uses` 含 `review` 的规定。这样 `check = "review"` 只表示「没有自动检查」；改名为 `none` 能把这一点说清楚，留给实现 `uses` 的那次改动。改 `uses` 不改变什么算合规，所以原地修改，修订号加一。

**怎么选。** 提议给领域 R96 加一条规定，与给规定加 `uses` 的那次改动一起写：只有凭 diff 和 pull request 描述就能判断有没有违反的规定，才标 `review`；只有 agent 干活时需要知道的规定，才标 `instructions`。

**由此而来的。**

- 编译出的 `AGENTS.md` 收录 `uses` 含 `instructions` 的启用规定。一个领域里启用的规定全都只进审查时，那里没有它的标题；对只进审查的规定的依赖或引用，按引用文件之外的规定渲染，即它的标题后面加「(规定 R42)」。
- `statute check` 在各个领域里列出 `uses` 含 `review` 的启用规定，以及它运行的每个程序的结果。
- `statute explain <rule>` 说明一条启用的规定去哪里：「R43 编进 `AGENTS.md`，并列给审查方」。
- 锁文件不记新东西：`uses` 是规定文件的一部分，锁文件已经用 SHA-256 钉住了它。
- 网站在规定详情页显示 `uses`，规定列表加一个按它筛选的选项。

## <a id="recommended-defaults-for-the-first-party-rules"></a>第一方规定的推荐默认值

第一方规定还没有 `default`。下面的表是推荐值，给这些规定加这个字段的那次改动照它来；从那以后以规定文件为准，这一节删掉；`uses` 也一样。它覆盖 2026-10-14 主分支上的规定，到 R126 为止；被取代的规定不列，因为它们从不会启用。一条规定没有列在它的领域下面，推荐值就是 `on`。

| 领域 | 默认 | 理由 |
| --- | --- | --- |
| R13 安全 | on | 凡是有 agent 工作的地方 |
| R14 协作 | on | 凡是 agent 替人做事的地方；其中审查和工单的规定是 off |
| R15 验证 | on | 凡是 agent 改动有检查的仓库的地方 |
| R16 文档 | on | 凡是 agent 写文档的地方 |
| 设计，R16 下新建的子领域 | off | 只用于有设计文档的仓库 (见下) |
| R77 Git | on | agent 提交的仓库几乎都是 Git 仓库 |
| R78 Fleet | off | 只用于多个 agent 分工的工作 |
| R83 代码 | on | 凡是 agent 写代码的地方；其中范围较窄的测试规定是 off |
| R96 编写规定 | off | 只用于编写规定的仓库 |
| 语音输入 (`voice-input`)，尚未建立：由之后把规定搬进去的改动建立，ID 届时分配 | on | agent 怎样读可能来自语音转文字的消息；已定为公开、默认开启 |

在 `on` 的领域里推荐为 `off` 的规定，按上面的判据 (某个人的偏好，或者某种环境)：

| 规定 | 为什么 off |
| --- | --- |
| R30 不在共用机器上起负载发生器 | 环境：共用的机器 |
| R31 在隔离环境里复现高负载问题 | 环境：共用的机器 |
| R52 发往公开仓库的内容由审查方先读 | 环境：一个审查 agent |
| R93 经 shell 传的文字只从带引号的 heredoc 文件传 | 关于怎样调命令的偏好 |
| R94 拿发出的文字与原文件核对 | 它的 parent R93 是 off |
| R116 到 R121：找工作、领工单 | 环境：一个存放工单的地方 |
| R57 改动只经审查过的 pull request 进主分支 | 偏好：一个人的仓库可以直接提交 |
| R58 只发布 CI 第一次就通过的提交 | 环境：主分支上的 CI |
| R59 合并后看合并提交的 CI | 环境：主分支上的 CI |
| R62 不合并自己的改动 | 关于谁来合并的偏好 |
| R63 审查放在改动的评论里 | 关于审查放在哪里的偏好 |
| R64 三轮审查 | 偏好：审查轮数 |
| R65 检查缺工具时什么都不装 | 环境：别人共用或管理的机器 |
| R71 不把改动设成自动合并 | 偏好 |
| R95 主分支 CI 红着时照常合并 | 偏好 |
| R25 到 R27 条款的测试数据 | 环境：为宪法保留测试数据的仓库 |
| R46 到 R49 指令文件单独成一次改动 | 关于怎样审查指令改动的偏好 |
| R56 对照 head 回读描述 | 关于怎样维护 pull request 的偏好 |
| R72 引用 CI 的那次运行，不以本地全量运行代替 | 关于在本地跑全量的偏好 |
| R69 不往 agent 工具自己的记忆里写 | 偏好 |
| R50 commit 钩子扫描暂存区里的凭证 | 环境：装好了扫描工具、配好了钩子 |
| R84 到 R86 范围较窄的测试规定 | 关于写哪些测试的偏好 |
| R88 注释只写代码说不出的东西 | 关于注释多少的偏好 |

`off` 的领域 (R78、R96) 里每条规定都是 `on`：规定清单开启这样的领域，就整个拿过来。语音输入领域里各条规定的默认开关，等它建立之后再定。

**设计单独成一个子领域。** 关于仓库设计的规定 (R19、R21、R112 到 R114、R122 到 R125，以及 one-of R79 和它的子节点 R80、R81) 只关乎有设计文档的仓库。推荐：在 R16 下新建子领域「设计」，`off`，把它们都放进去，每条在里面是 `on`；有设计文档的仓库只需开启这个子领域。搬过去就是改它们的 `parent`，与加 `default` 的那次改动一起做。

**one-of。** R79「让设计与代码保持一致」是目前唯一的 one-of。推荐默认 R81，即代码和设计可以分开改、每处差异都标出来：设计写在代码之前还是之后它都适用，而 R80 要求每次改动都带上设计。

**uses。** 按上面提议的规定，推荐为 `["instructions", "review"]` 的规定：R1、R19、R21、R43、R46、R47、R51、R56、R72、R75、R80、R81、R84 到 R89、R92、R97 到 R106、R108 到 R114、R122 到 R126。没有推荐只进审查的：这些规定也都是 agent 干活时需要知道的。其余的保持默认 `["instructions"]`。

## <a id="proposed-no-rule-under-a-rule"></a>提议：规定不挂在规定下面

待人决定。决定之前，[依赖、one-of 与规定包](rule-sets.md#dependencies-one-ofs-and-packs) 和[规定文件格式](rule-format.md#rule-file-format) 里的树照旧，上面的默认开关也是按它写的；提议一旦采纳，由一次改动把这两部分设计改过来。

**提议的内容。**

- 规定的 parent 是领域或 one-of，不再是规定。现在「规定挂在规定下面」表达的意思，即细化另一条规定，改用 `requires` 写。
- one-of 的 parent 是领域。
- one-of 的选项就是它的子节点，要么全是规定，要么全是领域，不混用；构建会检查。只有一条规定的选项就是那条规定，不必外包一个领域；要带几条规定的选项是一个以 one-of 为 parent 的领域，把这些规定放在里面；它是启用的那个选项时，其中的规定按各自的 `default` 进来，与 `domains` 开启的领域一样。领域仍然最多嵌套两级，这样的领域也算一级：子领域里的 one-of 只能以规定为选项。
- 每个 one-of 都指定一个选项作为默认 ([one-of 的默认](#in-the-rule-files))，这一点上面已经定了。
- 选项在自己的 `parent` 里指向它的 one-of，与现在 one-of 的子节点一样；one-of 不列 `options`。这样树仍然只记在 `parent` 里 ([规定文件格式](rule-format.md#rule-file-format))，加一个选项或挪一个选项只改一个文件，现有规定也不用为此改动。one-of 自己的文件里看不到它的选项，由 `statute explain` 和网站列出。
- 编译出的 `AGENTS.md` 里，一个领域的规定排在它的标题下一级；one-of 启用的那个选项放在 one-of 的位置上，加一行写明它属于哪个 one-of (「One of: Keep the design in step with the code.」)；是领域的选项保留自己的标题。网站上，one-of 是它领域列表里的一个框，列出每个选项并标出默认；选项只出现在框里，不在列表里重复。

**为什么推荐。** 读编译出的文件或网站的人，看到的每条规定都直接在一个领域或一个 one-of 下面，没有规定套在规定下面；只有一条规定的选项也不必单独包一个领域。规定的 parent 算作依赖这个特例随之消失，依赖穿过 one-of 传到 one-of 自己的 parent 也一样，因为那个 parent 现在总是领域。失去的是表示「一条规定细化另一条」的缩进；这层关系由 `requires` 保留，网站在两条规定上都显示它。

**它改变什么。** 要搬的规定只有一条：R94，它的 parent 是 R93；parent 改成 R13，并依赖 R93。one-of R79 的子节点 R80 和 R81 保持不变。`constitution-build` 检查没有规定以规定为 parent、one-of 的 parent 是领域、它的子节点全是规定或全是领域。statute 不再把 parent 当作依赖。锁文件不变。这些都还没有实现。

## <a id="what-this-changes"></a>它改变什么

还没有实现：本文件里的一切。statute 不读 `local-rules`、`domains`、`exclude`、`defaults` 和 `[language]` 表，也不读 `kind = "parameter"`、节点上的 `default`、`uses` 和占位符，`constitution-build` 也一样都不检查；现在规定清单和规定文件遇到这些键会报错。第一方规定目前没有参数，也没有 `default`。

受这一部分影响的其他设计：

- [选用规定](rules.md#choosing-rules)：法域使用全部通用规定、逐条点名去掉的推荐做法，让位于默认开关。
- [本地参数](rules.md#local-parameters)：参数是规定源里的节点，占位符是 `{$<name>}`，取值来自规定清单，而不是 overlay 的 `params.toml`。
- [规定清单与锁文件](rule-sets.md#rule-sets-and-the-lock-file)：规定清单多了 `local-rules`、`domains`、`exclude` 和参数的表，规定源条目多了 `defaults`，锁文件多了 `by` 和 `value`。
- [规定文件格式](rule-format.md#rule-file-format)：多了 `parameter` 这一种节点，领域、规定和 one-of 多了 `default`，规定多了 `uses`。
- [结果与审查回路](rule-checks.md#outcomes-and-the-review-loop)：`statute check` 只把 `uses` 含 `review` 的规定列给审查方，每条规定的程序照旧运行。
- [编译与网站](statute-command.md#compilation-and-the-site)：编译出的 `AGENTS.md` 只收录 `uses` 含 `instructions` 的规定。
- [依赖、one-of 与规定包](rule-sets.md#dependencies-one-ofs-and-packs)：规定清单没有选时，one-of 启用它的默认子节点。那里的「缺的依赖自动补上」一段说，依赖一个没有启用任何子节点的 one-of 会报错；「规定包」一段说，规定清单没有点名子节点的 one-of 不产生任何内容；两处都改为取默认子节点。
