<!-- en-sha256: 3cc5183cbc8194d4dc8208d06439290f95d8381afdd0879eff08c833057e5064 -->
# Agent Fleet 与迁移

## <a id="relation-to-agent-fleet"></a>与 Agent Fleet 的关系

工具链只是一组处理文件的普通命令：读入源文件和 overlay，写出输出文件和 manifest，再以一个状态码退出。它不调用 Agent Fleet，不读 Agent Fleet 的状态，也不依赖 Agent Fleet 怎样启动 agent。Agent Fleet (以及其他任何编排工具) 像任何 harness 一样使用这份输出。与 Agent Fleet 相关的文件只有 `.fleet/config.json`，它告诉 Agent Fleet 本仓的 issue 在哪里。

## <a id="migrating-from-the-current-hand-written-guide"></a>从现有的手写的指南迁移

这里只写方法和顺序；每条规定怎样对应，是另一项单独的工作。

1. 切换之前，手写的指南一直有效，已经换成自己规定清单的仓库除外。什么都不提前删。
2. 先批准宪法，因为每条规定的 `authority` 都要指向宪法里的某一条文。
3. 把现有的每条规定归入一个去处：宪法、通用规定、私有规定、参数 (取值移到 `params.toml`，规定里留一个占位符)、流程 (移进 skill)、来历 (移到私有的案例集)，或者删除。
4. 先实现 `statute` 和它的检查程序 ([命令与第一版](statute-command.md#the-command-and-the-first-version))，再分批移植规定，每批之后编译一次。
5. 每个仓库在加入规定清单的那次修改里，把编译出的输出和它所替换的指令对比，记下每一处有意为之的差异 ([规定源](rule-sources.md#cutover-from-the-hand-written-guide)，第 3 个条件)。
6. 现有的每一处仓库级覆盖，要么改成一条 `relaxes:` 声明，要么删掉，由人决定。
7. 切换：仓库一个一个切换，还在用的仓库都切换之后，停用手写的指南，不装编译出的文件代替它。

[规定源](rule-sources.md#cutover-from-the-hand-written-guide) 把第 3 步的去处「私有规定」按范围和可见性再分开，把第 6 步改为覆盖和 one-of，并写明第 7 步里仓库怎样切换、什么时候可以停用指南、停用之后怎样检查。
