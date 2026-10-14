<!-- en-sha256: c20c43469aea896ee3f778f3b84ca80ec800aed51c3bc5c127bb6026d46b5bb5 -->
# 修正案与发布

## <a id="approving-an-amendment"></a>批准修正案

这套做法的目的是让人在批准上花尽可能少的时间，这正是 `purpose` 的要求。人只读宪法说了什么，别的都不读：工具、检查和 fixture 都是普通变更 (见[文件结构](file-structure.md))。

宪法还是草案时，修正案的 pull request 通过 agent 审查后即合并，人事后在网站的预览上阅读；描述仍按下面的方式写，以便读得快。只有对已批准的一版的修改，才要等人对一个指明的 head 提交同意之后再合并。本仓怎样保存现行版和下一版的草案，由 `text-and-sources` 和 `draft-and-edition` 规定。

- 修正案的 pull request 只改 `constitution/`：源文件，以及从源文件重新构建出的生成文件。
- 描述以人所读语言的生成文本的 diff 开头，然后是其他每种语言的同一份 diff，折叠起来。生成文本的 diff 就是 agent 将要读到的内容，编号也包括在内；人不需要读源文件。
- 在问人之前先完成 agent 审查：CI 的宪法检查是绿的，审查方也核对过所有语言的文本说的是同一件事。人批准这次变更，就是基于这次审查批准所有语言的文本。
- 描述以待决问题结尾，每个问题都清楚、容易回答，并附推荐的答案 (`asking`)。
- 对已批准的一版，人的同意要指明所批准的 head 提交，合并的正是这个提交 (`independent-review`)。
- `proposing-amendments` 要求提供批准人所读语言的生成文本 diff，`amendment-languages` 要求所有语言的文本一起修改；上面这些步骤就是本仓满足这两项要求的做法。如果 head 之后又变了，再问一次人，只给出自所批准的 head 以来的 diff。

## <a id="releases-and-versions"></a>发布与版本

- 版本是 `main` 上的 tag `v<major>.<minor>.<patch>`。改变行为的宪法修正案或规定变更，至少升一个 minor 版本；改变输出的工具变更，至少升一个 minor 版本；其余的升 patch 版本。
- 发布 tag 只打在这样的 `main` 提交上：它的 push 触发的那次 `check` 运行第一次就成功了。重跑之后才变绿的不算；改为给之后一个第一次就通过的提交打 tag。
- 推送发布 tag 就使一个版本可以获取。默认情况下，发布是 `must-ask` 的一个事项，由规定 `must-ask-items` 加入，需要人同意；项目可以依照 `relaxations.project-overrides` 另行规定。
- `crates/` 下的 crate 发布到 crates.io，各自的版本写在自己的 `Cargo.toml` 里；发布一个 crate 也是一次发布。`statute` 0.0.1 是占住这个名字的占位版本。
- overlay 固定一个 tag 及其提交 SHA，两者对不上就拒绝构建。它换用新版本，要在 overlay 里提一次变更，在那里审查。
