<!-- en-sha256: 217016902a38f578795a6515e2322f8e459a37849648274bb16ed9815e86ecaf -->
# 全局配置与 git 规定源

状态：已定；部分已实现。已经实现的部分见[命令与第一版](statute-command.md#the-command-and-the-first-version)。这部分设计说明 statute 在用户家目录里把自己的文件放在哪里，全局配置怎样为每个仓库登记规定源，git 规定源怎样克隆、固定和读取，以及怎样用一条命令添加规定源。它以[规定清单与锁文件](rule-sets.md#rule-sets-and-the-lock-file) 和[规定源](rule-sources.md) 为基础。

## <a id="contents"></a>目录

- [statute 的目录](#the-statute-directories)
- [全局配置](#the-global-configuration)
- [一个名字指哪个规定源](#which-source-a-name-means)
- [git 规定源](#git-sources)
- [添加规定源](#adding-a-source)

## <a id="the-statute-directories"></a>statute 的目录

statute 按 XDG Base Directory 规范，把自己的文件放在家目录里的两个目录，Linux 和 macOS 一样：

```text
$XDG_CONFIG_HOME/statute/           ~/.config/statute/ by default
  config.toml                       the global configuration
$XDG_CACHE_HOME/statute/            ~/.cache/statute/ by default
  repos/<name>-<hash>/              a bare clone of each git source
  snapshots/<commit>/               the files of one commit, written once
```

- `repos/` 里每个地址一份裸克隆 (bare clone)：`<name>` 是地址的最后一段，去掉 `.git` (`agent-constitution`)，`<hash>` 是按原样写的地址的 SHA-256 的前 12 位十六进制，所以两个地址永远不会共用一份克隆。裸克隆没有工作区，statute 在那里做的任何事都不会改动别人正在用的文件。
- `snapshots/<commit>/` 放一个提交的文件，statute 像读磁盘上的规定源一样读它。它先写进旁边的一个临时目录，再改名放到位，此后不再改动；一个提交唯一确定它的内容，所以有这个提交的每份克隆都共用同一份快照。

缓存目录里的东西随时可以删：statute 需要时会重新克隆、重新写出。缓存本来就是这样的东西，规范把它和配置分开，配置是用户自己写、要留着的；备份或者同步 dotfiles 时，只取配置、不取缓存。`XDG_CONFIG_HOME` 和 `XDG_CACHE_HOME` 设成绝对路径时才用；没设、为空或者是相对路径时，规范说要忽略它们，这时退回家目录下的 `.config` 和 `.cache`，家目录取系统报告的那个 (Unix 上是 `HOME`)。macOS 也用同样的路径，不用 `~/Library`，与 uv、ruff 和 mise 对这类配置要用户手改的命令行工具的做法一样。如果叫 `~/.statute/`，它会和 statute 在仓库里放的 `.statute/` 同名，statute 在家目录里运行时两者就撞在一起。不从 `~/.statute/` 迁移：此前没有仓库用过 statute。

`statute init --global` 写出 `config.toml`，里面写明内置默认 (见下)，或者把 `R` 写成 `--source` 给的 git 地址，再附一个私有规定源的示例，以注释的形式；接着克隆或拉取它登记的每个 git 规定源，这样没有网络或没有账号的问题当场就会暴露。已有 `config.toml` 时原样保留，命令以退出码 1 退出。

## <a id="the-global-configuration"></a>全局配置

`config.toml` 登记用户的每份规定清单都能用的规定源，格式和规定清单的 `[sources]` 一样：

```toml
[sources]
R = { git = "https://github.com/Luolc/agent-constitution", branch = "main" }
me = { git = "git@example.test:me/rules.git", branch = "main", public = false }
```

- 每个规定源都是 git 规定源：`git` 加 `branch` 或 `tag`，和规定清单里一样 ([git 规定源](#git-sources))。提交进仓库的锁文件按地址和提交记录 git 规定源，每台机器都能重新读到；某台机器上的一个目录只能按它现在的样子读，所以它留在用 `path` 写它的那份规定清单里。
- 在 `git` 和 `branch` 或 `tag` 旁边写 `clone = "<dir>"`，就用用户自己维护的一份克隆，不用 `repos/` 下的。这时 statute 不克隆，也不往里拉取：它读 `refs/remotes/origin/<branch>` 或那个 tag 的提交，也就是用户上一次拉取留下的样子；那份克隆的 `origin` 不是 `git` 写的地址时报错。目录开头的 `~` 指家目录。
- 除 `R` 以外，每个规定源都必须写 `public`，和规定清单里一样；没有 `scope`，因为全局配置里的规定源按定义就是全局的 ([四类规定源](rule-sources.md#four-kinds-of-source))。
- 全局配置登记规定源，不启用任何规定：每份规定清单照旧列出自己用的规定和规定包，这样单看提交进仓库的规定清单和锁文件，就知道这个仓库编译出的输出里有什么。全局配置里的规定源，每份规定清单都可以按名字使用，就像规定清单自己写了它一样。全局配置里还可以写这个人自己的 `personal` 参数取值，比如回复用的语言，只有私有输出用它，提交的文件从不用；在这里写 `repo` 参数的取值，给一个警告并忽略 ([设定取值](rule-config.md#setting-a-value))。

**内置默认**是 statute 自己知道的唯一一个规定源：`R = { git = "https://github.com/Luolc/agent-constitution", branch = "main" }`。第一方规定源还没有给规定发布带 tag 的版本；发布以后，默认跟踪最新的 tag。

**全局规定源进哪一层。** 全局配置里的公开规定源进 `statute.toml` 的规定清单。私有的，在私有仓库 (`public = false`) 里同样进 `statute.toml`；在公开仓库里只进私有规定清单的私有层 ([公开仓库里的私有规定](rule-sources.md#private-rules-in-a-public-repository))：公开仓库提交的文件里从不出现它。在公开的 `statute.toml` 里启用私有规定源的规定会报错，并给出两条路：在私有层里启用它，`statute add --private me:4`；或者不用它。

## <a id="which-source-a-name-means"></a>一个名字指哪个规定源

对 `statute.toml` 里的每个名字，按下面的顺序，第一个写了它的地方决定是哪个规定源：

1. `statute.toml`；
2. 全局配置；
3. 锁文件，只对 `compile`、`check`、`explain` 和 `site` (见下)；
4. 内置默认，只对 `R`。

解析 `statute.toml`、它的锁文件和它的 `AGENTS.md` 时，从不读私有规定清单 `private.toml`。私有层只增加名字，见[公开仓库里的私有规定](rule-sources.md#private-rules-in-a-public-repository)：公开层已有的规定源，不管来自上面四处中的哪一处，`private.toml` 都不能再写；除此之外，私有层的名字先在 `private.toml` 里找，再在私有层带上的全局配置私有规定源里找，最后在 `private.lock` 里找。

规定清单写了一个规定源，就整个替换掉全局配置里的那个条目：各层的字段不合并。在规定清单的 `[sources]` 里写 `name = false`，就把全局配置里的这个规定源从这份规定清单里拿掉，就像它不存在；内置的 `R` 只能靠另写一个 `R` 来替换。`statute.toml` 里的 `[sources]` 可以不写：只用默认的第一方规定源的规定清单，一个都不用写。

锁文件排第三，是为了让没有用户全局配置的机器，比如 CI 或另一个人的机器，照样能构建出锁文件固定的内容：它记下每个 git 规定源的地址、跟踪的分支或 tag、提交，以及除 `R` 以外每个规定源的 `public` ([git 规定源](#git-sources))，这样哪些规定可以引用哪些的检查在那里也能跑。`lock` 和 `upgrade` 不把它当作规定源来读：它们按配置现在的样子解析规定清单，所以从全局配置里拿掉的规定源，下一次 `lock` 时就去掉了。

## <a id="git-sources"></a>git 规定源

规定清单或全局配置里的规定源可以是一个 git 仓库：

```toml
[sources]
team = { git = "https://example.test/our-team/rules.git", branch = "main", public = true }
```

- `git` 是 `git clone` 能接受的地址，按原样写：`https://...`、`ssh://...`、`git@host:path`，或者指向本机仓库的 `file://...` 和绝对路径；测试用的就是后两种，不需要网络。
- `branch` 和 `tag` 二选一，说明跟踪什么。一个条目里同时有 `path` 和 `git` 会报错，和以前一样。
- 它的规定在仓库顶层的 `rules/` 里，或者在 `rules-dir` 指定的目录里 (见[规定源与引用](rule-sets.md#sources-and-references))，宪法在 `constitution/<edition>/` 里，技能在 `skills/<name>/` 里，和磁盘上的规定源一样。

**固定版本。** 锁文件记下 git 规定源的地址、分支或 tag，以及提交：

```toml
[constitution]
git = "https://github.com/Luolc/agent-constitution"
commit = "<commit>"
edition = "2026-10"
revision = "<revision>"

[[sources]]
name = "R"
git = "https://github.com/Luolc/agent-constitution"
branch = "main"
commit = "<commit>"

[[sources]]
name = "team"
git = "https://example.test/our-team/rules.git"
branch = "main"
commit = "<commit>"
public = true
```

除 `R` 以外，每个规定源在那里都记有 `public`，磁盘上的规定源也一样；`rules-dir` 不是 `rules` 的规定源还记有 `rules-dir`，这样只有锁文件写着的规定源也从同一个目录读。

- 规定源的地址和分支或 tag 与锁文件记的相同时，`lock` 保留锁文件记的提交，这样规定清单不升级就什么都不变。新的规定源，或者地址、分支变了的，`lock` 会拉取，取分支的最新提交或 tag 指向的提交。
- `upgrade` 拉取每个 git 规定源，取各自最新的提交，再像现在一样显示变了什么。因为旧提交读得到了，现在可以显示一条规定改动的文字；这一点还没有实现。
- `compile`、`check`、`explain` 和 `site` 按锁文件记的提交读每个 git 规定源；没有锁文件，或锁文件里没有这个规定源的那一行时，像 `lock` 一样取最新提交。规定源的地址、分支或 tag 与锁文件记的不同，锁文件就过时了：它们报错，提示运行 `statute lock`，不读任何提交。它们只在克隆里缺这个提交时才拉取；拉取之后还缺，就报错，写明规定源、提交和地址；用 `clone` 时不拉取，直接报错，请用户在那份克隆里拉取。

**读取。** statute 把一个规定源克隆一次，`git clone --bare`，放进 `repos/` ([statute 的目录](#the-statute-directories))；上面的命令需要一个克隆里没有的提交，或者需要最新的提交时，就往这份克隆里拉取，`git fetch` 远端的分支和 tag。读一个提交时读它的快照：用 git 自己的命令，借一个临时的索引，把这个提交的文件写进 `snapshots/<commit>/`，所以任何克隆的工作区、`HEAD` 和索引都不会变。同一个提交因此总是得到同样的文件，编译出的内容只在锁文件变了时才变。

提示信息里，git 规定源的文件按规定源和提交来写，`R@1a2b3c4d:rules/7.toml`。

## <a id="adding-a-source"></a>添加规定源

`statute source add <name> <address>` 登记一个规定源：

- `<address>` 是一个 git 地址，或者磁盘上的一个目录。裸仓库算 git 地址，写成它的绝对路径。在规定清单里，别的目录写成 `path`，相对于 `statute.toml` 所在的目录；加 `--global` 时，目录必须是一份克隆，写成 `clone`，它的 `origin` 写成 `git`。
- `--branch` 或 `--tag` 说明跟踪什么；两个都不给时，取远端 `HEAD` 指向的分支，并在条目里写明。
- `--rules-dir` 写出条目的 `rules-dir` (见[规定源与引用](rule-sets.md#sources-and-references))。
- 写入之前，先克隆这个规定源或读那个目录，并带上它解析一遍规定清单；失败就什么都不写。

**公开与否。** `--public` 或 `--private` 说了算。两个都不给时，statute 用 `gh repo view --json visibility` 问这个地址；是目录时，问包含它的那个仓库：

| GitHub 显示 | 写入的 `public` |
| --- | --- |
| `PUBLIC` | `true` |
| `PRIVATE` | `false` |
| `INTERNAL`，或者没有答案 (没有 `gh`、没登录、GitHub 上没有这个地址、目录不在任何仓库里) | `false`，并给出提示，说明原因，以及加 `--public` 就能改成公开 |

不会泄漏的那一边和 `init` 的相反：在那里，错写成 `public = true` 只会让规定清单更严格；在这里，错写成 `public = true` 会让公开规定能引用这个规定源的规定，把它们的文字写进公开的文件 ([哪些规定可以引用哪些](rule-sources.md#which-rules-may-cite-which))。

**写到哪里。**

| | 写入 |
| --- | --- |
| `--global` | `$XDG_CONFIG_HOME/statute/config.toml` |
| 公开的规定源，或私有仓库的任何规定源 | `statute.toml` |
| 公开仓库的私有规定源 | 私有规定清单 `$(git rev-parse --git-path common/statute)/private.toml`，没有时创建；规定源的 `path` 改写成绝对路径；命令会打印这个文件的路径 |

公开仓库的私有规定源写进私有层，命令不停下：私有层里的东西不会进入任何提交的文件，而添加私有规定源，通常就是为了私有 overlay。提示会说明它写到了哪里，以及加 `--public` 就写进 `statute.toml`。要写入的文件里已经有这个名字，命令报错；规定源要写进私有规定清单而公开层已有这个名字时，也报错，因为私有层只增加。只有全局配置里有的名字，在 `statute.toml` 或 `private.toml` 里替换它，见[一个名字指哪个规定源](#which-source-a-name-means)，提示也会说明这一点。有锁文件时，像 `add` 那样重写它。
