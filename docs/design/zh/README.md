<!-- en-sha256: a86e7c6444b9ee026836fb0fe103970f23cd6355336bd1193a0030da816c4a4a -->
# agent-constitution 设计

状态：草案。这是本仓唯一的一份设计，由下面列出的几个文件组成。宪法本身从它的版目录 `constitution/2026-10/` 下的 `index.toml` 和 `articles/` 构建成两个版本：[英文版](../../../constitution/2026-10/generated/constitution.en.md)、[中文版](../../../constitution/2026-10/generated/constitution.zh.md)；本设计只按 ID 引用宪法 (见[源文件格式的 ID 一节](source-formats.md#ids))，从不复述它的内容。

本仓提供三类可以安装的内容：宪法、通用规定和通用 skill。用户通过配置选择安装其中哪些。怎样安装、需要哪些元数据，还没有定，会在开发工具链的过程中确定。

## <a id="contents"></a>目录

每个文件讲一个部分，可以单独读；按下面的顺序读，就是整份设计。

- [概览](overview.md)：目标与非目标、两层结构、语言。
- [文件结构](file-structure.md)：目录规划、各阶段，以及宪法保护哪些内容。
- [源文件格式](source-formats.md)：TOML 文本表和 ID。
- [宪法](constitution.md)：宪法的源文件、编号和 agent 版。
- [规定与放宽](rules.md)：规定的源文件、选用规定、放宽声明和本地参数。
- [Skill](skills.md)：skill 的目录结构。
- [编译](compilation.md)：怎样构建指令文件。
- [静态网站](static-site.md)：Hugo 网站及其部署。
- [检查程序、fixture 与反馈](checkers.md)：检查程序、fixture、加载验证，以及预留的反馈回路。
- [修正案与发布](amendments-and-releases.md)：批准修正案、发布与版本号。
- [fleet 与迁移](fleet-and-migration.md)：与 fleet 的关系，以及怎样从手写的指南迁移过来。
- [规定体系](rule-system.md)：规定体系 (规定编号、规定源与引用、规定清单与锁文件、依赖、检查与审查回路)，以及它将取代上面哪些部分；目前只实现了仓库级，且只支持一个规定源；各个主题在下面四个文件里。
- [规定格式](rule-format.md)：一条规定包含什么、ID 与修订号、规定文件格式、规定与 skill、从规定追溯到宪法。
- [规定清单、引用与依赖](rule-sets.md)：规定清单与锁文件、规定源与引用、依赖、one-of 与规定包。
- [规定检查与审查](rule-checks.md)：检查、结果与审查回路、试行规定。
- [statute 命令](statute-command.md)：编译与网站、命令与第一版、为什么用 Rust。
- [规定源](rule-sources.md)：四类规定源 (全局或本仓、公开或私有)、哪些规定可以引用哪些、覆盖、子领域，以及从手写的指南切换过来；尚未实现。
