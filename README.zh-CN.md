<div align="center">

<a href="https://agent-constitution.luolc.com/zh/"><picture><source media="(prefers-color-scheme: dark)" srcset=".github/assets/icon-dark.svg"><img src=".github/assets/icon-light.svg" width="112" alt="Agent 宪法"></picture></a>

# Agent 宪法

### 给所有 coding agent 的一部宪法，以及细化它的规定。

**一部宪法和一套规定库，**<br>
按仓库编译成 agent 读的 AGENTS.md，<br>
再用检查判断规定有没有守住。

[![CI](https://github.com/Luolc/agent-constitution/actions/workflows/ci.yml/badge.svg)](https://github.com/Luolc/agent-constitution/actions/workflows/ci.yml) [![License: Apache-2.0 and CC BY 4.0](https://img.shields.io/badge/license-Apache--2.0%20%C2%B7%20CC%20BY%204.0-23301f)](#许可)

**[在线阅读](https://agent-constitution.luolc.com/zh/)** &nbsp;·&nbsp; **[设计](https://agent-constitution.luolc.com/zh/design/)** &nbsp;·&nbsp; **[规定](rules/)** &nbsp;·&nbsp; [English](README.md) · **简体中文**

</div>

<br>

AGENTS.md 一类给 coding agent 的指令文件，越改越长、越来越难读：前后矛盾，与代码脱节，原则、做法和流程混在一起。本仓照法律的写法来写它们：一部简短的宪法写原则，规定谁能决定什么；带编号的规定细化宪法，每条只提一项要求，并写明怎样判断它有没有被违反；命令 `statute` 把一个仓库选中的规定编译进它的 AGENTS.md，并运行这些规定的检查。

<picture>
  <source media="(prefers-color-scheme: dark)" srcset=".github/assets/architecture-zh-dark.svg">
  <img src=".github/assets/architecture-zh.svg" alt="Agent 宪法的结构：宪法由规定细化；仓库在 statute.toml 里选规定，由 statute.lock 锁定；statute 把它们编译成 AGENTS.md，并写出检查；agent 依此工作、审查；遇到没有规定覆盖的缺口，就写一条试行规定。">
</picture>

## 宪法

现行的是 2026-10 版，共十一条，讲安全、agent 依谁的授权行事、委托、验证、规定的位阶与演进，以及宪法本身怎样修改。宪法不提任何命令或工具，所以下面的规定怎么变，它都不用跟着变。

宪法有英文和中文，每种语言各有两个版本：全文，以及给 agent 读的精简版，省去只写给人看的部分。可以在 **[网站](https://agent-constitution.luolc.com/zh/)** 上读，也可以看仓库里的 [`constitution/2026-10/generated/`](constitution/2026-10/generated/)。源文件是 TOML，每节一个文件，在 [`constitution/2026-10/`](constitution/2026-10/) 下；编号和修改的规矩见 [宪法](docs/design/zh/constitution.md)。

## 规定

每条规定是一个文件 [`rules/<id>.toml`](rules/)，用英文写，附中文译文。规定的 ID 是数字，永不复用；正文分要求、理由和判定方法三部分，都写得够短，能够检查。规定挂在 domain 下面，例如安全、协作、验证、文档，每条都注明它细化的是宪法的哪些条款。宪法还是草案时写的规定，状态是 `draft`。agent 发现某件事没有规定覆盖、动手写下的约定，先是 `trial` (试行) 规定，在实际工作中站住了脚，再转为 `active`。规定从不删除，只会被取代 (superseded) 或废止 (repealed)。

文件格式、规定树、规定包和审查循环见 [规定体系](docs/design/zh/rule-system.md)。

## command: statute

`statute` 是规定体系的命令，用 Rust 写，在 [`crates/statute`](crates/statute/)。它还在开发中：目前只从本地磁盘上的源读规定，也就是本仓的一份检出；crates.io 上的版本还只是占位。从源码构建：

```sh
git clone https://github.com/Luolc/agent-constitution.git
cargo install --path agent-constitution/crates/statute
```

然后在 agent 工作的那个仓库里 (这里假定克隆的目录就在它旁边)：

```sh
statute init --source ../agent-constitution  # 写出 statute.toml
statute add 43        # 启用规定 43，并写出 statute.lock
statute lock          # 手改 statute.toml 之后重新解析
statute compile       # 写出 AGENTS.md、skill 和检查
statute check         # 运行已启用规定的检查
```

`statute check` 每条规定输出一行结果：`pass`、`violation`、`inconclusive`，没有脚本能判定的规定是 `review`；对结果为 `inconclusive` 或 `review` 的规定，后面再附上它的理由和判定方法，给审查方看。有规定被违反时退出码为 1。它在 CI 里和每次 push 之前运行，不作为提交钩子，因为检查可能很慢。`statute explain <id>` 说明某条规定为什么被启用，其余命令见 `statute --help`。目前做到哪一步，见 [命令与第一版](docs/design/zh/rule-system.md#the-command-and-the-first-version)。

## 开发

[设计](docs/design/zh/README.md) 写了目录结构、源文件格式、检查和发布，[AGENTS.md](AGENTS.md) 列了各条命令。简单说，需要 [uv](https://docs.astral.sh/uv/) 和 Rust 工具链：

```sh
uv sync                          # Python 工具，装进 .venv
uv run pre-commit install        # 提交钩子
uv run constitution-build        # 重新生成宪法
cargo test --locked              # statute 的测试
```

## 许可

宪法、规定和 skill 的文字 (`constitution/`、`rules/`、`skills/`) 采用 [CC BY 4.0](LICENSE-CC-BY-4.0) 许可。其余部分，包括 `tools/` 和 `crates/` 下的代码，采用 [Apache-2.0](LICENSE) 许可。网站的主题 (`web/themes/hugo-book`，一个子模块) 和取自 Octicons 的图标沿用各自的 MIT 许可。
