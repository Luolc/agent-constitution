<!-- en-sha256: 7f6e8ae9be5440a253ca07ad90d2e5f2345a874f7759b820e8de51c07c785b31 -->
# 规定源

状态：已定，部分已实现，见[命令与第一版](statute-command.md#the-command-and-the-first-version)。这一部分讲一个规定清单从哪几类规定源取规定、哪些规定可以引用哪些、规定清单怎样用自己的一条规定顶替另一条 (覆盖，override)、领域怎样嵌套，以及手写的指南怎样迁成规定、怎样停用。它以[规定体系](rule-system.md) 为基础：引用的写法 (`R42`、`L12`、`team:7`) 见[规定源与引用](rule-sets.md#sources-and-references)，规定树、one-of 与规定包见[依赖、one-of 与规定包](rule-sets.md#dependencies-one-ofs-and-packs)。设计里受这一部分影响的其他地方已经按它写好，并各自标明哪些还没有实现；[它改变什么](#what-this-changes) 一节列出这些地方，以及实现其余部分会替换掉工具现在的哪些行为。

## <a id="contents"></a>目录

- [四类规定源](#four-kinds-of-source)
- [公开与否](#public-or-not)
- [哪些规定可以引用哪些](#which-rules-may-cite-which)
- [公开仓库里的私有规定](#private-rules-in-a-public-repository)
- [覆盖](#overrides)
- [子领域](#subdomains)
- [用 one-of 表达相互冲突的做法](#conflicting-practices-as-one-ofs)
- [从手写的指南切换过来](#cutover-from-the-hand-written-guide)
- [它改变什么](#what-this-changes)

## <a id="four-kinds-of-source"></a>四类规定源

这里要看一条规定的两个属性：管多大范围，谁能读到。范围分全局和本地：全局管一个人或一个团队的所有仓库，本地 (local) 只管一个仓库。能不能读到分公开和私有。两两组合就是四类：

| | 公开 | 私有 |
| --- | --- | --- |
| 全局 | 本仓的规定，写作 `R42` | 个人或团队自己的规定源，比如一个私有仓库：他们的 bot 和机器叫什么、密钥库怎样分层、目录在他们的机器上放在哪 |
| 本地 | 公开仓库的 `.statute/rules/`，写作 `L12` | 私有仓库的 `.statute/rules/`，写作 `L12`；公开仓库的私有规定则放在仓库之外的规定源里 (见下文) |

- **全局公开**：第一方规定源 `R`，以及其他团队公开的规定源，如 `team:7`。
- **全局私有**：规定清单在 `[sources]` 里声明、带 `public = false` 的规定源，可以是私有 git 仓库，也可以是磁盘上的一个目录。
- **本地公开**：仓库公开时，仓库自己的规定，放在 `.statute/rules/` (见[本地规定的目录](rule-config.md#the-directory-of-local-rules))。
- **本地私有**：仓库私有时，仓库自己的规定。公开仓库把私有规定放在仓库之外的规定源里，声明时写 `scope = "local"`，见[公开仓库里的私有规定](#private-rules-in-a-public-repository)。

**规定源和仓库自己的规定**是两回事，一个仓库可以两者都有：

- 规定源把规定发布给别的仓库用。它的规定放在它顶层的 `rules/`，或者它的条目用 `rules-dir` 指定的目录 (见[规定源与引用](rule-sets.md#sources-and-references))；想用这些规定的规定清单在 `[sources]` 里按 git 地址或路径声明这个规定源，引用时写 `R42` 或 `team:7`。
- 仓库自己的规定只约束这个仓库。它们放在 `.statute/rules/`，引用时写 `L12`，只编译进这个仓库的输出。别的仓库没法引用它们：引用别的仓库自己的规定，没有任何写法，`rules-dir` 也不能指到 `.statute/` 里面。

本仓两者都有：它在 `rules/` 里发布第一方规定，在 `.statute/rules/` 里放约束本仓开发的规定。别的仓库也该遵守的规定，放进规定源；只在本仓有意义的，写成本地规定。

不论哪一类，每条规定都自成一体 (self-contained)：引用别的规定只通过 `parent`、`requires`、`conflicts`、`overrides` 和 `{@rule:...}`，引用宪法只通过 `authority` 和 `{@constitution:...}`。一条规定还要指向手写的指南，或者指向任何既不是规定也不是宪法的文字，就说明它还没迁完 (见[切换](#cutover-from-the-hand-written-guide))。

`statute.toml` 里，规定清单多一个字段，规定源的条目多两个：

```toml
public = true

[sources]
R = { git = "https://github.com/Luolc/agent-constitution", tag = "<tag>" }
team = { git = "https://example.test/our-team/rules.git", branch = "main", public = true }
```

- 规定清单顶层的 `public`：持有这份清单的仓库是否公开。必填，没有默认值。它决定本地规定的可见性，也决定提交进仓库的规定清单能写哪些规定源 (见[公开仓库里的私有规定](#private-rules-in-a-public-repository))。
- 规定源上的 `public`：除 `R` 以外的每个规定源都必填；`R` 一定公开，写 `R = { ..., public = false }` 会报错。
- 规定源上的 `scope`：`"global"` (默认) 或 `"local"`，只在覆盖时起作用 (见[覆盖](#overrides))。

## <a id="public-or-not"></a>公开与否

可见性靠声明，读规定清单时不去查询：规定清单为自己、为每个规定源写明 `public = true` 或 `public = false`。要是构建时去托管平台查 (GitHub 上仓库的 visibility)，构建就离不开网络和账号，同样的输入在两台机器上也可能得出不同结果；要是按规定源放在哪来猜 (`R` 公开，其余都私有)，一个团队公开的规定源就成了私有的，公开规定也就不能引用它的规定了。

声明错了，只有一个方向危险。私有仓库声明了 `public = true`，检查比需要的严，但什么也不会泄漏。公开仓库声明了 `public = false`，就可能把私有规定写进谁都能读的文件。所以 `statute check` 有一项内置检查，在能查的时候把声明和托管平台对一对，像[检查](rule-checks.md#checks) 里的内置检查那样通过 `gh` 查：规定清单写的是 `public = false`，而仓库的 visibility 是 `PUBLIC`，就失败；`gh` 答不上来时，报告说无法判断。

这个声明只在写下时填一次，之后只从 `statute.toml` 里读。`statute init` 会问 `gh` 它写规定清单的那个仓库的 visibility，除非 `--public` 或 `--private` 已经指定；GitHub 给不出明确答案时，取不会泄漏的那一边：

| GitHub 显示 | 写入的 `public` |
| --- | --- |
| `PUBLIC` | `true` |
| `PRIVATE` | `false` |
| `INTERNAL`，企业内的人都能读 | `true`，并给出 warning，说明怎样改成私有 |
| 没有答案 (没有 `gh`、没登录、没有 GitHub remote) | `true`，并给出 warning，说明原因以及怎样改成私有 |

添加规定源时，`statute source add` 用同样的办法问 `gh`，只是不会泄漏的那一边反过来：`INTERNAL`、没有答案，以及不在任何 GitHub 仓库里的目录，都当作私有；公开仓库的私有规定源写进私有规定清单，命令不停下 ([添加规定源](global-configuration.md#adding-a-source))。

## <a id="which-rules-may-cite-which"></a>哪些规定可以引用哪些

私有规定可以引用公开规定，公开规定不能引用私有规定。公开规定的正文是要发布的，引用一条谁都读不到的规定，好一点是个断掉的链接，坏一点就泄漏了 (被引规定源的名字和地址)。

公开规定源的规定 (`R`、公开的规定源，或者公开仓库的本地规定) 在下面任何一处指向私有规定源的规定，构建就失败：

- `requires` 或 `conflicts`；
- `overrides` (见[覆盖](#overrides))；
- `parent`，它可以指向另一个规定源的领域 (见[子领域](#subdomains))；
- 标题或正文任何一部分里的 `{@rule:...}` 引用。

报错写明是哪条规定、哪个字段、指向的是哪种节点，以及两边各是哪个规定源：

```text
rules/12.toml: error: `requires` names "me:4", a rule of the private source `me`; a public rule may name only rules of public sources
rules/21.toml: error: `parent` names "me:1", a domain of the private source `me`; a public rule may name only rules of public sources
```

这项检查只读各规定源的声明，用不到私有规定源的文件，所以能在公开仓库的 CI 里跑，那里读不到私有规定源。

## <a id="private-rules-in-a-public-repository"></a>公开仓库里的私有规定

公开仓库提交的文件都是公开的，其中包括规定清单、锁文件和编译出的 `AGENTS.md`。在 `statute.toml` 里写一个私有规定源，就会把它的地址公开出去；把它的规定编译进 `AGENTS.md`，就会把规定的正文公开出去。所以在 `public = true` 的规定清单里：

- `statute.toml` 只写公开的规定源；写了 `public = false` 的规定源就报错，并提示把它写进私有规定清单。
- 私有规定源里只放私有的东西。它不抄公开规定，要用就像别的规定那样引用。
- 私有规定源写在私有规定清单 `private.toml` 里。它放在 git 自己的目录里，不在工作区：

  ```text
  $(git rev-parse --git-path common/statute)/
    private.toml          私有规定清单
    private.lock          它的锁文件
  ```

  在一个 clone 里，这个路径就是 `.git/common/statute/`，主检出和每个 linked worktree 都一样。私有规定清单以 statute 运行所在 worktree 的 `statute.toml` 为基础，按[规定清单与锁文件](rule-sets.md#rule-sets-and-the-lock-file) 里说的方式继承它，再加上规定源和规定：

  ```toml
  rules = ["me:4", "here:2"]

  [sources]
  me = { path = "/home/me/my-rules", public = false }
  here = { path = "~/my-rules/repos/example", public = false, scope = "local" }
  ```

  它可以像 `statute.toml` 那样列 `packs`，根可以是两层中任何一个规定源的节点，这样私有规定源新加的规定会跟着启用；规定包也可以启用公开规定，这时只有私有层启用它们，和它列出的规定一样。它没有 `[compile]`，因为它的输出位置是固定的 (见下文)；也没有 `base`，因为它的基础总是命令运行处的那份 `statute.toml`。其中的 `path` 要么是绝对路径，要么以 `~/` 开头；写相对路径会报错，因为这个文件供所有 worktree 共用，相对路径从每个 worktree 出发会指到不同的地方。`statute source add` 写的是绝对路径。它的规定源不能与 `statute.toml` 的重名，它的规定和规定包也不能与 `statute.toml` 列出的重复。在私有仓库 (`public = false`) 里，这个文件会报错：那里由 `statute.toml` 写下一切。不在 git 仓库里就没有私有层，带 `--private` 的命令会失败。`statute add --private` 和 `statute remove --private` 修改它，先在同一目录写一个临时文件，再改名替换。

- 这个私有层只在公开层之上增加规定。它不能覆盖 `statute.toml` 里的规定，也不能去掉其中的规定 (见[覆盖](#overrides))：不论私有层怎样写，提交进仓库的 `AGENTS.md` 里都有那条规定，agent 两份文件都读，就会看到两条相互矛盾的规定。
- 它有自己的锁文件 `private.lock`，只锁定私有层自己的规定源，别的都不记：每个规定源按 `statute.lock` 记录规定源的方式记，git 规定源记地址、分支或 tag 和 commit，另记 `public` 和 `scope`。它不记任何规定，也不记公开层的任何东西。`statute lock` 把它和 `statute.lock` 一起写出，修改私有规定清单或其规定源的命令都会重写它。两层合起来的解析，每次在命令需要时根据这个 worktree 的 `statute.lock` 和 `private.lock` 在内存里算出来，从不落盘。这样所有 worktree 读私有规定源时用的是同一批 commit，而公开层各自以自己分支锁定的为准；如果锁文件把两层一起锁定，又由不同分支上的 worktree 共用，那它在除一个以外的所有 worktree 里都是过期的。
- git 从不跟踪它自己目录里的文件，所以私有规定清单和它的锁文件不可能被提交，不需要 `.gitignore`，也不需要检查。私有层在工作区里写的是 `.statute/` 下的私有输出和顶层的两个软链 (见下文)；写 `.statute/` 下的那个文件之前，statute 用 `git check-ignore` 确认 git 忽略了它，两个软链按下文的方式检查。一个路径最后匹配的模式是否定模式 (`!`) 时，算没有被忽略。有一个没被忽略，或者 git 确认不了，就一个文件都不写，公开的也不写。
- 只由私有层启用的规定，它们的 skill 和 `check = "command"` 检查不写进提交的文件，即 `.agents/skills/` 和 `statute-checks.toml` ([编译与网站](statute-command.md#compilation-and-the-site))；`statute check` 照样运行这些检查。私有规定的 skill 怎样安装，是一个尚未解决的问题。

放在 git 的目录里，是为了让一个 clone 的所有 worktree 共用一份私有规定清单。git 在各 worktree 之间共用 common 目录下 `common/` 里的一切 (gitrepository-layout(5))，而 `git rev-parse --git-path common/...` 从任何一个 worktree 都能得到这个位置；换成 git 不认识的路径，比如 `--git-path statute`，得到的是每个 worktree 各自的目录。放在工作区里的文件不会随 `git worktree add` 出现，因为它只检出 git 跟踪的文件，每个新 worktree 都得复制一份，之后各份就会不一致。放在家目录里、按仓库路径区分的目录也能共用，但 clone 一移动或删除，它就没人认领了，而且要设计一套把路径编码成文件名的规则；按 remote 地址区分，会把同一个 remote 的两个本该不同的 clone 合成一份，没有 remote 的仓库也没法区分。放在 git 目录里的代价是文件不显眼：写这两个文件的命令会打印它们的完整路径。重新 clone，或者不经 git 复制过去的仓库，一开始没有这两个文件。

**名字仍叫 `private`。** `local` 在 statute 里已经指一种范围 (`scope = "local"`、`L12` 和本地的两类规定源)，所以像别的工具给一台机器自己的设置起名那样叫 `statute.local.toml`，会被读成本地规定的配置。这两个文件的特点是不能公开，`private` 说的正是这一点。

**私有输出**是一个文件 `.statute/AGENTS.full.md`：提交进仓库的 `AGENTS.md` 加上只由私有层带进来的规定 (它自己的规定，以及只由它启用的公开规定)，每条放在它在规定树里的位置。有两个软链指向这个文件：

```text
.statute/
  .gitignore              statute 写的，要提交：列出下面的生成文件
  rules/                  仓库自己的规定，要提交
  AGENTS.full.md          公开规定与私有规定
  site/                   本地网站
CLAUDE.local.md -> .statute/AGENTS.full.md
AGENTS.override.md -> .statute/AGENTS.full.md
```

- Claude Code 读 `CLAUDE.local.md`，而且只要有这个文件就不再读 `AGENTS.md`，所以它只读全文。子目录里的 `AGENTS.md` 它也不读，见下文。这是在默认设置下；改成在 `CLAUDE.md` 之外也读 `AGENTS.md` 的设置时，两份它都读。
- Codex 和 Pi 读 `AGENTS.override.md`，用它代替同一目录下的 `AGENTS.md`，所以它们只读全文。
- Grok 既读 `AGENTS.md` 也读 `CLAUDE.local.md`，所以公开规定它会读两遍。两遍是同样的规定，互不矛盾；这样的重复可以接受。
- 仓库里有 `CLAUDE.md` 时，Claude Code 除了 `CLAUDE.local.md` 也会读它，于是它的内容叠在全文之上：它是指向 `AGENTS.md` 的软链或 `AGENTS.md` 的副本时，公开规定读两遍；否则多出来的文字可能与规定不一致。Grok 把指向同一个文件的 `CLAUDE.md` 和 `AGENTS.md` 算作一份，所以有没有这个软链，公开规定它都读两遍。`statute compile` 遇到 `CLAUDE.md` 会给出 warning ([编译与网站](statute-command.md#compilation-and-the-site))。
- OpenCode 暂不支持。
- 提交进仓库的 `AGENTS.md` 不指向 `.statute/AGENTS.full.md`：读了软链的 agent 会把全文读两遍，两个软链都没找到的 agent 读到的是公开规定。

两个软链经 `.git/info/exclude` 忽略，不写进任何 `.gitignore`：Grok 会跳过被 `.gitignore` 匹配的指令文件，判断依据是软链本身的路径，而且它不读 `.git/info/exclude`。所以 `statute compile` 用 `git check-ignore --verbose` 确认每个软链都被某个 `.gitignore` 之外的模式忽略，否则什么都不写，并列出要加进 `.git/info/exclude` 的几行。这两个路径上已有的文件不是这样的软链时，原样保留，编译失败。

**`.statute/` 里既有要提交的，也有不提交的。** `.statute/rules/` 里仓库自己的规定要提交；私有输出和本地网站 (见[本地网站](statute-command.md#the-local-site)) 不提交。所以仓库的 `.gitignore` 不写 `.statute/`：statute 自己写 `.statute/.gitignore`，就像 uv 在它建的 `.venv` 里、ruff 在它的 `.ruff_cache` 里那样，按名字列出生成文件：

```gitignore
# written by statute
/AGENTS.full.md
/site/
```

这样，statute 以后新加的文件在名单写上它之前会被跟踪，在 `git status` 里看得见；写成 `*` 再加例外，它就被藏起来了。每个要往 `.statute/` 写生成文件的命令，在 `.statute/.gitignore` 不存在时先写它，它以 statute 的那一行开头但内容不同时重写它；不以那一行开头的 `.statute/.gitignore` 是仓库自己的，命令什么都不写并失败。这个文件要提交，这样新 clone 的仓库在 statute 第一次运行之前就会忽略这些生成文件。上面那项 `git check-ignore` 检查仍在写每个文件之前进行。

Codex 读一个项目的指令文件，从仓库根目录到工作目录的各级文件合在一起，最多读到它自己的配置 `project_doc_max_bytes` 设的字节数，超出的部分截掉；statute 不给全文设上限。

有 `CLAUDE.local.md` 时，Claude Code 也不加载任何子目录里的 `AGENTS.md`：工作目录在那个子目录时启动不加载，碰到那里的文件时也不加载，而没有 `CLAUDE.local.md` 时这两种情况它都会加载。子目录的规定对它就丢了，除非模型碰巧自己去读那个文件。所以有私有输出的仓库把这类规定写进规定清单，不放在子目录的 `AGENTS.md` 里。编译私有层时，`statute compile` 遇到 git 跟踪的、在 `statute.toml` 所在目录 (通常就是仓库顶层，也是写软链的地方) 之下的 `AGENTS.md`，逐个给出 warning，说明 Claude Code 不加载它，其中的规定应当并入规定清单。statute 不把它们的内容编进全文。

**新的 worktree。** 私有规定清单、它的锁文件和 `.git/info/exclude` 由一个 clone 的各个 worktree 共用，但私有输出和两个软链是某一个工作区的文件，而 `git worktree add` 只检出 git 跟踪的文件。这些文件由 git 的 `post-checkout` 钩子写出。git 在 `git worktree add` 之后 (除非带 `--no-checkout`)、每次用 `git switch` 或 `git checkout` 切换分支之后运行它，运行位置是工作区的顶层 (githooks(5))；钩子放在 common 目录里，所以一个钩子对这个 clone 的所有 worktree 都生效。钩子只运行一条命令，`statute compile --private-only`：

```sh
#!/bin/sh
[ "$3" = 1 ] && statute compile --private-only; exit 0
```

检出分支时 `$3` 是 1，检出文件时是 0，所以私有输出跟着分支走，因为不同分支的 `statute.toml` 可能不同。钩子的路径是 `git rev-parse --git-path hooks/post-checkout` 打印的那个，它会遵循 `core.hooksPath`；钩子文件归某个钩子管理工具管的仓库，把这一行加在那里，pre-commit 也能安装这一类钩子。statute 不安装钩子，由用户在每个 clone 里装一次。`--private-only`：

- 只写 `.statute/.gitignore`、`.statute/AGENTS.full.md` 和两个软链，从不写 `AGENTS.md`、skill 或 `statute-checks.toml`；
- 读这个 worktree 的 `statute.toml` 和 `statute.lock`，以及私有规定清单和它的锁文件，不需要联网：git 规定源从 statute 的缓存里读 (见[statute 的目录](global-configuration.md#the-statute-directories))，什么都不 fetch；
- 没有 `statute.toml`、仓库是私有的 (`public = false`)，或者没有私有规定清单时，什么都不做，不出声，退出码为 0；
- 其余情况下，只要普通的 `statute compile` 会失败或需要联网，它就什么都不写，打印一条 warning，说明原因和要运行的命令，退出码为 0：锁文件过期、缓存里缺 commit、软链没有经 `.git/info/exclude` 忽略、规定清单解析不了，都是这样。钩子失败会让 `git worktree add` 和 `git switch` 以它的退出码退出 (githooks(5))，所以它从不失败。

软链用相对路径，指向这个 worktree 自己的 `.statute/`。

私有规定只补充公开规定，从不与之矛盾，正如私有层只增加规定 (见上文)：Grok 在全文之外还单独读公开规定，而追加读两份文件的 agent，在两份不一致时并不稳定地偏向哪一份。

各家 agent 的行为是 2026-10-14 在非交互模式下测的，版本是 Claude Code 2.1.296、Codex 0.159.3 和 Grok 1.0.50，每次运行都关掉了 Grok 的 folder trust 检查 (不关的话，Grok 在非交互模式下不加载任何项目指令)；Pi 1.2.0 每种情况跑了一次；Codex 的 `project_doc_max_bytes` 依据它的文档和源码。在一个小任务的几次运行里，兜底那一行写成 `@` 路径时，Codex 和 Grok 各两次都读了它指向的文件，Claude Code 则当作导入加载了它；换成同一类、但措辞与 statute 写的不同的一行 (路径写在反引号里，没有 `@`) 时，在别的都没加载全文的情况下，Claude Code 两次都读了全文，每次都是模型自己去读的。每次运行时用户自己的指令文件和 skill 都在，它们本身也催 agent 去找指令。布局本身用一份代替全文的文件测过：

- 软链 `CLAUDE.local.md` 经 `.git/info/exclude` 忽略时，Grok 2 次运行 2 次都加载了；经 `.gitignore` 忽略时，2 次运行 2 次都跳过。
- 仓库根目录有 `CLAUDE.local.md` 时，Claude Code 4 次运行 4 次都没有加载 `sub/AGENTS.md`，其中两次工作目录在 `sub/`，两次工作目录在根目录、改 `sub/` 里的一个文件；没有它时，两次对照运行都加载了，一次在启动时，一次在读了 `sub/` 里的文件之后。四次里有一次是模型自己去读了那个文件。
- 同时有 `CLAUDE.md -> AGENTS.md` 和 `CLAUDE.local.md` 时，Claude Code 在一次运行里把公开规定加载了两遍，只有 `CLAUDE.md -> AGENTS.md` 时一遍。Grok 把两者算作一份，依据是 `grok inspect` 列出的文件，没有跑模型。
- Pi 经 `AGENTS.override.md` 只加载了全文，不加载 `AGENTS.md`；软链经 `.git/info/exclude` 或 `.gitignore` 忽略都一样。
- 全文布局下，Claude Code 在默认设置时只加载了全文，根目录和子目录的 `AGENTS.md` 都不加载。

没有测的：交互模式；没有用户级指令和 skill 时的兜底那一行；子目录里的 `CLAUDE.md -> AGENTS.md`，以及它能不能让 Claude Code 重新读到那个子目录的规定；Claude Code 在 `CLAUDE.md` 之外也读 `AGENTS.md` 的那个设置，对子目录的效果；以及这个布局下 Grok 读不读子目录的 `AGENTS.md`。每个否定结论只对所数的那几次运行成立；其余的各只跑了一次。

私有层已按这里所写实现，但本节标明尚未实现的部分除外。

私有仓库 (`public = false`) 用不着这些：一份 `statute.toml` 写下所有规定源，输出照常提交。全局私有规定等用户级做出来以后归用户级 ([规定清单与锁文件](rule-sets.md#rule-sets-and-the-lock-file)，放在哪里一段)；在那之前，它们这样进入一个仓库，它们的规定源在全局配置里登记一次 ([全局配置与 git 规定源](global-configuration.md))。

## <a id="overrides"></a>覆盖

规定清单可以用自己的一条规定顶替另一个规定源的一条规定。多数偏离用不着这样做。公开规定只写原则，具体的值留给下面的层，由它们写成自己的规定来补充：比如一条公开规定写 worktree 放在主检出之外、单独的目录里，一条全局私有规定再写明具体目录，主检出放在 `~/dev`，worktree 放在 `~/wt`。这两条规定同时成立，后一条是对前一条的补充，不覆盖什么。公开规定要写成能这样补充的样子。

覆盖用在一条规定与规定清单的需要无法同时成立的时候。比如一条公开规定写 pull request 的描述用英文，这个值是规定源自己定的，没有留给下面的层；某个仓库的读者都读中文，它就写一条本地规定，规定描述用中文，覆盖那条公开规定。长远的解决办法是向规定源提议把这条规定改成只写原则的形式。编译出的文件里只有这条本地规定，放在被覆盖的那条原来的位置，另起一行写明覆盖了哪条，`Overrides R200.` ([编译与网站](statute-command.md#compilation-and-the-site))。那条公开规定并没有被取代，在其他规定清单里照样有效。

**怎样声明。** 覆盖方用一个字段写明它顶替哪条规定：

```toml
id = 3
kind = "rule"
overrides = "R200"
```

`overrides` 只写一条规定，写法与 `requires` 相同。一条规定要顶替两条，就是两项要求，应该写成两条规定。领域和 one-of 不能被覆盖：规定清单改选 one-of 的另一个子节点；规定源里没有想要的子节点，就向那个规定源提议加一个 (见[用 one-of 表达相互冲突的做法](#conflicting-practices-as-one-ofs))。带 `overrides` 的规定没有 `parent`，它的位置就是被覆盖那条的位置；写了 `parent` 构建就失败。

**谁能覆盖谁。** 规定源分三级，一条规定只能覆盖级别更低的规定源里的规定：

1. 第一方规定源 `R`，什么也不能覆盖；
2. 其余的全局规定源，公开私有都算：一个团队或一个人自己的规定，可以在他们所有的仓库里顶替一条第一方规定 (私有规定源的规定在公开仓库里只能覆盖只由私有层启用的规定，见下文)；
3. 本地规定：仓库自己的、放在 `.statute/rules/` 的规定，以及任何 `scope = "local"` 的规定源。

同一级的两条规定不能相互覆盖；同一个规定源里的规定只能原地修改或者被取代，不能被覆盖。允许一层套一层：本地规定可以覆盖 `team:7`，而 `team:7` 覆盖 `R200`；编译出的文件里只出现这条本地规定，那一行写「Overrides team:7, which overrides R200」。引用方向 (见[哪些规定可以引用哪些](#which-rules-may-cite-which)) 同样适用：公开规定不能覆盖私有规定。

**只在同一份输出里覆盖。** 覆盖只发生在编译进同一份输出的规定之间。私有仓库把所有层编译进一个文件，所以任何规定都可以覆盖级别更低的规定：被覆盖的规定不出现，覆盖方写明它覆盖了哪条。公开仓库里，私有层只增加规定 (见[公开仓库里的私有规定](#private-rules-in-a-public-repository))：它启用的规定覆盖了公开层的规定，即 `statute.toml` 直接启用或经依赖带进来的规定，就报错：

```text
error: `here:2` overrides "R200", a rule of the public rule set `statute.toml`; in a public repository (`public = true`), `private.toml` may only add rules
```

在私有层内部，一条规定可以覆盖任何只由私有层启用、级别更低的规定，因为两者都编译进私有输出；在公开层内部，一条规定可以覆盖级别更低的公开规定。两者都与私有仓库一样。

**构建怎样处理。** 覆盖方启用了，覆盖才生效：

- 被覆盖的规定，对所有指向它的要求、对它所在的 one-of 来说，都算已启用：要求 `R200` 的规定得到满足，`R200` 在 one-of 下的兄弟节点仍被排除。它自己的要求不带进来，带进来的是覆盖方的要求。它的 `parent` 是例外：`parent` 是规定时 (直接是规定，或者隔着一个 one-of)，它仍算覆盖方的一项要求，因为覆盖方站在被覆盖的那条的位置上。
- 被覆盖的规定上声明的冲突，以及别的规定针对它声明的冲突，都不适用；适用的是覆盖方的冲突。
- 它的子节点不变。
- 它的检查不跑，跑的是覆盖方的检查。
- 覆盖只在覆盖方启用的地方生效：规定清单按路径分了段的，只在启用它的那几段里生效。
- 启用的两条规定覆盖同一条规定，构建失败，报错里写出这两条。
- 覆盖方要顶替的那条规定没有被任何东西启用时，覆盖方编译在那条规定本该出现的位置，并给出警告，说那条规定不在规定清单里。

**锁文件**像记录其他规定一样记录被覆盖的规定，多一个字段：

```toml
[[rules]]
rule = "R200"
revision = 2
sha256 = "<SHA-256 of rules/200.toml>"
original-sha256 = "<SHA-256 of rule 200 without its translations>"
overridden-by = "L3"
```

这样规定源以后对 `R200` 的修改仍会出现在 `statute upgrade` 里，每一行后面注明它被 `L3` 覆盖，提醒用户检查这个覆盖是否仍然成立。

**和这些不是一回事。**

- 取代：规定源在所有规定清单里永久替换自己的一条规定 (见 [ID 与修订号](rule-format.md#ids-and-revisions))；覆盖是在一份规定清单里替换另一个规定源的规定。
- `conflicts`：两条规定不能同时启用，谁也不顶替谁 (见[依赖、one-of 与规定包](rule-sets.md#dependencies-one-ofs-and-packs))；覆盖是用一条规定顶替另一条。
- one-of：规定源预先想到的几种做法，各自是一条规定，每份规定清单选一条；覆盖是规定源没有想到的偏离。两份规定清单以同样的方式覆盖同一条规定时，就把这种偏离作为 one-of 的一个子节点向规定源提议。
- 去掉一条规定再加一条：规定清单就和原来那条断了联系。覆盖保留了这层联系：它在树里的位置、指向它的要求，以及原来那条改动时的提醒。
- 放宽 (见[放宽声明](rules.md#relaxation-declarations))：放宽是依据宪法的某项许可解除一条规定的一部分；覆盖属于一个法域选用哪些规定 (`division-of-instructions.rules`)，不需要许可。

## <a id="subdomains"></a>子领域

一个领域可以挂在另一个领域下面，成为子领域，比如 Git 挂在工作流下面，Rust 挂在编程语言下面。子领域就是一个 `parent` 指向某个领域的领域：

```toml
id = 105
kind = "domain"
parent = 104
```

- 领域的 `parent` 如果有，是一个领域，层数不限：同一规定源里的写编号，另一个规定源里的写法与 `requires` 相同 (`"R78"`)。它也可以是同一规定源里的一个 one-of，这个领域是它的一个选项 (见[规定不挂在规定下面](rule-config.md#no-rule-under-a-rule))。领域可以嵌套任意多层；领域的 `parent` 是规定，构建失败；各节点的 parent 成环，无论在一个规定源里还是跨规定源，构建也失败。引用方向同样适用：公开规定源的领域只能挂在公开的领域下面。
- 规定和 one-of 可以把子领域当作 `parent`，就像把领域当作 `parent` 一样；规定的 `parent` 从不是规定。
- 规定的 `parent` 也可以指向另一个规定源的领域或子领域，但不能指向另一个规定源的规定或 one-of。这样，一个仓库关于 Git 的本地规定会出现在通用的 Git 标题下面，而不是另起一个标题。引用方向同样适用：公开规定的 `parent` 必须是公开的领域。
- 出于同样的理由，一个规定源的领域也可以挂在另一个规定源的领域下面：私有规定源里关于用户自己那套 agent 的领域，作为第一方「agent」领域的子领域出现，它的规定在它下面。第一方规定源自己的领域是根：在它里面，只有规定可以有指向另一个规定源的 `parent` ([检查程序](checkers.md#checkers))。
- 把一个领域作为规定包启用，也就启用了它各个子领域下的规定；子领域本身也可以作为一个规定包的根。规定包是一个规定源里的一棵子树：挂在它的领域下面的别的规定源的规定或子领域，不在规定包里；规定要由规定清单在 `rules` 里列出来启用，别的规定源的子领域可以作为它自己的规定包启用。

**怎样显示。** 编译出的 `AGENTS.md` 不显示领域和子领域：它们的规定按树的顺序排成一个平铺的列表，顺序见[编译与网站](statute-command.md#compilation-and-the-site)：子领域的规定排在所属领域自己的规定后面；挂在一个领域下的别的规定源的节点，排在它自己规定源的节点后面，和领域本身的排法一样。在网站上，子领域像领域一样作为它下面那些规定的标题，嵌套在所属领域的标题下面；它的页面列出下面的节点 (见[静态网站](static-site.md))。

## <a id="conflicting-practices-as-one-ofs"></a>用 one-of 表达相互冲突的做法

几个仓库有充分的理由用互不相容的方式工作时，就把这几种方式写成一个 one-of 的子节点，每份规定清单选一条。不写「做 A，除非……」这样的规定，再在每个仓库需要例外时打一次补丁。

设计怎样跟上代码，就是这样的一对做法。文档领域下面有一个 one-of「让设计与代码保持一致」，它有两个子节点 (这里的 ID 只是占位；规定本身写在 `rules/` 里，不写在设计里)：

- `<A>`：设计和它描述的代码在同一个 pull request 里改。
- `<B>`：代码可以先于设计合入，也可以晚于设计合入，但每一处不一致都要标明：设计里还没实现的部分写明尚未实现；pull request 让设计落在代码后面的，在描述里写明，并留下一项 follow-up。

选 B 的理由：先设计再写代码很正常；想快速推进的项目让代码先行；设计集中在一处时，并行的修改合并时会冲突。规定清单在 `rules` 里启用其中一条，和其他 one-of 一样；启用 `<B>` 的规定清单不需要放宽什么，也不需要覆盖什么；monorepo 可以按路径分段各选各的。「设计是描述性的，不是指令」「设计按主题拆成多个文件，让并行的修改少碰在一起」，这些是与这个 one-of 并列的单独规定，不是写在某个子节点里的条件。

## <a id="cutover-from-the-hand-written-guide"></a>从手写的指南切换过来

手写的指南等到每一部分都有了去处，并且下面四个条件都成立时，再一步整体换成四类规定，指南管着的所有仓库同时切换。没有过渡条款：切换之后，没有哪段文字让指南的一部分继续有效；也不先切某一个主题 (比如凭证)，再切其他的。只适用于某个环境的内容本来就不会进第一方规定源，也不必留在指南里等：它从一开始就是全局私有规定或本地规定。

**清点。** 指南的每一项都有且只有一个去处，记在私有一侧的对照表里，做法同[从现有的手写的指南迁移](fleet-and-migration.md#migrating-from-the-current-hand-written-guide) 第 3 步，只是其中的「私有规定」分成三种：

- 宪法；
- 第一方规定 (`R`)：任何仓库都能照原样使用的；
- 全局私有规定：对这个人的所有仓库都成立，但写到了他们的机器、账号、bot 或目录布局的；
- 某个仓库的本地规定，仓库公开就是公开的，私有就是私有的；
- 参数、skill (操作步骤)、来历 (放进私有的案例库)，或者删掉。

某个仓库偏离指南的地方 (那份指南的第 6 步) 改成一条覆盖、一个 one-of 的子节点，或者删掉。

**完成的判据**是下面几条都成立，在私有一侧检查：

1. 指南的每一项在对照表里都有去处，去处是规定的，写明它的引用 (`R42`、`me:4`、`L12`)。没有一项待定。
2. 任何一类规定都不提指南、指南的文件或指南条目的编号。私有词扫描 (见[检查程序](checkers.md#checkers)) 把指南的文件名和条目编号的模式作为扫描词来跑，每个模式先在一行必定命中的文本上跑一次。
3. 指南管着的每个仓库都有规定清单，`statute lock` 和 `statute compile` 对它都成功，私有规定清单也在内。编译出的输出已经与指南逐节对比过，每一处差异都记为有意为之。
4. 编译出的输出通过[加载验证](checkers.md#load-verification)。

然后由私有一侧的一次修改装上编译出的输出、替换指南，并停用指南。

## <a id="what-this-changes"></a>它改变什么

设计里受这一部分影响的地方已经按它写好，并各自标明尚未实现的内容：

- [规定文件格式](rule-format.md#rule-file-format)：规定的 `parent` 可以指向另一个规定源的领域，领域可以有 `parent`，在自己的规定源或另一个规定源里，规定可以用 `overrides` 代替 `parent`。
- [规定清单与锁文件](rule-sets.md#rule-sets-and-the-lock-file)：`public`、`scope`、私有规定清单，以及锁文件里的 `overridden-by`。
- [依赖、one-of 与规定包](rule-sets.md#dependencies-one-ofs-and-packs)：任意多层嵌套的领域。
- [编译与网站](statute-command.md#compilation-and-the-site)：子领域的规定在平铺列表里排在哪里。
- [命令与第一版](statute-command.md#the-command-and-the-first-version)：目前实现了什么，以及按上面的切换来切换。
- [检查程序](checkers.md#checkers)：`statute validate` 在一个规定源之内对新字段查什么，跨规定源的哪些由 statute 查。
- [选用规定](rules.md#choosing-rules)：修改一条规定用覆盖。
