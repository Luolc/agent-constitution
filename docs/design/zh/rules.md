<!-- en-sha256: ca874e75f5445904fd73e5391356ade34608bba5c849c877e71d06d33d5dbb42 -->
# 规定与放宽

## <a id="rules"></a>规定

一条规定一个文件，`rules/<id>.toml`，这样一份提案就是一个文件，diff 里也正好只有一条规定在变。

```toml
id = "credential-storage"
status = "draft"
scope = "all"
authority = ["constitution:safety", "constitution:sensitive-info"]

[title]
en = "<Title>"
zh = "<标题>"

[content]
en = '''
<statement: condition, action, exception, what to do when unsure>
'''
zh = '''
<正文>
'''
```

- `[title]` 和 `[content]` 为 `index.toml` 中的每种语言各设一个键。
- 正文用 `{@constitution:<id>}` 引用宪法，用 `{@rule:<id>}` 引用另一条规定 (见 [ID](source-formats.md#ids))；在编译出的指令文件里，对规定的引用显示为那条规定的标题，其余的显示方式在阶段 B 确定。规定没有编号。
- 关于消息或通知的规定，要写明它管哪一种：agent 对 agent、agent 对人，或者两者都管。
- `id`：固定不变，同时是文件名。改标题不会改它。
- `status`：`draft`、`active` 或 `repealed`。只有 `active` 的规定参与编译。废止的规定保留文件和 ID。宪法还是草案时，所有规定也都是 `draft`。
- 与宪法不同，规定可以删除。删除后它的 ID 单独占一行写进 `rules/retired-ids.txt`，永不再用；构建会拒绝 ID 列在其中的规定。这个文件不存在时，视为空列表。
- `scope`：规定针对谁：`all`，或者 `schema/` 中定义的一个短列表里的某一项 (例如 `implementer`、`reviewer`、`merger`)。编译器把它显示为一行前缀，但不按它筛选。所有角色共用一份编译出的文件。
- `authority`：规定所依据的宪法条、款或项，写法与正文中的引用相同 (`constitution:<id>`)；其中不得有已废止的。没有 authority 的规定不能编译。
- `adjustable` (可选)：这条规定里允许下位文本依照 `relaxations.adjustable-rules` 修改的值，每个值附带默认值，例如 `adjustable = ["wait_hours=24"]`。没有这个字段的规定不能这样放宽。可调值与本地参数 (见下文) 不是一回事：本地参数是私有 overlay 填入的值，可调值是仓库或任务可以修改的值。

其他字段 (`supersedes`、`verification`、`expires`) 等到真有规定需要时再加，不提前加。

overlay 里的私有规定格式相同，与通用规定共用一个命名空间。编译器检查核心与 overlay 的规定 ID 合在一起不重复，并且都没有在任何一边被停用。

## <a id="choosing-rules"></a>选用规定

通用规定是一套默认清单，并不是对每个法域都有约束力的义务 (`division-of-instructions.rules`)：某个法域需要的规定，另一个法域可能有充分的理由不用 (一个始终私有、放在自托管服务器上的仓库，可能有意把凭证存在仓库里)。这一点待人决定。推荐的做法：

- 法域默认使用全部通用规定。它的 overlay 要去掉某条规定，只能在 `overlay.toml` 里点名，并附一行理由 (`exclude = ["credential-storage"]`)。构建 manifest 列出去掉了哪些。
- 法域内的仓库要偏离某条规定，只能在宪法允许的地方写放宽声明 (见下文)。仓库不从编译出的文件里去掉规定，因为一份编译出的文件供一个法域的所有仓库共用。
- 要修改一条规定而不是弃用它，overlay 先去掉它，再加一条私有规定替代。
- 宪法不能这样去掉，也不能这样放宽。

理由：默认全部使用，新加的规定就会自动到达每个法域，不必有人记得把它列进去；而按需加入的清单会随着规定增多而过时。去掉一条规定不算放宽 (`division-of-instructions.rules`)，但逐条点名并写明理由，能让这件事一直看得见，也把一个法域弃用的所有规定集中在一处。按法域而不是按仓库去掉规定，与输出的构建方式一致：一个法域一份文件。

## <a id="relaxation-declarations"></a>放宽声明

下位文本 (仓库的 `AGENTS.md`，或者任务文件) 要放宽一条规定，只能写明宪法里允许这样做的那一项：

```markdown
## Relaxations

- relaxes: constitution:relaxations.adjustable-rules
  target: rule:example-rule/wait_hours=4
  scope: <this repo, or this task>
  condition: <how this text meets the condition that item states>
```

一项放宽写一个条目，四个键都要有：依据的许可 (`relaxations` 中的一项，写法与规定中的引用相同)、放宽的对象 (一条规定、一个边界事项如 `must-ask.means-of-production`，或者依照 `relaxations.adjustable-rules` 时，某条规定的一个可调值及其新的取值)、适用范围，以及怎样满足那一项所写的条件。没有这样声明的放宽不生效，不论文件其他地方怎么写。私有 overlay 不声明放宽，而是去掉规定 (见「选用规定」)。

## <a id="local-parameters"></a>本地参数

需要本地取值的规定，不写取值，而写参数名：`{{ models.reviewer.default }}`。overlay 的 `params.toml` 提供这些值。参数只是普通的值，从来不是规定的文字，这样私有一侧改一个模型名或一份机器列表，不必动任何规定。未解析或者未使用的参数会让构建失败。
