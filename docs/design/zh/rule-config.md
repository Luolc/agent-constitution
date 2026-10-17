<!-- en-sha256: 14c8ae86a6fdc19407fe8c6beab1c49281f3246b466b5ca0cde163f3f9f93f1c -->
# 规定配置

状态：已定；已实现。已经实现的部分见[命令与第一版](statute-command.md#the-command-and-the-first-version)，[它改变什么](#what-this-changes) 一节也列出了它们。这部分设计说明规定清单在规定列表之外还能设定的三样东西：仓库自己的规定放在哪个目录，规定留给下层的参数取什么值 (先做语言)，以及哪些领域和规定不必列出就会启用，由它们各自声明的开关决定；此外还有每条规定自己声明的一项：它是给干活的 agent、给审查方，还是两者都给。它也定下规定树的形状：规定不挂在规定下面。它以[规定清单与锁文件](rule-sets.md#rule-sets-and-the-lock-file)、[规定文件格式](rule-format.md#rule-file-format) 和[规定源](rule-sources.md) 为基础，也以用户配置目录里的[全局配置](global-configuration.md#the-global-configuration) `$XDG_CONFIG_HOME/statute/config.toml` (默认是 `~/.config/statute/config.toml`) 为基础：全局配置为用户的每份规定清单登记规定源，开启的规定只进私有输出。

三样东西的目的相同：采用 statute 的仓库拿到的是一小套最通用、放在哪里都不太会出错的规定，其余的按需开启；只关乎某个人或某种环境的东西，是仓库设定的一个值或开启的一条规定，而不是要它去掉的一条规定。

## <a id="contents"></a>目录

- [本地规定的目录](#the-directory-of-local-rules)
- [参数](#parameters)
- [默认开关](#default-switches)
- [规定用在哪里](#where-a-rule-is-used)
- [规定不挂在规定下面](#no-rule-under-a-rule)
- [它改变什么](#what-this-changes)

## <a id="the-directory-of-local-rules"></a>本地规定的目录

仓库自己的规定 `L12` 放在 `statute.toml` 旁边的 `.statute/rules/` 里 (见[四类规定源](rule-sources.md#four-kinds-of-source))。规定清单顶层的 `local-rules` 可以指定另一个目录，相对于 `statute.toml` 所在的目录：

```toml
local-rules = "policy/agents"
```

- 默认是 `".statute/rules"`。仓库顶层的 `rules/` 不当作仓库自己的规定读：它属于某个规定源，只有某个规定源就在那个目录时才读，比如 `R = { path = "." }`。`rules/` 这个名字太常见，statute 不该在用到它的仓库里占用它；放进 `.statute/`，这些规定就和 statute 的其他文件在一起，就像某个工具要提交的文件放在 `.claude/` 或 `.github/workflows/` 里一样。`.statute/rules/` 要提交；`.statute/` 只把 statute 生成的文件列为忽略 (见[公开仓库里的私有规定](rule-sources.md#private-rules-in-a-public-repository))。
- 这里的规定，以及 `scope = "local"` 的源里的规定，只为本规定清单而写，所以没有被任何东西启用的这类规定多半是疏漏，带 `overrides` 的尤其如此，因为它没有 `default`。statute 对它给出警告，点名这条规定，以及能把它编译进来的那一项 `rules`。已被取代或废止的规定、`exclude` 关掉的规定，以及 one-of 的选项，不在此列。公开仓库有私有规定清单时，这条警告由私有层给出，因为它能看到两层；警告点名的是能列出这条规定的那份规定清单：私有层添加的源里的规定是 `private.toml`，其余的是 `statute.toml`。
- 只有 `statute.toml` 能设定它。它描述的是仓库的布局，每台机器和 CI 都一样，所以全局配置和私有规定清单里都没有这个键。
- 磁盘上的规定源，比如 `R = { path = "." }`，仍从 `<path>/rules/` 或者它的 `rules-dir` 指定的目录读 (见[规定源与引用](rule-sets.md#sources-and-references))：那些是规定源的规定，不是本地规定。`local-rules` 指向同一个目录时，其中的规定归那个规定源：本地引用其中一条会报错，并提示写成 `R12`。
- 这个目录里可以有 `params.toml`，即其中参数的声明 (见[参数](#parameters))；statute 不把它当作规定读。

本仓是第一方规定源，也放只管本仓开发的规定：这些规定约束本仓库，不是给别人用的通用规定。本仓有了规定清单之后，用 `R = { path = "." }` 指向 `rules/` 里的通用规定，自己的规定 `L12` 按默认放在 `.statute/rules/`。这样它在第一方规定之外还有本地规定，四类规定源在它这里都可能出现。[本地网站](statute-command.md#the-local-site) 把第一方规定列为全局公开，把本仓自己的规定列在本地公开一节。

## <a id="parameters"></a>参数

已经实现。

讲原则的规定把具体取值留给下面的层 (`rules/101.toml`)。有些取值在每个仓库里都是同一类选择：commit message 用什么语言、代码注释用什么语言、回复人用什么语言。这类取值由规定在正文里写一个参数，由规定清单给出取值，就像 linter 的配置给出 `line-length`。最先做的参数是语言、issue 跟踪系统里的几个名字和放主检出的目录。参数不是 `adjustable` 可调值 ([规定](rules.md#rules))：参数的值由规定清单为读它输出的每个 agent 选定，可调值则是仓库或任务的文字可以在宪法许可下放宽的值。

### <a id="declaring-a-parameter"></a>声明参数

每个规定源在一个文件里声明自己的参数，即它规定目录里的 `params.toml`：本仓是 `rules/params.toml`；仓库自己的规定则是本地规定目录里的 `params.toml` (见[本地规定的目录](#the-directory-of-local-rules))。statute 不把这个文件当作规定读；没有这个文件的规定源不声明参数。每个参数是数组 `parameter` 里的一张表：

```toml
[[parameter]]
name = "language.commits"
type = "language"
scope = "repo"
default = "en"

[parameter.title]
en = "Language of commit messages and pull request titles"
zh = "commit message 与 PR 标题的语言"

[parameter.translations]
zh = "<SHA-256 of the English title>"
```

- `name`：用点连起来的两个词，每个词由小写字母和数字组成、用连字符连接，在规定源的参数里唯一。第一个词是规定清单里设定它的那张表 (见下)，所以不能是规定清单已有的键：`constitution`、`public`、`rules`、`packs`、`sources`、`compile`、`domains`、`exclude`、`local-rules` 或 `base`。规定正文和规定清单写的都是名字，这样两处读起来都直接说明了意思。
- `type`：取以下之一
  - `language`：statute 知道名称的语言代码 (目前是 `en` 和 `zh`)；
  - `string`：任意非空的文字，只占一行，两端没有空格，也不含反引号。字符串是名字，所以由规定自己把占位符放进代码片段 (`` `{$work-items.label-approved}` ``)，statute 填值时不另加标记；
  - `choice`：`values` 列出的字符串之一。`values` 只在这个类型下有，而且必填：不能为空，不能有重复，每个值都要满足上面 `string` 的要求。`language` 就是取值由 statute 给定的 `choice`。

  数字类型等第一个需要它的参数出现时再加。statute 按类型检查 `default` 和规定清单设定的每个取值，不符时失败，并指出是哪个参数、哪个值。它不检查字符串是否指向真实存在的东西，比如 issue 跟踪系统里的某个标签。
- `scope`：`repo` 或 `personal`，必填；它决定由谁设定取值 ([设定取值](#setting-a-value))。
- `default`：规定清单没有设定时得到的值。这个键与规定树节点上的同名键意思相同，都是规定清单什么都不说时得到的东西：这里是一个值，在领域或规定上是开关，在 one-of 上是它的一个选项 ([默认开关](#default-switches))。
- `[title]` 说明这个值管什么。参数没有 `lang`：它的原文是 `[title]` 里 `[translations]` 没有列出的那一种语言，必须恰好一种。`[translations]` 记原文标题的 SHA-256，与没有适用范围的领域一样 ([规定文件格式](rule-format.md#rule-file-format))。

写了别的键就失败。取值来自固定列表的参数是这样的：

```toml
[[parameter]]
name = "python.quote-style"
type = "choice"
values = ["double", "single"]
scope = "repo"
default = "double"

[parameter.title]
en = "Quotes around strings in Python code"
```

参数没有 ID、修订号、状态，也没有 parent：它不是规定树上的节点，从不单独启用，在锁文件的规定列表里也没有一行。有启用的规定用到它，它就在规定清单里。改参数的标题或默认值不碰任何规定文件。新的默认值确实会改变 agent 收到的要求，但只在没有设定取值的规定清单里；锁文件记下了取值，所以 `upgrade` 会显示这一变化 (见下)。

每个规定源一个文件，这是与另外三个位置比较后选的。如果参数是规定树上的节点，它就会有 ID、修订号、状态和一个 parent 领域，这些都是为要启用、被引用、会被取代的规定准备的，参数一样也用不上；它的领域也只能是树上的一处，而语言是跨领域用的：commit message 在 Git 下面，回复人在协作下面。在用到它的每条规定里声明，默认值要每条规定写一遍，还可能写得不一致。写在领域文件里，它又只能放在一个领域里。一个文件把一个规定源的参数放在一起，读者在一处就能看全。

### <a id="a-parameter-or-a-one-of"></a>参数还是 one-of

两种做法只差填进同一句话里的一个值，就是带一个参数的一条规定；要求本身不同，句子、做法或检查不同，就是一个 one-of 的几个选项，每个选项是一条完整的规定 (`rules/230.toml`)。参数只做替换：规定正文从不按它的值分叉 (「是 A 就这样做，是 B 就那样做」)，statute 也不在规定正文里支持条件或表达式。需要分叉的正文拆成一个 one-of。

`choice` 类型跟的是一些 linter 的做法：把规则和规则的设置分开。ruff 按代码选规则，另外设 `flake8-quotes.inline-quotes`；ESLint 给规则带选项，`quotes: ["error", "double"]`。规定说的是什么必须成立；设置说的是它在几个值里取哪一个。

### <a id="placeholders"></a>占位符

规定正文用 `{$<name>}` 写一个参数：

```toml
[text.statement]
en = "Write commit messages and pull request titles in {$language.commits}."
zh = "commit message 与 PR 标题用{$language.commits}写。"
```

- 这个写法与引用的 `{@...}` 同属一类 ([ID](source-formats.md#ids))，也不会与 `{{ }}` 冲突：Hugo 模板用 `{{ }}`，而网站的数据会经过 Hugo。
- 规定只用自己规定源和第一方规定源的参数，只写名字。两者声明了同一个名字就失败，并指出两处。
- 占位符只能出现在规定的标题和正文的三个部分里，别处都不行：出现在领域的标题或适用范围、one-of 的标题或参数的标题里，都会失败。名字不是参数名的形状，或者没有这个参数，也失败，与引用了不存在的规定一样。
- 每份译文含有与原文相同的占位符，次数也相同；statute 像检查引用那样检查这一点。占位符从不翻译，翻译的是它周围的文字。
- `[translations]` 里的哈希按原文的写法算，包括占位符。设定取值不碰任何规定文件，所以不改变任何哈希。

### <a id="setting-a-value"></a>设定取值

规定清单在参数名第一个词那张表里设定参数：

```toml
[language]
communication = "zh"
docs = "zh"
```

参数的 `scope` 决定由谁设定：

- **`repo`**：进入仓库的东西，对在仓库里工作的每个人都一样，比如仓库文档或 commit message 用的语言。只有 `statute.toml` 设定它。全局配置和私有规定清单 `private.toml` ([公开仓库里的私有规定](rule-sources.md#private-rules-in-a-public-repository)) 不能设定：在那里设了这类值，给一个警告，指出参数和文件，并忽略这个值。
- **`personal`**：只关乎 agent 为之工作的那个人，不论 agent 在哪个仓库工作：目前是 agent 写给这个人时用的语言，它为这个人写的工单用的语言 (这个人的工单跟踪系统服务于这个人所有的仓库)，这个人在自己机器上用的各个目录，以及这个人环境里的发行版本、CI runner 和 FedNet socket (见下)。`statute.toml` 给出仓库的取值；[全局配置](global-configuration.md#the-global-configuration) 和私有规定清单都可以设定这个人自己的取值，私有输出用它 (见下)。

`repo` 参数的取值按顺序取第一个有的：`statute.toml`，参数的 `default`。`personal` 参数在私有输出里的取值按顺序取第一个有的：私有规定清单、全局配置、`statute.toml`、`default`；在提交的 `AGENTS.md` 里只取后两者中的第一个。

- 键不对应规定清单任何规定源的参数，报错；值的类型不对，或者是 statute 不知道名称的语言代码，也报错；全局配置服务每个仓库，其中不对应本规定清单任何规定源参数的键不予理会。为没有启用规定在用的参数设了值，给一个警告；这样关掉相关规定时，值可以留在原处。
- `statute init` 不从全局配置复制任何取值：新的规定清单从默认值开始，个人的取值直接从全局配置进入私有输出。

提交的文件不随编译的人而变：提交的 `AGENTS.md` 和 `statute.lock` 只含 `statute.toml` 的取值和默认值，所以没有全局配置的 CI 构建出同一个文件，每个人读到的规定也一样。只有某一个人想要的东西不进这些文件。

**语言参数**共六个，默认都是 `en`：

| 名字 | 范围 | 管什么 |
| --- | --- | --- |
| `language.communication` | `personal` | agent 写给人的话：回复、提问、汇报 |
| `language.code-comments` | `repo` | 代码里的注释 |
| `language.docs` | `repo` | 仓库里的文档：Markdown、设计和其他文字文件 |
| `language.commits` | `repo` | commit message 与 pull request 标题 |
| `language.pull-requests` | `repo` | pull request 描述、pull request 里的评论和审查 |
| `language.work-items` | `personal` | issue 和其他工单 |

commit message 与 pull request 描述分成两个参数，是因为常见的做法是前者用英文，方便查历史、方便工具处理，后者用读这个仓库的人的语言。代码本身，即标识符、文件名和属于接口的字符串，没有参数：一律用英文，这一点已经定了。

**issue 跟踪系统里的名字。** 领域 R175 (`tickets`) 里关于工单的规定会写到跟踪系统的标签和状态名，这些名字因系统、因团队而异。四个范围为 `repo` 的 `string` 参数给出这些名字，默认值都是常见的名字：

| 名字 | 默认 | 管什么 |
| --- | --- | --- |
| `work-items.label-needs-decision` | `needs-user` | 等人来看或来定的工单的标签 |
| `work-items.label-approved` | `approved` | 人已批准的工单的标签 |
| `work-items.status-done` | `Done` | 以完成告终的一次尝试的状态 |
| `work-items.status-abandoned` | `Abandoned` | 被放弃的一次尝试的状态 |

**目录。** 一个人把各仓库的主检出、worktree 以及 agent 在机器上用到的其他目录放在哪里，是这个人自己的事，在这个人工作的每台机器上都一样，所以每个目录都由一个范围为 `personal` 的 `string` 参数给出，默认值取一个中性的名字，不对应任何人的实际布局：

| 名字 | 默认值 | 用到它的规定 |
| --- | --- | --- |
| `dirs.main-checkouts` | `~/src` | R287、R288 |
| `dirs.worktrees` | `~/worktrees` | R309 |
| `dirs.scratch` | `~/agent-scratch` | R310 |
| `dirs.data` | `~/local-data` | R311 |
| `dirs.third-party` | `~/third-party` | R312、R313 |
| `dirs.initiatives` | `~/initiatives` | R314 |

R314 在领域 R161 (`fleet-cli`) 里，其余的在领域 R127 (`machine`) 里。用到这些参数的规定都默认关，依赖 scratch 目录和数据目录的 R217 到 R220 也默认关，没打开它们的规定清单对这些目录什么也不说。要用它们的人在自己那一层打开它们并给出取值：公开仓库里由全局配置或私有规定清单给私有输出；私有仓库没有私有输出，由 `statute.toml` 设定。

**发行版本、runner 与 socket。** R135、R212、R322 和 R323 把这个人环境里的三个值留给规定清单：Linux 机器与测试容器的发行版本，CI 服务托管的 Linux runner 的标签，本机 FedNet client 的 socket (R322 和 R323 共用)。每个值都由一个范围为 `personal` 的 `string` 参数给出，默认值不对应任何人的环境；用到它的规定默认关，并依赖它补值的那条规定：

| 名字 | 默认值 | 用到它的规定 | 补值的规定 |
| --- | --- | --- | --- |
| `linux.container-image` | `debian:13` | R316 (领域 R127) | R135 |
| `ci.linux-runner-label` | `ubuntu-22.04` | R317 (领域 R276) | R212 |
| `fednet.client-socket` | `/run/fednet/client.sock` | R318 和 R325 (领域 R188) | R322 和 R323 |

机器的发行版本和容器的基础镜像由同一个值 (镜像的名字和 tag) 给出，两者不会对不上。R135、R212、R322 和 R323 没有被取代，因为它们的要求不靠这个值也成立：启用它们而没打开新规定的规定清单仍有它们，只是不提这个值。取值的设定方式与目录相同。

这十九个参数都声明在 `rules/params.toml` 里。表 `work-items` 与键 `language.work-items` 不冲突：一个是规定清单的表，一个是表 `language` 里的键。

`[compile] lang` 与语言参数分开：它指定编译出的 `AGENTS.md` 用哪种语言写，关乎 agent 读什么；参数说的是 agent 写什么。

### <a id="values-in-the-lock-file-and-the-outputs"></a>锁文件与输出里的取值

**锁文件**为启用规定用到的每个参数记一行，与规定分开，记下提交的 `AGENTS.md` 得到的取值：

```toml
[[parameters]]
name = "language.commits"
value = "zh"
```

规定清单里的取值或参数的默认值变了，锁文件就过期了，与规定改动时一样；`upgrade` 列出这个参数及其新旧取值。个人的取值来自全局配置和私有规定清单，不进任何锁文件：statute 每次编译私有输出时现读。

**编译出的 `AGENTS.md`** 用取值替换占位符，不加任何标记：语言写成它在文件所用语言里的名称 (`Chinese`、`中文`)，字符串照写。规定的要求读起来就像取值本来就写在规定里。

**私有输出** `.statute/AGENTS.full.md` 写入 `personal` 参数的个人取值，其余与提交的文件相同：没有个人取值时，两个文件内容一致。在公开仓库里，某个 `personal` 参数的个人取值与提交的取值不同时，即使没有私有规定清单，`statute compile` 也写私有输出，全局配置开启了规定时也一样 ([全局配置开启什么](global-configuration.md#what-the-configuration-turns-on))，`statute compile --private-only` 这时同样会写。私有仓库 (`public = false`) 没有私有输出，所以那里个人的取值不起作用，`statute compile` 用一条 note 说明这一点，并指出是哪个参数。

**`statute explain <name>`** 说明取值、它从哪里来 (`statute.toml`、默认值；`personal` 参数还可能来自私有规定清单或全局配置，两个取值不同时都列出)，以及哪些启用的规定在用它。

**公开网站**为每个规定源设一个参数页：列出每个参数的标题、名字、类型、范围、默认值 (`choice` 还列出可选的值)，以及用到它的规定。占位符显示为它的默认值，标成一个取值，带一个提示，说明这是可配置的参数并写出参数名，并链接到它在那一页上的条目。**本地网站**以同样的方式显示它那份规定清单的取值，并在参数页上写明每个取值从哪里来。

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

- 取 `on` 或 `off`。one-of 的子节点没有 `default`；由 one-of 指定其中一个作为默认 (见下)。
- 带 `overrides` 的规定也没有 `default`：只有规定清单启用它时它才进来，进来后顶替被覆盖的规定 ([覆盖](rule-sources.md#overrides))；被覆盖的规定是 one-of 的选项时，它自己再写一个开关，就会与 one-of 的默认相矛盾。
- 只适用于部分仓库或部分工作方式的领域是 `off`，比如 Agent Fleet 这个领域只适用于在多个 agent 之间分工的工作：规定清单开启它之前，它的规定一条也不进来。子领域的 `default` 在它的领域开启之后才起作用。
- 一条规定只有属于最通用、放在哪里都最不容易出错的那一类时才是 `on`。依赖某个人的偏好或环境 (共用的机器、一个审查方、某种 CI 服务、某个工具) 的规定是 `off`。
- 规定源的默认值开启的规定必须能单独解析，statute 会检查这一点。一条仍有效 (既没被取代也没被废止) 的规定，自己的 `default` 是 `on`，上面每个领域都是 `on`，途经的每个 one-of 的默认都是通往它的那个选项，它就是按默认启用的。这样的规定依赖的同一规定源里的每条规定，也必须按默认启用；依赖一个 one-of 算作满足。

statute 读 `default`，节点缺了它、或者在上面所说不该有的地方写了它，都报错，并做上面和下面的检查；还没有选项的 one-of 没有可以点名的 `default`。第一方的规定、领域和 one-of 都写了它：规定文件就是第一方默认值的记录，`statute explain --defaults` 列出一份规定清单从这些默认值得到什么 (见下)。

**one-of 的默认。** 每个 one-of 都在一个必填字段里指定它的一个选项，即一个子节点，作为默认：

```toml
id = 79
kind = "one-of"
parent = 232
default = 81
```

- `default` 是这个 one-of 一个子节点的 ID；写成别的节点，statute 报错。
- one-of 所在的领域开启，one-of 就开启：它启用默认的选项，除非规定清单选了另一个选项，或者把这个 one-of 关掉 (见下)。这样仓库开启一个领域，每个 one-of 都有一个能用的选择，不用自己去选，只改它在意的那几处。是领域的选项，其中的规定各按自己的 `default` 进来，与 `domains` 开启的领域一样 (见[规定不挂在规定下面](#no-rule-under-a-rule))。
- 只有部分仓库需要的 one-of，放在一个 `off` 的领域或子领域里，由需要的仓库开启。
- 默认选项带进来的规定也必须能单独解析，与 `on` 规定一样 (见上)。

改动规定、领域或 one-of 的 `default` 不改变什么算合规，所以是原地修改，修订号加一 ([ID 与修订号](rule-format.md#ids-and-revisions))。`upgrade` 列出因此进来或退出的规定，与列出依赖带进来的规定一样。

### <a id="in-the-rule-set"></a>规定清单里

已经实现。

规定清单只列出与默认值不同的地方：

```toml
rules = ["R57", "R62"]
domains = ["R78"]
exclude = ["R83", "R337"]
packs = ["R96"]
```

- `rules` 与现在一样，开启一条规定。对 `off` 的规定或 `off` 领域里的规定，开启它们靠的就是这一项；列出默认值已经开启的规定，给一个警告。列出 one-of 的一个规定选项，就用它代替 one-of 的默认。
- `domains` 开启一个 `off` 的领域：其中的规定按各自的 `default`。列出一个 `on` 的领域，给一个警告。列出 one-of 的一个领域选项，就选中它，与 `rules` 选中规定选项一样。
- `exclude` 关掉一个默认开启的领域、one-of 或规定。关掉一个节点，就关掉它的整棵子树；关掉的 one-of 一个选项也不启用。被某条启用的规定依赖的规定不能关：构建失败，并指出依赖它的那条规定。one-of 的选项，不论是默认的还是别的，都不能写进 `exclude`：构建失败，并提示在 `rules` 或 `domains` 里写另一个选项，或者关掉整个 one-of。同一个节点既在 `exclude` 里、又在 `rules` 或 `packs` 里，也失败，并提示只留一处。
- `packs` 与现在一样，开启整棵子树里的每条规定，不论它的 `default`；但在 one-of 处只开一个选项：规定清单在 `rules` 或 `domains` 里点名的那个，没有点名就是默认的那个；是领域的选项，开启其中每条规定。`exclude` 可以关掉其中一部分。
- 一条规定依赖某个 one-of，而规定清单没有启用这个 one-of 的任何选项时，带进来的是默认选项，与 statute 补上任何缺失的依赖一样，不再报错。
- `statute add` 把领域写进 `domains`，把规定写进 `rules`；`statute remove` 从 `rules`、`domains` 或 `packs` 里去掉一项，这条规定、这个领域或这个 one-of 仍然默认开启时，把它写进 `exclude`。one-of 的选项它哪里都不写：选项仍然开启、又没有从哪个列表里去掉时，它报错，并指出关掉它的两种做法 (添加另一个选项，或去掉整个 one-of)。
- `private.toml` 可以有 `rules`、`domains`、`packs` 和 `exclude`，它们只改变私有输出：`AGENTS.md` 和 `statute.lock` 仍是 `statute.toml` 给出的样子 ([公开仓库里的私有规定](rule-sources.md#private-rules-in-a-public-repository))。它们可以点名任何规定源的节点，包括第一方规定源，每一项都与在 `statute.toml` 里一样起作用，作用在两层合起来的规定清单上：
  - `exclude` 关掉任一层的默认值开启的领域、one-of 或规定，或者 `statute.toml` 列出的。它点名的 `statute.toml` 的 `rules`、`domains` 或 `packs` 里的一项，在私有层里去掉；`statute.toml` 在它点名的领域下列出的规定仍然启用，与在 `statute.toml` 里一样，要按名字关掉。
  - `rules`、`domains` 和 `packs` 开启默认值没有开启的，也重新开启 `statute.toml` 的 `exclude` 点名的：这一项在私有层里离开那个 `exclude`，也不给出「默认值已经开启它」的警告，因为只有去掉那个 `exclude`，默认值才开启它。
  - 上面 `exclude` 的检查对两层合起来成立。被任一层的某条启用的规定依赖的规定不能关：构建失败，并指出依赖它的那条规定；关掉它的是 `private.toml` 的 `exclude` 时报在 `private.toml` 上，是 `statute.toml` 的时报在 `statute.toml` 上。私有规定依赖的公开规定，只有连同那条私有规定一起才能关掉；依赖 `statute.toml` 排除的规定的私有规定，需要 `private.toml` 的 `rules` 列出那条规定，报错会把这一点列为出路。one-of 的选项不能列；同一个节点既在 `private.toml` 的 `exclude` 里、又在它的 `rules` 或 `packs` 里，也失败。
  - 一项不能重复 `statute.toml` 同一个列表里已有的一项。

  `statute add --private` 写进 `private.toml` 的 `rules` 或 `domains`，先把这一项从它的 `exclude` 里去掉；`statute remove --private` 从它的列表里去掉一项，默认值或 `statute.toml` 仍然开启这条规定或这个领域时，把它写进它的 `exclude`。

  这个文件属于一个 clone，所以一个人要在所有仓库里开启同样的领域，就改在全局配置里列一遍 ([全局配置开启什么](global-configuration.md#what-the-configuration-turns-on))。`private.toml` 能否选与 `statute.toml` 不同的 one-of 选项，是[待决问题](rule-system.md#open-questions) 之一。

**取谁的默认值。** 规定清单取第一方规定源 `R` 的默认值，不论 `R` 从哪里来；也取它自己的 `[sources]` 写了的每个规定源的默认值，即 `statute.toml` 里写的，私有层则是 `private.toml` 里写的，后者的默认值只对私有层算数。仓库自己的规定 `L` 也算这样的规定源：它们的 `default` 与第一方规定的一样算数。只有全局配置登记的规定源不带进任何默认规定；要用它的规定就逐条列出，或者在规定清单里写上这个规定源，取它的默认值。否则，往全局配置里加一个规定源，就会在每个仓库里开启规定，而这些仓库的规定清单里没有任何一处写明。规定源条目写 `defaults = false`，就不取这个规定源的任何默认值：

```toml
[sources]
team = { git = "https://example.test/our-team/rules.git", branch = "main", public = true, defaults = false }
```

`defaults` 只是规定清单里规定源条目的键：写在全局配置里会报错，因为全局配置登记的规定源不带默认。

`statute init` 不写 `rules`、`domains` 或 `exclude`：新的规定清单就是第一方的默认值。

### <a id="defaults-in-the-lock-file-and-the-outputs"></a>锁文件与输出里的默认值

**锁文件**与现在一样记下每条启用的规定。只因默认值而启用的规定，在它那一行加 `by = "default"`，这样锁文件能说明哪些规定不是选进来的，`upgrade` 也能列出因 `default` 改动而进来或退出的规定。

**`statute explain <rule>`** 对一条规定或一个领域，说明它是否默认开启，以及规定清单为什么有它或没有它：「R44 默认开启，它的领域 R13 也开启」；「R337 默认开启，被 `exclude` 关掉」；「R78 默认关闭；`statute add R78` 开启它」。对 one-of，它指出默认的选项，规定清单选了别的选项时也指出选的是哪个。`statute explain --defaults` 为规定清单打出三张列表：它保留的默认开启的规定、它关掉的默认开启的规定、它开启的默认关闭的规定。

**编译出的 `AGENTS.md`** 不变：它收录启用的规定，不论它们怎么进来的。

**公开网站**在列表和详情页上把每个领域和每条规定标成默认开启或默认关闭，标出每个 one-of 的默认选项，列表加一个只显示默认开启规定的筛选，让读者一眼看出一份新的规定清单里有什么。**本地网站**在它那份规定清单是否启用这条规定的旁边，显示同样的这些标记。两个网站都已经显示这些标记。

### <a id="moving-a-repository-over"></a>把仓库迁过来

仓库迁到 statute 上来，不论是从手写的指南迁，还是从默认开关出现之前写的规定清单迁，都从默认值出发，过三张列表；规定清单写好之后，`statute explain --defaults` 会把它们打出来：

1. 仓库保留的默认开启的规定：什么都不用做；
2. 它必须关掉的默认开启的规定：写进 `exclude`，每项用一行注释说明原因；
3. 它需要的默认关闭的领域和规定：写进 `domains` 和 `rules`；某个 one-of 要选默认以外的选项时，写进 `rules` 或 `domains`。

默认开关出现之前写的规定清单，把用到的每条规定都列在 `rules` 和 `packs` 里；按默认值，它还可能得到从没列过的规定。默认开关上线后的第一次 `lock` 会列出它们，迁移时把每一条放进第二张列表，或者保留。指导迁移的 skill [statute-migrate](../../../skills/statute-migrate/SKILL.md) 按这个顺序过这三张列表。

## <a id="where-a-rule-is-used"></a>规定用在哪里

一条规定有两类读者：干活的 agent 读编译出的 `AGENTS.md`，审查方过 `statute check` 打出的清单 ([结果与审查回路](rule-checks.md#outcomes-and-the-review-loop))。不是每条规定两边都用得上。「不在 agent 之间传递凭证」(`rules/45.toml`) 是 agent 干活时必须知道的，但手里只有 diff 和 pull request 描述的审查方判断不了它有没有被违反。「描述里写的 head 就是审查的 head」这类要求则只对审查方有用。大部分规定根本不必进审查。

**规定文件里**用 `uses` 列出规定去哪里，取下面两个值的非空子集：

```toml
uses = ["instructions", "review"]
```

- `instructions`：`statute compile` 把这条规定写进编译出的 `AGENTS.md`。
- `review`：`statute check` 把这条规定连同理由和核验方法列给审查方。
- 没写 `uses` 的规定取 `["instructions"]`；要 `review` 就写出来。空列表报错，这两个以外的值或者同一个值写两次也报错。领域和 one-of 没有 `uses`，参数不是节点，也没有。

`uses` 与 `check` 是两回事。`check` 说的是由程序怎样检查这条规定：`builtin`、`command`，或者 `none`，即没有程序 ([规定文件格式](rule-format.md#rule-file-format))。`statute check` 运行每条有程序的启用规定的程序，不论它的 `uses`；列给审查方的只有 `uses` 含 `review` 的规定。`uses` 出现之前，`none` 叫 `review`；写 `check = "review"` 时 statute 报错，提示改写成 `none`。改 `uses` 不改变什么算合规，所以原地修改，修订号加一。

**怎么选。** 规定 R233 (`rules/233.toml`) 说明了：只有凭 diff 和 pull request 描述就能判断有没有违反的规定，才标 `review`；只有 agent 干活时需要知道的规定，才标 `instructions`。

**由此而来的。** statute 读 `uses`，编译出的 `AGENTS.md`、`statute check`、`statute explain` 和两个网站都按它来。

- 编译出的 `AGENTS.md` 收录 `uses` 含 `instructions` 的启用规定；对只进审查的规定的引用，和别的引用一样写成它的引用。
- `statute check` 在各个领域里列出 `uses` 含 `review` 的启用规定，以及它运行的每个程序的结果。
- `statute explain <rule>` 说明一条启用的规定去哪里：「R43 编进 `AGENTS.md`，并列给审查方」。
- 锁文件不记新东西：`uses` 是规定文件的一部分，锁文件已经用 SHA-256 钉住了它。
- 网站在规定详情页显示 `uses`，规定列表加一个按它筛选的选项。

## <a id="no-rule-under-a-rule"></a>规定不挂在规定下面

[依赖、one-of 与规定包](rule-sets.md#dependencies-one-ofs-and-packs) 里的树是下面这个形状，statute 读规定源时检查：

- 规定的 parent 是领域或 one-of，从不是规定。规定挂在规定下面时原本表达的「它细化了另一条」，改写成 `requires`。
- one-of 的 parent 是领域。
- one-of 的选项就是它的子节点，要么全是规定，要么全是领域，不能混。只有一条规定的选项就是那条规定，外面不套领域；需要几条规定的选项是一个以这个 one-of 为 parent 的领域，规定放在里面；它被选中时，其中的规定各按自己的 `default` 进来，与 `domains` 开启的领域一样。这样的领域和别的领域一样，也可以有自己的子领域和 one-of。
- 每个 one-of 都指定一个选项作为默认 (见[one-of 的默认](#in-the-rule-files))。
- 选项在自己的 `parent` 里写它的 one-of；one-of 不列 `options`。树仍然只记在 `parent` 里 ([规定文件格式](rule-format.md#rule-file-format))，增加或移动一个选项只改一个文件。one-of 自己的文件里看不到它的选项；`statute explain` 和网站会列出来。
- 编译出的 `AGENTS.md` 里，one-of 被选中的选项放在 one-of 的位置，不写任何指明 one-of 的内容：是规定的选项和别的规定一样是一项，是领域的选项写成一个标题，下面是它的规定，比 one-of 所在的领域低一级 ([编译与网站](statute-command.md#compilation-and-the-site))。在网站上，one-of 是它所在领域列表里的一个框，列出每个选项并标出默认；是规定的选项只出现在这个框里，不在列表里再出现一次。是领域的选项，它的规定也留在列表里，挂在它自己的标题下 (公开网站上是它的分组行)，比 one-of 所在的领域低一级；one-of 不作任何分组的小标题。两个网站都已经这样渲染。

**为什么。** 读网站的人，看到的每条规定都直接在领域或 one-of 下面，没有规定嵌在规定里；只有一条规定的选项也不需要自己的领域。规定的 parent 算作依赖的特例没有了；依赖穿过 one-of 传到 one-of 的 parent 的特例也没有了，因为那个 parent 总是领域。失去的是表示一条规定细化另一条的缩进；`requires` 保留了这层关系，网站在两条规定上都会显示。

**改了什么。** 移动了一条规定：R94 原来的 parent 是 R93，现在挂在 R13 下，并依赖 R93。statute 不再把 parent 算作依赖；锁文件没有变。

## <a id="what-this-changes"></a>它改变什么

已经实现：statute 读领域、规定和 one-of 上的 `default` (规定文件需要它的地方必填)、`uses` 与 `check = "none"`，`check = "review"` 报错，做上面关于默认值和规定树的检查，编译出的 `AGENTS.md` 和 `statute check` 按 `uses` 来；`statute validate` ([命令与第一版](statute-command.md#the-command-and-the-first-version)) 对一个规定目录做这些检查。第一方规定都写了 `default` 和 `uses`，`rules/params.toml` 声明了它们的参数。

规定清单的默认开关也已经实现：`domains`、`exclude`、`local-rules` 和规定源条目的 `defaults`，默认值生效，锁文件里的 `by = "default"`，以及 `statute add`、`remove`、`explain` (包括 `--defaults`) 对它们的处理。

参数已经实现：`params.toml` 和占位符都读取并检查，规定清单、私有规定清单和全局配置设定取值，锁文件、`upgrade`、`explain <name>` 和两份输出都按取值来。

本地网站标出默认开关、每个 one-of 的默认选项、`uses` 和参数的取值 (见[本地网站](statute-command.md#the-local-site))；公开网站标出默认开关和每个 one-of 的默认选项，显示 `uses`，并能按它们筛选；它还为每个规定源设一个参数页，规定正文里的占位符链接到那里。

编译出的 `AGENTS.md` 把每个 one-of 启用的选项里的规定放在 one-of 的位置，不写任何指明 one-of 的内容；两个网站都把 one-of 画成装着它的选项的框。

这部分设计描述的内容都已实现。

这部分设计改动了设计的以下部分：

- [选择规定](rules.md#choosing-rules)：其中建议一个管辖范围用全部通用规定、按名字去掉不要的规定，改由默认开关代替。
- [本地参数](rules.md#local-parameters)：参数声明在它的规定源的 `params.toml` 里，占位符是 `{$<name>}`，取值来自规定清单，`personal` 参数还可以来自这个人，而不是来自覆盖层的 `params.toml`。
- [规定清单与锁文件](rule-sets.md#rule-sets-and-the-lock-file)：规定清单增加 `local-rules`、`domains`、`exclude` 和参数的表，规定源条目增加 `defaults`，锁文件增加 `by` 和 `[[parameters]]`。
- [规定文件格式](rule-format.md#rule-file-format)：领域、规定和 one-of 上的 `default`，规定上的 `uses`，`check = "none"`，以及节点可以有哪些 parent。
- [结果与审查回路](rule-checks.md#outcomes-and-the-review-loop)：`statute check` 只把 `uses` 含 `review` 的规定列给审查方，每条规定的程序照旧运行。
- [编译与网站](statute-command.md#compilation-and-the-site)：编译出的 `AGENTS.md` 只收录 `uses` 含 `instructions` 的规定。
- [依赖、one-of 与规定包](rule-sets.md#dependencies-one-ofs-and-packs)：规定不挂在规定下面；规定清单没有选时，one-of 启用它的默认选项。其中「补上缺失的依赖」一段 (依赖一个没有启用任何选项的 one-of 时报错) 和「规定包」一段 (规定清单没有点名任何选项的 one-of 不贡献任何规定)，都改为取默认选项。
