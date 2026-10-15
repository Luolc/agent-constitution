<!-- en-sha256: e29451eae27d6eeecb162d792639e5d89f344b6f022f4a519a897203ba00968e -->
# 规定与放宽

## <a id="rules"></a>规定

parent 树上的一个节点一个文件，`rules/<id>.toml`，这样一份提案就是一个文件，diff 里也正好只有一条规定在变。格式、ID 和状态见[规定文件格式](rule-format.md#rule-file-format) 与 [ID 与修订号](rule-format.md#ids-and-revisions)。

- 正文用 `{@constitution:<id>}` 引用宪法，用 `{@rule:<编号>}` 引用另一条规定 (见 [ID](source-formats.md#ids))；在编译出的指令文件里，对规定的引用显示为那条规定的标题，其余的显示方式在阶段 B 确定。
- 关于消息或通知的规定，要写明它管哪一种：agent 对 agent、agent 对人，或者两者都管。
- 宪法还是草案时，所有规定也都是 `draft`；宪法现行之后，新规定从 `trial` 开始 ([试行规定](rule-checks.md#trial-rules))。
- `adjustable` (可选)：这条规定里允许下位法依照 `relaxations.adjustable-rules` 修改的值，每个值附带默认值，例如 `adjustable = ["wait_hours=24"]`。没有这个字段的规定不能这样放宽。可调值与本地参数 (见下文) 不是一回事：本地参数是私有 overlay 填入的值，可调值是仓库或任务可以修改的值。

overlay 里的私有规定格式相同，放在它自己的规定源里 ([规定源与引用](rule-sets.md#sources-and-references))。

## <a id="choosing-rules"></a>选用规定

通用规定是一套默认清单，并不是对每个法域都有约束力的义务 (`division-of-instructions.rules`)：某个法域需要的规定，另一个法域可能有充分的理由不用 (一个始终私有、放在自托管服务器上的仓库，可能有意把凭证存在仓库里)。这一点待人决定；[规定配置](rule-config.md#default-switches) 里的默认开关实现之后，取代下面的推荐。推荐的做法：

- 法域默认使用全部通用规定。它的 overlay 要去掉某条规定，只能在 `overlay.toml` 里点名，并附一行理由 (`exclude = [3]`)。构建 manifest 列出去掉了哪些。
- 法域内的仓库要偏离某条规定，只能写放宽声明 (见下文)，而且限于宪法允许的情形。仓库不从编译出的文件里去掉规定，因为一份编译出的文件供一个法域的所有仓库共用。
- 要修改一条规定而不是弃用它，就由 overlay 的一条规定覆盖它 (见[覆盖](rule-sources.md#overrides))。在公开仓库里，私有层只能增加规定，不能覆盖；那里的修改写成一条公开的本地规定，或者向这条规定的规定源提议。
- 宪法不能这样去掉，也不能这样放宽。

理由：默认全部使用，新加的规定就会自动适用于每个法域，不必有人记得把它列进去；而按需加入的清单会随着规定增多而过时。去掉一条规定不算放宽 (`division-of-instructions.rules`)，但逐条点名并写明理由，能让这件事一直看得见，也把一个法域弃用的所有规定集中在一处。按法域而不是按仓库去掉规定，与输出的构建方式一致：一个法域一份文件。

## <a id="relaxation-declarations"></a>放宽声明

下位法 (仓库的 `AGENTS.md`，或者任务文件) 要放宽一条规定，只能写明宪法里允许这样做的那一项：

```markdown
## Relaxations

- relaxes: constitution:relaxations.adjustable-rules
  target: rule:42/wait_hours=4
  scope: <this repo, or this task>
  condition: <how this text meets the condition that item states>
```

一项放宽写一个条目，四个键都要有：依据的许可 (`relaxations` 中的一项，写法与规定中的引用相同)、放宽的对象 (一条规定、一个边界事项如 `must-ask.means-of-production`，或者依照 `relaxations.adjustable-rules` 时，某条规定的一个可调值及其新的取值)、适用范围，以及怎样满足那一项所写的条件。没有这样声明的放宽不生效，不论文件其他地方怎么写。私有 overlay 不声明放宽，而是去掉规定 (见「选用规定」)。

## <a id="local-parameters"></a>本地参数

需要本地取值的规定，不写取值，而写参数名：`{{ models.reviewer.default }}`。overlay 的 `params.toml` 提供这些值。参数只是普通的值，从来不是规定的文字，这样私有一侧改一个模型名或一份机器列表，不必动任何规定。未解析或者未使用的参数会让构建失败。[参数](rule-config.md#parameters) 取代这一节：规定源在自己的 `params.toml` 里声明参数，取值由规定清单设定，`personal` 参数则由这个人设定。
