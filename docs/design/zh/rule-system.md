<!-- en-sha256: 9e32d8cbf7769a4eef00efe19b0bb078ce15acd69fcee7d0ca209a97d1b3e003 -->
# 规定体系

状态：草案。本仓的规定已经按[规定文件格式](rule-format.md#rule-file-format) 编写，由 `constitution-build` 检查。命令 `statute` 已经实现了本设计仓库级的一部分，支持磁盘上的规定源和 git 规定源：它能写出、解析和升级规定清单及其锁文件，把它们编译成一份 `AGENTS.md`，连同启用的规定需要的 skill 一起写出，还能运行检查。目前实现了什么、还没实现什么，见[命令与第一版](statute-command.md#the-command-and-the-first-version)；用户级还没有实现。设计的这一部分由下面几个文件组成，描述规定怎样编写、编号、选用、检查和编译，包括还没实现的部分。凡与设计其余部分目前的说法不同之处，都列在下文的[它将取代什么](#what-this-replaces)；那些部分保持原样，等到实现本设计的工作再去修改。

## <a id="contents"></a>目录

- [术语](#terms)
- [规定格式](rule-format.md)：一条规定包含什么、ID 与修订号、规定文件格式、规定与 skill、从规定追溯到宪法。
- [规定清单、引用与依赖](rule-sets.md)：规定清单与锁文件、规定源与引用、依赖、one-of 与规定包。
- [规定检查与审查](rule-checks.md)：检查、结果与审查回路、试行规定。
- [statute 命令](statute-command.md)：编译与网站、命令与第一版、为什么用 Rust。
- [它将取代什么](#what-this-replaces)
- [待决问题](#open-questions)

## <a id="terms"></a>术语

- **规定**：细化宪法某一条文的一项要求 (`division-of-instructions.rules`)，附有判断它是否被违反的方法。
- **规定源**：存放规定的仓库。本仓是一个；自己编写规定的团队把规定放在自己的规定源里 (见[规定源与引用](rule-sets.md#sources-and-references))。
- **规定清单**：一个仓库选用的规定，写成一个列出规定 ID 和规定包的配置文件；它可以继承另一份规定清单。
- **领域 (domain)**：parent 树顶层的节点 (Python、CI、Git、安全等)，只提供一个标题 (见[依赖、one-of 与规定包](rule-sets.md#dependencies-one-ofs-and-packs))。
- **one-of**：parent 树上的一种节点，它的子节点至多启用一个；一个都不启用也可以 (见[依赖、one-of 与规定包](rule-sets.md#dependencies-one-ofs-and-packs))。
- **锁文件**：写在规定清单旁边；它固定每条启用规定的修订号，并把每个规定源解析到一个确切的提交。
- **检查**：针对一条规定，判断一个仓库、一次变更或一份记录是否合规的代码。
- **审查方**：对检查定不了的情况作出判断的 agent。

## <a id="what-this-replaces"></a>它将取代什么

[规定文件格式](rule-format.md#rule-file-format) 已经实现，[ID](source-formats.md#ids)、[源文件格式](source-formats.md) 和[规定](rules.md#rules) 现在描述的就是这种格式的规定。[Skill](skills.md) 写明了 `statute compile` 会在仓库里安装启用的规定需要的 skill。本设计的其余部分实现之后，会取代当前设计的另一些部分。那些部分现在不改；等实现本设计的工作合入时再改。

- [选用规定](rules.md#choosing-rules)：带锁文件、在其中列出规定 (按 ID 或规定包) 的规定清单，取代「默认使用全部通用规定，再点名排除」。规定清单属于一个仓库，可以继承一份基础规定清单，由后者接管法域的那部分职能。每份规定清单还写明它依据的宪法 (见[规定清单与锁文件](rule-sets.md#rule-sets-and-the-lock-file))。
- [放宽声明](rules.md#relaxation-declarations)：规定清单不用某条规定，或者在 one-of 里选了另一个子节点，都不需要放宽。`adjustable` 的值和 `relaxes:` 声明是否保持原样，在实现本设计时决定。
- [概览](overview.md)：「每个法域自己选用哪些通用规定」这项目标和私有 overlay 选用规定的职责，移到每个仓库的规定清单 (连同继承的基础清单)，overlay 的 apply 步骤变成编译命令；overlay 保留与个人相关的部分和它自己的规定。
- [编译](compilation.md)：`statute compile` 已经能写出仓库的 `AGENTS.md`，并在旁边写出 skill 和检查 ([编译与网站](statute-command.md#compilation-and-the-site))。[编译](compilation.md) 描述的仍是计划中用户级文件的编译器，其中规定按第一个 `authority` 所在的条分组；等规定体系实现了用户级再改 (见[待决问题](#open-questions))。
- [检查程序](checkers.md#checkers)：增加由 Rust 二进制文件运行的规定检查及其三种结果；本仓源文件现有的检查程序保留。

## <a id="open-questions"></a>待决问题

- 公开规定源的索引：格式、谁可以往里加、什么时候构建。
- 跨规定源的版本具体怎样解析：各规定源互相声明的范围怎样与 tag 和分支匹配，对不上时构建报告什么。
- monorepo 的路径分段：交互模式下 agent 的行为 (没有测过)，以及仓根文件里的规定怎样写明它适用的目录，这套语法还没定。
- 用户级的规定：默认层级为用户级的规定，在哪里选用、在哪里锁定、编译到哪里。
- 命令的正式名字，在第一次正式发布前定下。`statute` 已由一个占位版本占住；换别的名字就要另行发布。
