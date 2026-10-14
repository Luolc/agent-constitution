<!-- en-sha256: 781a912fdb6a42e660cce7910b3ae76026b8f94ed951d768e95f0df218e955ca -->
# 文件结构

各阶段沿用本仓起步时的计划：A 是宪法和本设计，B 是结构化的源文件和确定性的编译，C 是反馈与提案 (只观察)，D 是限定范围的自动采纳。C 和 D 的目录只预留，不创建。

```text
README.md                 本仓首页，英文                               A
README.zh-CN.md           同一页的中文版                               A
LICENSE                   Apache-2.0，适用于下面那部分文字以外的一切   A
LICENSE-CC-BY-4.0         CC BY 4.0，适用于 constitution/、rules/
                          和 skills/ 的文字                            A
AGENTS.md                 在本仓工作的 agent 遵守的仓库规则            A
constitution/
  <YYYY-MM>/              版目录，目前是 2026-10 (constitution.md)     A
    index.toml            状态、版、修订号、语言、标题、卷首           A
    articles/
      <NN>-<id>/          一条一个目录，带编号                         A
        article.toml      条本身：标题、引言                           A
        <NN>[a-z]-<id>.toml  一款一个文件，带编号：状态、
                          各语言的正文、项                             A
    generated/            由源文件构建，提交入库                       A
      constitution.en.md  英文宪法                                     A
      constitution.zh.md  中文宪法                                     A
      constitution.en.agent.md  agent 版，英文                         A
      constitution.zh.agent.md  agent 版，中文                         A
docs/
  design/                 本设计，一部分一个文件，
                          由 README.md 索引                            A
rules/
  <id>.toml               一条通用规定一个文件，含各语言的正文         A
  retired-ids.txt         已删除规定的 ID，一行一个                    A
skills/
  <name>/SKILL.md         一个通用 skill 一个目录，模板文件放在
                          旁边，原样安装                               A
schema/                   规定、fixture 和 overlay manifest 的 schema  B
tools/                    一个 uv workspace，每个子目录一个 Python 项目：
  compiler/               宪法编译器，包名 constitution_compiler，
                          命令 constitution-build                      A
    src/constitution_compiler/
      build.py            用下面这些模块构建并检查宪法                 A
      model.py            源文件在哪里、文件名的格式、
                          读入后存放的记录结构                         A
      schema.py           schema，以及检查 1 中逐表的格式检查          A
      read.py             读入源文件，检查 1                           A
      checks.py           检查 2 至 5                                  A
      languages.py        各语言之间不同的地方                         A
      render.py           渲染宪法                                     A
      base.py             检查 7，对照基准提交核对编号，
                          以及对 git 的访问                            A
      revision.py         检查 7，对照基准核对修订号                   A
    tests/                渲染、检查、基准的测试，以及共用的测试
                          树 (test_constitution_fixture.py)            A
  site/                   包 constitution_site，依赖 compiler          A
    src/constitution_site/
      count_tokens.py     统计静态网站显示的 token 数
                          (count-tokens)                               A
      check_site_anchors.py  检查构建出的网站的链接
                          (check-site-anchors)                         A
    tests/                它们的测试，以及 test_site.py，即网站
                          Hugo 模板的测试                              A
  checks/                 包 repo_checks，本仓自己的检查               A
    src/repo_checks/
      check_instruction_changes.py  指令文件单独变更
                          (check-instruction-changes)                  A
      lint_zh_sources.py  对 zh 源文本跑 limae (lint-zh-sources)       A
    tests/                它们的测试                                   A
  <others>                编译器、检查程序、私有词扫描程序             B
examples/
  overlay/                一份虚构的私有 overlay                       B
  golden/                 它的编译输出和构建 manifest                  B
fixtures/
  static/                 确定性的 fixture，happy 与红线               B
  behavior/               真实 agent 的场景                            C
cases/<id>.md             已裁决的案例                                 C
proposals/                为采纳流水线预留                             D
web/                      静态网站的 Hugo 项目：hugo.toml、
                          content adapter、布局、样式表、
                          脚本、i18n                                   A
  themes/hugo-book/       主题，一个固定在某个提交的 git submodule     A
site/                     构建出的静态网站，不提交                     A
.github/workflows/        CI                                           A
.github/assets/           README 的图标、示意图和截图，
                          分浅色与深色                                 A
.pre-commit-config.yaml   提交钩子                                     A
pyproject.toml            workspace 的根：成员、Python 版本、
                          limae、pre-commit 和 Python 检查程序的
                          版本固定，以及它们的配置                     A
uv.lock                   workspace 唯一的锁文件，由 uv 写出           A
.python-version           uv 运行 tools/ 所用的 Python 次版本号        A
.fleet/config.json        给 fleet 用的 issue 跟踪元数据；
                          工具链从不读它                               A
```

有三项阶段 B 的内容提前开始。`tools/` 随宪法的构建在阶段 A 开始：宪法本身就是从一条一个目录、一款一个文件的源文件编译出来的 (见[源文件格式](source-formats.md))。`rules/` 也在阶段 A 开始，放从宪法中移出的具体做法。`skills/` 在阶段 A 只是普通的目录，每个目录放一个 `SKILL.md`，带通常的 skill front matter (`name`、`description`)，不带本仓自己的元数据。编译和检查规定、构建指令文件，仍留在阶段 B。

阶段 B 按下面的顺序实现：`schema/` 和 schema 检查程序，`rules/` 的其余部分 (由迁移填充)，编译器，`examples/` 和漂移检查，引用检查程序和放宽检查程序，私有词扫描程序，`fixtures/static/` 和 fixture 检查，最后是加载验证。每一步单独就能用。

宪法中关于修改的那一条所保护的，是宪法的正文 (`protected-text`)；在核心部分，它是版目录下的源文件 (`index.toml` 和 `articles/`)，以及由它们构建出的文件的措辞。改动正文就是修正案，按 `approving-amendments` 的规定生效 (见[批准修正案](amendments-and-releases.md#approving-an-amendment))。其余一切都是依照 `rule-evolution` 的普通变更 (`ordinary-tooling`)，与其他变更一样审查和测试：`tools/` 和 `schema/` (编译器、检查程序、fixture 检查的运行程序)、`.github/workflows/`、`fixtures/` 和 `examples/golden/`。人的注意力放在宪法说了什么上；工具靠单元测试和正常路径的端到端测试保证可靠，不靠人去读。生成文件只能通过重新构建来改。对构建脚本的改动，如果只改变生成文件的编号或版式，是普通变更，它的 pull request 要展示生成文件的 diff，让人扫一眼格式；如果改变了它们的措辞，就改变了正文，是修正案 (`protected-text`)。构建脚本自己写的说明 (文件头一行、废止说明，以及 agent 版里的开头说明和代替条文正文的那句说明) 不是宪法的正文：它们随脚本一起改，走普通变更 (`ordinary-tooling`)，只能说正文已经说过的话 (宪法约束每一个 agent，`terms.agent`)、指向完整的正文、说明版式；会改变含义的说明，要先有修正案。agent 版略去哪些条文由源文件设定 (`audience`，见[宪法](constitution.md#constitution))，所以改动它是修正案。
