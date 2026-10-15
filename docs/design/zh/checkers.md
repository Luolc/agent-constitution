<!-- en-sha256: 593643c3f3bd8af03aada2aae8c1657ac615e85fd6bef6625fd671aca6d370ae -->
# 检查程序、fixture 与反馈

## <a id="checkers"></a>检查程序

每个检查程序都是一条命令，失败时以非零状态退出，并打印文件和位置。每个检查程序证明什么、不证明什么：

| 检查程序 | 证明 | 不证明 |
| --- | --- | --- |
| 宪法 (`constitution-build --check`) | 每份宪法源文件的格式与文件名都符合这几处的要求：[源文件格式](source-formats.md)、[宪法](constitution.md)；每个 ID 格式正确，在自己的命名空间里唯一；编号在各自的范围内唯一；同一段文本的各语言引用相同的 ID；每处对宪法的引用和每条规定的 authority 都能解析到一个未废止的条文，宪法只引用自身，每条规定的状态与宪法的相符；提交入库的输出与重新构建的结果相同；带 `--base` 时，基准为现行版的，版与基准相同或比它新；在基准的那一版里，基准中有编号的内容都没有消失、改号或移动，基准中的规定都没有消失；带 `--base` 时，不论基准是什么状态，在同一版内，以及基准是草案时不论两边各写的是哪一版，修订号在宪法正文有改动时大于基准的，没有改动时等于基准的 | 两种语言意思相同，或者一种语言的改动已经带到另一种语言 |
| 规定 (`statute validate rules`) | 每个规定文件符合[规定文件格式](rule-format.md#rule-file-format)，树符合[规定不挂在规定下面](rule-config.md#no-rule-under-a-rule) 的形状，每个引用的目标存在、种类也对，每份译文的哈希和引用与原文相符，默认值能单独解析 (见下文) | 规定说的是它该说的；没人标为冲突的两条规定能一起用 |
| 指令文件单独变更 (`check-instruction-changes --base`) | 一项变更自基准以来改动的文件，要么全是指令文件，要么全是其他文件，生成文件不计；改名按旧路径和新路径各算一次 | 路径表是完整的：agent 当作指令读、但表里没有列出的文件，会被当成其他文件放过。工具的改动重新生成了某个生成文件时，它只是渲染方式变了；这一点由审查方核对 |
| 静态网站 (`hugo-0.167.0 --panicOnWarning`，再跑 `check-site-anchors`) | Hugo 构建所有页面时没有警告，设计文档里指向设计文件的相对链接都指到它的页面，也没有链接指出仓库之外；每个页面上，每个页内链接 `#id` 都有带这个 id 的元素，每个指向本站其他页面的链接 (相对路径，或者以 `/` 开头、从站点根算起) 都指向站内的一个文件，带片段的还要在那个页面上有带这个 id 的元素，没有重复的 id，列表符号也没有作为字面文字显示出来 (`- §`、`- 第`) | 页面看起来正确，或者字体能加载 |
| 中文排版 (`lint-zh-sources`；对仓里每个 Markdown 文件跑 limae) | 宪法源文件和规定源文件里的每段 `zh` 文本、构建出的中文宪法的两个版本，以及设计的中文译文，都通过 limae 的默认规则；对宪法的引用按它渲染出的中文来判断 | 中文读起来自然，或者与英文意思相同；英文文本也过 limae 的规则，那里多半查不出什么 |
| 设计译文 (`check-design-translation`) | `docs/design/zh/` 里的文件与 `docs/design/` 一一对应，不多不少；每份译文开头是英文文件当前的 SHA-256，接着是标题，标题以下的各级标题与英文文件相同，顺序和级别都一样，每个都带着英文标题的锚点；设计里的每个相对链接，无论哪种语言，都指向仓里的一个文件或目录 | 译文说的就是英文说的，或者更新哈希那一行时译文确实同步改过了 |
| Python 代码 (`ruff check`、`ruff format --check`、`pydoclint`、`basedpyright`，在 `pyproject.toml` 里配置) | `tools/` 下的脚本符合所配置的 lint 规则和格式，docstring 写明了每个参数和返回值，类型检查程序在 standard 模式下没有发现错误 | 脚本做的就是 docstring 说的；那要靠测试 |
| Rust 代码 (提交钩子 `cargo fmt --check`；CI 里的 `cargo clippy --all-targets -- -D warnings`、`cargo test` 和 `cargo publish --dry-run -p statute`) | `crates/` 下的 workspace 格式正确，clippy 没有警告，测试通过，`statute` 能像 crates.io 收到它时那样打包和构建 | 测试没覆盖到的地方，命令的行为也与设计一致，或者 crates.io 会接受这次上传 |
| Schema | fixture 和 overlay manifest 有必需的键，取值都在允许的范围内；ID 与文件名一致 | 正文的意思与字段所说的一致 |
| 引用 | 每个被引用的 ID (宪法条文、规定、案例) 都存在且未废止；没有源文件指向核心或 overlay 以外的文件 | 被引用的那一条确实支持这条规定 |
| 漂移 | `examples/golden/` 与编译器今天从 `examples/overlay/` 构建出的结果相同 | 真实的 overlay 能构建成功；它只说明编译器的行为没有在没人注意的情况下改变 |
| 私有词扫描 | 在 CI 里，通用类别 (密钥管理器路径、私有地址段、家目录路径、邮箱地址) 没有命中；overlay 运行它时，私有词表没有命中 | 不存在任何私有内容：不在词表里的词会被放过 |
| 放宽 | 每条 `relaxes:` 都写明了 `relaxations` 中一个确实存在、对该位阶开放的项，一个确实存在的对象，一个适用范围和一个条件；依照 `relaxations.adjustable-rules` 时，目标规定把该参数列为 `adjustable` | 条件确实满足，或者不存在未声明的放宽 |
| fixture 检查 | 每个 fixture 都绑定到存在的 ID，并且通过；红线 fixture 从不跳过，只有某个法域去掉的规定的 fixture 除外，它们在该法域的 manifest 里列为未运行 | 在这些情形以外，agent 的行为也符合 fixture 的预期 |

宪法检查按下面的顺序运行各项检查，在第一项发现错误的检查之后停下，打印这一项的所有错误：

1. 格式：每个 TOML 文件都能解析；必需的键一个不多、一个不少，类型正确；每个文本表的语言与 `index.toml` 的完全一致；`constitution/` 下恰好有一个版目录，即放着 `index.toml` 的目录；版目录的 `articles/` 下全是条的目录，名字符合条的格式，各有一个 `article.toml`，条目录里其余的都是名字符合款的格式的 `.toml` 文件；`articles/` 旁边没有 `sections/`；`id` 等于去掉编号的名字；`status`、项的 `number` 和 `repealed` 格式正确，彼此一致，也与 `index.toml` 的状态一致；`edition` 是一个真实的月份 `YYYY-MM`。隐藏的名字 (以 `.` 开头，比如 `.DS_Store` 或编辑器的交换文件) 跳过。
2. ID：每个 ID 格式正确；条和款的 ID 在整部宪法里唯一，项的 ID 在所在的款里唯一。
3. 结构：编号在各自的范围内唯一；废止的条只含废止的款。
4. 语言：在每个文本表里，每种语言引用的 ID 集合都与默认语言的相同。
5. 引用：每个 `{@...}` 和每个 `authority` 条目都能解析；目标已废止 (废止的款里的项也算废止) 是错误；宪法只引用自身，不带命名空间；规定用 `constitution:` 或 `rule:` 引用。没有被取代、也没有废止的规定，`authority` 同时写了一个条款和它上面的条款 (项和它所在的款或条，款和它所在的条) 不算错误：各项检查通过后，无论带不带 `--check`，构建都会对每一对这样的条款打印一条警告，`path:line: warning: rule <id>: authority names both <above> and <below>, which is under it`，退出码不受影响。
6. 重新构建的结果与提交入库的 `constitution.<lang>.md` 和 `constitution.<lang>.agent.md` 逐字节相同 (只在 `--check` 下)。
7. 对照基准提交核对编号 (只在 `--check --base <git-ref>` 下，用 `git show` 读取)：基准的 `index.toml` 是 `active` 时，状态没有退回 `draft`，当前的 `edition` 与基准的相同或是更晚的月份；与基准相同时，基准中的每一条、每一款和每一项，按 ID 对应，都仍然存在且编号不变，款仍在同一条的目录里，项仍在同一款里；已废止的仍然废止；基准 parent 树上的每个节点都仍然存在。每个错误都说明属于哪一种：版不晚于基准、消失、改号、移到别的条、不再废止。基准的文件在基准自己的版目录里找，它的名字可能与当前的不同。基准是 `draft`、还没有版目录或条目录，或者 `edition` 早于当前的，就跳过比较，并打印一条说明。

   只要基准有 `index.toml`，不论它是什么状态，检查 7 的修订号部分都会运行，除非基准是 `active` 且它的 `edition` 与当前的不同；基准是 `draft` 时，不论两边各写的是哪一版都比较，这样给草案准备的那一版改名，修订号不会从头算起，批准后的文本的修订号也大于草案的。`articles/` 下有文件增加、删除或改动 (按它在各自版目录里的路径比较)，或者 `index.toml` 除 `revision` 以外的任何一行不同，就算宪法正文改了；生成文件和 `rules/` 不算。正文改了，`revision` 必须大于基准的；没改，必须等于基准的。「大于」是严格大于，不是「加一」，这样一次跳号的变更不算错误。草案的 `0.<n>` 按数字 `n` 比较 (`0.9` < `0.10`)，并且小于所有整数修订号；其他形式的修订号是错误。基准是另一版的 `active` 时，不比较修订号。这部分的目的是让一个修订号只对应一份文本。

规定是[规定文件格式](rule-format.md#rule-file-format) 所定格式的 parent 树节点，各在一个 `rules/<id>.toml` 里。上面的检查只读其中由宪法决定的部分。检查 1：节点的状态与 `index.toml` 的关系同规定一样；检查 5：每个 `{@constitution:...}` 引用和 `authority` 的每一项 (不带命名空间) 都解析到一个没有废止的条文，`authority` 同时写了一个条文和它上级的，给出警告；检查 7：基准里的节点仍然存在，已经废止或被取代的仍然如此；基准早于 parent 树、规定以单词命名的，由 `slug` 等于那个单词的节点承接。`params.toml` 不是节点，构建跳过它。

规定文件的其余部分由 `statute validate rules` 检查 ([命令与第一版](statute-command.md#the-command-and-the-first-version))，它是 `rules/` 或 crate 改动时运行的提交钩子，所以 CI 里也运行。它把这个目录当作第一方规定源读，检查单看一个规定源就能查的：每个节点的键按它的 `kind` 来，类型和取值正确，其中包括 `default` 和 `uses` ([规定配置](rule-config.md))；`id` 就是文件名，文件名不补零；slug 互不相同；每个 `parent`、`requires`、`conflicts`、`overrides` 和 `superseded-by` 都指向一个存在、种类也对的节点，parent 构成的树符合[规定不挂在规定下面](rule-config.md#no-rule-under-a-rule) 和[子领域](rule-sources.md#subdomains) 所说的形状，`requires` 不成环、没有多余的项；文本的每种语言引用的与原文相同，每种译文语言都有每段文本，`[translations]` 里的哈希等于原文现在的哈希，不等时错误信息会打印新的哈希；每个 `{@rule:<id>}` 都指向本规定源里一条存在且没有废止的规定；规定源的默认值能单独解析 ([默认开关](rule-config.md#default-switches))；以及[依赖、one-of 与规定包](rule-sets.md#dependencies-one-ofs-and-packs) 所说的 warning。每条发现单独一行写到标准错误；有错误时以状态 1 退出。它也读 `params.toml`，检查其中每个参数，并检查规定的每个占位符都对应其中一个参数 ([参数](rule-config.md#parameters))。`crates/statute/tests/real_rules.rs` 也读本仓的 `rules/`，有任何 error 或 warning 都失败。

[规定源](rule-sources.md) 的检查也这样分。`statute validate` 检查单看一个规定源就能查的：规定有 `parent` 或 `overrides`，必须有一个，不能两个都有；`overrides` 是另一个规定源的一条规定，写作 `"team:7"`，不能是列表，也不能是数字，因为数字指的是同一规定源里的节点；指向另一个规定源的 `parent` 只有规定能有，写作 `"team:3"`，只检查写法，因为第一方规定源的领域是根 ([子领域](rule-sources.md#subdomains))；别的规定源的领域，statute 接受它指向另一个规定源的 `parent`。本仓就是第一方规定源，所以在这里 `overrides` 或 `parent` 里的 `"R200"` 指的是同一规定源的规定，会失败；`parent` 写成这样时提示改写成编号。要看别的规定源才能查的，在解析规定清单时查：引用方向、覆盖的级别、公开仓库的私有层所做的覆盖，以及另一个规定源里的 `parent` 是否存在。这些都已实现。

每项检查都有一个让它失败的测试和一个通过的对照；编号检查在一个临时的 git 仓库上测试 (`tools/compiler` 的 `tests/test_constitution_checks.py`、`tests/test_constitution_base.py`，以及针对它们读到的规定部分的 `tests/test_rule_tree.py`；`statute validate` 的测试在 `crates/statute/tests/validate.rs` 和这个 crate 的其他测试里)。检查 1 至 6 和这些测试作为提交钩子运行，也在 CI 里运行；检查 7 只在 CI 里运行：在 pull request 上，对照检出的合并提交所基于的那个基准分支的末端 (即它的第一个父提交)；在推送到 `main` 时，对照推送前的那个提交。

修订号部分的可靠程度取决于这个基准。必需检查 `check` 不是严格模式：pull request 落后于 `main` 也能合并，所以它的 CI 运行可能没见过在它上次运行之后合并的修正案，两个修正案可能都用同一个修订号通过检查。pull request 上的运行排除不了这种情况；发现它的是 `main` 上的推送运行，它对每个合并提交做同样的检查，对照的是它之前的那个提交。如果它因为修订号重复而变红，合并方提一个更正的修正案，给正文一个自己的修订号。历史里已经重复的修订号就留着；每次推送只按它自己的提交判断。

指令文件单独变更的检查 (`check-instruction-changes`) 把 agent 当作指令读的改动与其他所有改动分开，使两者都能单独审查、单独撤销。哪些路径是指令文件、哪些是生成文件、哪些是其他文件，写在脚本顶部的一张表里：一部分是在任何仓库都成立的通用部分，一部分是本仓自己的 (宪法、规定和 skill 的源文件)，还有本仓的生成文件。通用部分包括 agent harness 在任何目录下读的文件，以及仓库顶层 statute 用来编译指令文件的文件：`statute.toml`、`statute.lock`、`statute-checks.toml` 和 `.statute/`，还有任何目录下的旧版副本 `<name>.legacy.md` (见 [statute-migrate](../../../skills/statute-migrate/SKILL.md) 的 Files 表)。生成文件跟着改动它的那一边走：与宪法源文件一起改，就是修正案；只与工具一起改，就是工具的改动。脚本比较 `HEAD` 与 `--base` (`git diff <base>...HEAD`，关闭改名检测)，两类混在一起时把两组文件都列出来。CI 用与检查 7 相同的基准运行它；一次推送到 `main` 同时带了几项变更的，作为一项来检查。

别的仓库在自己的 CI 里运行同一项检查：它按固定的 tag 和提交 SHA 取回本仓，就像 overlay 固定核心的版本那样 (见「发布与版本」)，然后在自己的检出目录里运行 `PYTHONPATH=<core>/tools/checks/src python3 -m repo_checks.check_instruction_changes --root . --base <base>`：这个脚本只需要标准库，所以那个仓库既不需要 uv，也不需要本仓的 workspace。通用部分始终适用。那个仓库在自己的 `AGENTS.md` 里用写给 agent 看的文字声明自己的指令路径，再把同样的路径以 `--instructions <regex>` 传给脚本，一个路径一个，把它的生成文件以 `--generated <regex>` 传入；两个选项都会替换掉本仓自己的那部分和本仓的生成文件。

设计译文检查 (`check-design-translation`) 让设计的中文译文 `docs/design/zh/<name>.md` 与英文 `docs/design/<name>.md` 保持同步，英文是源文件；网站两种语言都展示 (见[静态网站](static-site.md))。译文第一行是 `<!-- en-sha256: <hex> -->`，即翻译所依据的英文文件的 SHA-256，第二行是 `# ` 标题。标题以下的每个标题都以英文对应位置那个标题的锚点开头，如 `## <a id="checks"></a>检查`：锚点就是 GitHub 和 Hugo 给英文标题生成的那个 (转小写、去掉标点、空格换成连字符，重复的加 `-1`)，所以指向 `#checks` 的链接在两种语言里、在 GitHub 上和网站上都能用。设计文件之间的链接照英文原样写，指出 `docs/design/` 的链接多加一层 `../`，这样每个链接都从它所在的文件解析。以下情况检查会失败：任何一边缺文件；哈希不是英文文件当前的哈希；任何一个标题的级别或锚点与英文对应的不同，或者只有一边有。围栏代码块里的行不算标题。英文文件或译文里的相对链接，从它所在文件的目录解析后指不到仓里的文件或目录 (比如译文里的链接少了一层 `../`)，检查也会失败，否则网站会把它改成指向 GitHub 上一个不存在的文件；代码里的链接跳过。所以英文文件的任何改动，哪怕只改一个错字，都会让检查失败，直到同一个 pull request 把改动带进译文、更新哈希那一行；检查会打印新的哈希。`docs/design/zh/` 不存在时，检查打印一行说明，只检查英文的链接；目录一旦存在，每个文件都要有译文。

中文排版用 limae 及其默认配置检查。`lint-zh-sources` 把版目录下 `index.toml` 和 `articles/` 里、以及 `rules/` 里的每段 `zh` 文本，各作为一个单独的 Markdown 文件交给 limae，把每条发现打印成 `path:line: field: <message>`，其中 field 是 TOML 表名，项带上它的 ID (`items.capital.content`)。在此之前，它把每处引用 `{@...}` 替换成形状像宪法引用渲染结果的中文 (`第某条`)，检查完再换回来：`agent{@x}` 渲染成 `agent第十条`，需要一个空格，而 limae 在花括号旁边看不出来；`依照{@x}批准` 则不需要 (limae 0.13.3，2026-10 读取)。对规定的引用也用同样的占位，它只在阶段 B 确定规定引用的渲染方式之前代替渲染结果 (那时会是规定的标题，可能以拉丁字母开头或结尾)。带 `--fix` 时，它把 limae 的修正原地写回，不改动其他任何字节，并重新解析文件来确认这一点。提交钩子和 CI 只检查；修正由人手动运行。limae 钩子对仓里每个 Markdown 文件运行，主题 `web/themes/` 除外，英文文件也在内，limae 在那里多半查不出什么。对版目录 `generated/` 下的中文文件 `constitution.zh.md` 和 `constitution.zh.agent.md`，这个钩子还查只有构建之后才有的内容：款和项的编号、标题、渲染出的引用周围的文字。那里四个文件中任何一个的发现，都在源文件或构建脚本里修，从不在生成文件里修。规定还没有构建出的文本，所以它们的中文由源文件检查覆盖。不是生成的 Markdown 文件里的发现就在文件里修。误报就在那一行上方加 `<!-- limae-disable-next-line <规则> -->` 注释放过，不为此在整个仓里关掉规则。

私有词扫描只打印位置和类别，从不打印匹配到的文本。在 CI 里它只运行通用类别；词表在私有一侧运行时注入 (`--terms-file`)，从不提交。每个类别先对一个必须匹配的对照字符串运行一次，这样写坏的模式不会悄无声息地通过。

未声明的放宽不可能全部查出来。放宽检查程序还会以警告的形式标出这样的下位法：它在规定或条文的 ID 旁边写了解除义务的措辞 (「not required」「instead of」「may skip」)，却没有 `relaxes:` 条目。其余的靠审查和行为 fixture。这个检查程序在每个仓库自己的 CI 里运行，对照固定的核心版本，检查那个仓库的 `AGENTS.md`。编译器对 overlay 的私有规定运行同样的警告：一条私有规定如果解除了某条在用的通用规定的义务，应当改为点名去掉那条规定。

检查输出里包含某句话，证明的是这句话在那里，不证明 agent 会照做；关于这一点，只有行为 fixture 和真实的运行能说明些什么，而且它们也只是抽样。

## <a id="fixtures"></a>Fixture

```text
fixtures/static/happy/<name>/
fixtures/static/redline/<name>/
fixtures/behavior/happy/<name>/     stage C
fixtures/behavior/redline/<name>/   stage C
```

每个 fixture 目录放一个 `fixture.md` (front matter 写 `binds: [constitution:asking, rule:3]`、`kind: happy` 或 `redline`，以及一句简短的说明，讲它展示了什么)，还有它的输入和预期结果。

- 静态 fixture 是确定性的：输入一份 overlay 或一份下位法，对编译器或某个检查程序做一个断言 (能构建；以这个错误失败；这条声明被拒绝)。它们在每次 CI 运行中都跑。
- 行为 fixture (以后再做) 给真实的 agent 一个场景，评判它做了什么。它们记录 harness、模型和设置、运行次数以及每次失败的情况；通过一次，也不当作保证来报告。
- happy fixture 和红线 fixture 分开放，这样一个安全情形的修复如果破坏了日常工作，就会表现为一个失败的 happy fixture。
- 每个 fixture 至少绑定一个宪法或规定的 ID，写法与规定中的引用一样带命名空间；ID 解析不了的 fixture，检查程序会拒绝。
- 发布之前，所有 fixture 都要跑，happy 和红线全部，这是第 26 号规定 (`fixture-runs`) 的要求。静态 fixture 在本仓的 CI 里跑。行为 fixture (有了以后) 需要安装好的 harness 和模型凭证，本仓的 CI 没有这些；它们在私有一侧跑，发布需要有在打 tag 的那个提交上跑过的记录。
- fixture 和其他文件一样遵守宪法：只用虚构的数据 (`example.test` 域名、文档用地址段、虚构的名字)，没有真实凭证，没有私有词。私有词扫描覆盖 `fixtures/`。

待人决定：红线 fixture 放在哪里。推荐：与它测试的文本放在一起，绑定到那段文本的 ID。

- 宪法：它的红线 fixture 放在核心里。针对任何项目都不能改的内容的 (例如修正案要经过人，`must-ask.amendments`)，对每个法域都运行。针对默认边界事项的，测试的是默认安排；依照 `relaxations.project-overrides` 另行规定了这一项的项目，运行它自己为这项另行规定写的 fixture，代替核心的那个，与被去掉的规定的处理方式相同。
- 通用规定：每条规定的红线 fixture 放在核心里，绑定到这条规定的 ID。核心的 CI 运行全部这些 fixture。法域运行它所用规定的 fixture；它去掉的规定的 fixture 在那里不运行，它的 manifest 把它们列为未运行，从不列为已通过。
- 私有规定和仓库的放宽：它们的 fixture 放在各自那一边 (overlay、仓库)，在那一边的 CI 里对照固定的核心版本运行，就像放宽检查程序已经在那里运行的那样。

理由：fixture 与它测试的文本一起变，所以应当放在那段文本所在的地方；把它绑定到 ID，意味着去掉一条规定时，它的 fixture 也随之去掉，不必另外维护一份清单。具体的硬性规定 (例如关于凭证的) 现在是可选的规定，不在宪法里，所以它们的 fixture 不能放在宪法一级、到处运行。每个项目都复制一份核心的 fixture，会逐渐走样，而且测试的是项目并不拥有的文本。

## <a id="load-verification"></a>加载验证

编译出文件，不说明 agent 读到了它。overlay 的 apply 步骤用每个 harness 自己的只读入口检查它 (截至 2026-10-12：Codex 的 `debug prompt-input`、Grok 的 `inspect`，以及 Claude Code 的 `InstructionsLoaded` hook)：

- 正臂：纯文本文件头里的输入摘要，出现在每个 harness 实际加载的内容里。(Claude Code 在注入文件之前会去掉块级 HTML 注释，所以摘要不能放在注释里。)
- 对照臂：一个不应出现的字符串确实没有出现。核心自带一条固定的 `draft` 金丝雀规定，它的正文是一个独一无二的字符串；它在每次构建的源文件里，从不出现在任何输出里。
- 一个入口只有返回加载的内容、而不只是文件路径，才算数；哪些入口满足这一点，在阶段 B 实现这项检查时逐个 harness 确认。

如果正臂通过，对照臂也找到了它的字符串，这项检查就分不清正确的加载和加载了错误的文件，判为失败。核心提供这项检查；它在私有一侧运行，因为需要安装好的 harness。

## <a id="feedback-loop-reserved-places"></a>反馈回路：预留的位置

现在不实现。先定下形状，让以后的工作有地方可放：

```text
event -> proposal -> rule revision, case, or amendment -> release
```

- 事件来自工作本来就会留下记录的地方：worker 报告、审查发现、失败的检查、人的更正和答复。核心不存储事件；事件源只需要交来一条简短的记录 (发生了什么、当时生效的是哪次构建和哪些规定 ID、证据的出处、初步分类)。「不需要改规定」也是一个有效的结论。
- 案例 (`cases/<id>.md`，阶段 C)：事实、当时生效的构建、适用的规定 ID、决定及其状态。案例解释一条规定，从不授予许可。一个案例如果会改变权利、义务或禁止，就变成对规定的修订；如果会对所有项目放宽一个边界事项，就变成一份修正案提案，或者，对规定库所加的事项，变成对那条规定的修改，并由人在这次修改上批准 (`list-defaults`)；只对一个项目放宽，用一条 `relaxations.project-overrides` 声明即可。
- 中间地带 (`middle-ground`)：agent 拿不准、去问人时，答复作为候选案例写在私有一侧，附上来源。同类答复反复出现时，就变成一份规定提案。规定库可以做哪些边界变更、哪些需要修正案，见 `list-defaults`。
- 规定提案是改动 `rules/` 的 pull request，依照 `rule-evolution` 采纳，每份都要回答：什么问题、什么证据、为什么是规定而不是修工具、涉及哪些 ID、依据什么、可能弄坏什么、怎样测试和回滚。明显的效率提升依照 `ordinary-rule-adoption` 采纳；小的改进集中处理 (`prefer-tooling`)。
- 修正案提案是改动 `constitution/` 的 pull request；只有依照 `amendment-procedure` 才生效。
- 找出下一项工作并验证它 (阶段 D)：接口是两种记录格式，与任何 issue 跟踪系统都无关。工作项：做什么、为什么、验收标准、停止条件。验收记录：运行的每项检查、在哪个提交上、显示结果的那行输出，以及没运行的每项检查和原因。找工作的一方和验证的一方，不论是什么，都交换这两种记录；阶段 D 开始时，核心在 `schema/` 里定义它们。
