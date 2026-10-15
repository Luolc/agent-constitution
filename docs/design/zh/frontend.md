<!-- en-sha256: 1661ef2e620a8ddbe5d931fa064e4ed390539b8c06482acc706a91a453ddfce7 -->
# 前端

本文写本仓页面的样子和行为，以及改动页面时怎么做、怎么审：设计 token、图标、动效、组件及其状态，以及 pull request 要附的截图。它适用于展示宪法和规定的两个网站：[公开网站](static-site.md) 和 `statute site` 生成的[本地网站](statute-command.md#the-local-site)。

尚未实现：本文有些部分先于代码写成。现有页面在[现状](#status) 一节列出的地方符合它，其余部分会在各页重做时取代现在的做法。token、图标和脚本在代码里怎么写，见[代码实现](#in-the-code)。

## <a id="contents"></a>目录

- [设计 token](#design-tokens)：什么是 token，以及关于取值的那条规矩。
- [颜色](#color)：两套主题和各种状态。
- [字体](#type)：字体、字号和字重，以及规定列表的各种行。
- [间距、圆角和阴影](#space-radius-and-shadow)。
- [图标](#icons)：只用一套图标库 Lucide，以及现有图标换成什么。
- [动效](#motion)：时长、缓动和减弱动效。
- [交互与无障碍](#interaction-and-accessibility)：点击区域的大小、只在能悬停的设备上用 hover、给辅助技术的名称和状态、不靠脚本也能用的页面。
- [组件](#components)：标记和 tag、图例、筛选器、规定表、one-of、弹出层、侧边栏和按钮。
- [不做的事](#not-done)：页面上永远不做的事。
- [截图与审查](#screenshots-and-review)：改动页面的 pull request 附什么、CI 跑什么、审查方核什么。
- [代码实现](#in-the-code)：token、样式表和图标放在哪里，脚本怎么构建。
- [现状](#status)：哪些已经实现，以及这里的设计取代什么。
- [待定问题](#open-questions)。
- [出处](#sources)：本文参考的外部规范。

## <a id="design-tokens"></a>设计 token

token 是一个有名字的值：一种颜色、一个尺寸、一段时长。下面每个 token 都写成名字、light 主题下的值和 dark 主题下的值，两者相同时只写一个。名字不带前缀 (`text`、`space-4`)；在样式表里，每个 token 是一个带 `--t-` 前缀的自定义属性 (`--t-text`，见[代码实现](#in-the-code))。

关于取值只有一条规矩：在定义 token 的地方之外，页面样式里的颜色、字号、字重、间距 (margin、padding、gap)、圆角、阴影、时长和缓动一律写 token 名，不直接写值。可以直接写的值只有这几种：`0`、`auto`、`100%`、`1px` 和 `2px` 的边框、媒体查询里的断点 (那里不能用自定义属性)，为把某一个字形摆进行内而设的偏移 (条目编号后的星号，要加注释说明)，`clamp()` 里让间距在两个 token 之间随窗口变化的 `vw` 项，以及打印样式的值 (打印没有主题：白纸、以磅为单位的字号)。页面需要一个 token 里没有的值，先在本文里把它加成 token。

另有两条在两套主题下的每个页面都成立：任何状态都不只靠颜色表示 (当前页还要加粗或带色条，错误还要有图标或文字)；下面的每种状态在两套主题下都要画出来。

## <a id="color"></a>颜色

配色沿用现在的 sage 配色，状态和 tag 的 token 是新加的。对比度指 WCAG 的对比度，按下表的值计算。

| Token | Light | Dark | 用途 |
| --- | --- | --- | --- |
| `bg-page` | `#dde6d5` | `#1e2520` | 正文所在的页面底色。 |
| `bg-panel` | `#d1dcc8` | `#252d27` | 侧边栏、弹出层、筛选器的面板。 |
| `bg-hover` | `#c7d0bf` | `#343b35` | 页面上的一行或一个按钮在指针下时。 |
| `bg-panel-hover` | `#bcc7b4` | `#3a423b` | 面板上的一项在指针下时。 |
| `bg-active` | `#b2bdaa` | `#444c45` | 一项被按下时。 |
| `bg-selected` | `#c5c5b0` | `#3e3e33` | 当前项：侧边栏里的当前分组、选中的选项。 |
| `bg-target` | `#e3d79c` | `#4a4a2a` | 锚点跳到的元素，短暂显示。 |
| `text` | `#23301f` | `#d3dccf` | 正文和正文里的链接。 |
| `text-muted` | `#4b5946` | `#9cab98` | 标签、次要文字、侧边栏里的页面、目录。 |
| `text-faint` | `#5a6754` | `#839080` | 页面上更次一级的说明：标题下的图例。不用在面板上。 |
| `accent` | `#7d3a1e` | `#dba37f` | 指针下的链接、当前项的文字、焦点框、主按钮。 |
| `accent-hover` | `#6b381e` | `#d9ae8f` | 主按钮在指针下时。 |
| `text-on-accent` | `#dde6d5` | `#1e2520` | `accent` 底色上的文字。 |
| `border-subtle` | `#b5c4aa` | `#38453a` | 细线：各部分之间的分隔线、链接的下划线、tag 的轮廓。 |
| `border-strong` | `#6d7968` | `#717970` | 控件和框：输入框、one-of 的框、规定列表表头下面的线。 |
| `status-active` | `#245a20` | `#9fd39a` | 规定列表里 Active 的图标；Active tag 的文字。 |
| `status-active-bg` | `#c3d2bc` | `#303d31` | 它的底色。 |
| `status-trial` | `#66460a` | `#e2c07a` | Trial 的图标；Trial tag 的文字。 |
| `status-trial-bg` | `#ccd0b9` | `#393b2d` | 它的底色。 |
| `status-draft` | `#4b5946` | `#9cab98` | Draft 的图标；Draft tag 的文字。 |
| `status-draft-bg` | `#c9d2c1` | `#303831` | 它的底色。 |
| `quadrant-global-public` | `#acc8df` | `#274160` | 规定源象限 tag 的底色 (文字用 `text`)。 |
| `quadrant-global-private` | `#e4d098` | `#5e491a` | |
| `quadrant-local-public` | `#a5dcb1` | `#1f5533` | |
| `quadrant-local-private` | `#cdbadf` | `#473360` | |

派生的颜色都是配色的混合，这些值就是这样选出来的，配色变了也能跟着调整：`bg-hover` 是 12% 的 `text` 混入 `bg-page`，`bg-panel-hover` 是 12% 的 `text` 混入 `bg-panel`，`bg-active` 是 18% 的 `text` 混入 `bg-panel`，`bg-selected` 是 14% 的 `accent` 混入 `bg-panel`，`accent-hover` 是 20% 的 `text` 混入 `accent`，`border-strong` 在 light 主题下是 60% 的 `text` 混入 `bg-page`，dark 主题下是 46% (满足下面对比度要求的最小比例)，每种状态的底色是 14% 的该状态文字色混入 `bg-page`，每个象限是 30% 的对应色相 (蓝 `#3b82f6`、琥珀 `#f59e0b`、绿 `#22c55e`、紫 `#a855f7`) 混入 `bg-page`。

两套主题下都要满足的对比度：

- `text`、`text-muted` 和 `accent` 在 `bg-page` 和 `bg-panel` 上，以及 `text-faint` 在 `bg-page` 上：不低于 4.5。最低的是面板上的 `text-muted` (light 5.25，dark 5.86) 和页面上的 `text-faint` (两套都是 4.67)。`text-faint` 在面板上只有 4.22，低于下限，所以面板上最淡的文字用 `text-muted`。
- `text-on-accent` 在 `accent` 和 `accent-hover` 上：不低于 4.5 (light 6.55 和 7.40，dark 7.13 和 7.76)。
- 状态 tag 的文字在它的底色上：不低于 4.5 (light 5.19、5.43、4.78；dark 6.67、6.56、5.01)。
- `text` 在象限 tag 上：不低于 4.5 (最低 6.10)。
- 列表里状态的图标在页面上和 `bg-hover` 上：不低于 4.5，高于图标要求的 3 (Active light 6.39 和 5.16，dark 9.14 和 6.72；Trial 6.69 和 5.40，8.99 和 6.61；Draft 5.81 和 4.69，6.48 和 4.77)。
- `text` 在分组行的底色上：不低于 4.5 (在 `bg-active` 上 light 7.10，dark 6.30；在 `bg-panel-hover` 上 7.90 和 7.37)。
- hover 底色相对它所在的表面：不低于 1.2，这样在 light 模式下也看得出变化 (`bg-hover` 在页面上 light 1.24、dark 1.36；`bg-panel-hover` 在面板上 1.24 和 1.36)。
- `accent` 在 `bg-selected` 上：不低于 4.5 (light 4.80，dark 4.92)。
- `border-strong` 作为控件的边 (未勾选的复选框、输入框、菜单)，在 `bg-page` 和 `bg-panel` 上：不低于 3，即 WCAG 对控件边界的下限 (light 3.57 和 3.22，dark 3.48 和 3.15)。
- 任何状态都不让其中文字的对比度低于 4.5。`text-muted` 只在 `bg-hover` 上够 (light 4.69，dark 4.77)；在 `bg-panel-hover`、`bg-active` 和 `bg-selected` 上降到 3.67 到 4.48 之间，所以在这些底色上，弱化的标号和 tag 的字改用 `text`。`accent` 在 `bg-active` 上低于 4.5 (light 4.31，dark 4.04)，所以按下的项文字用 `text`。

焦点框是 `2px` 的 `accent`，画在元素外 (偏移 `2px`)，只在键盘获得焦点时显示 (`:focus-visible`)，所有控件都一样。

## <a id="type"></a>字体

字体不变：英文用 EB Garamond，中文用 EB Garamond 加 Noto Serif SC (SIL Open Font License 1.1)，回退到本机的衬线字体，正文和控件都用它们。正文字号随窗口在下限和上限之间变化，栏宽以 em 为单位，一行的长度因此保持不变，取值同[静态网站](static-site.md)：

| Token | 英文 | 中文 |
| --- | --- | --- |
| `font` | EB Garamond，然后是本机的衬线字体 | EB Garamond、Noto Serif SC，然后是本机的衬线字体 |
| `font-size-base` | `clamp(1.125rem, 0.645vw + 0.686rem, 1.375rem)` | `clamp(1rem, 0.548vw + 0.583rem, 1.17rem)` |
| `measure` | `36em` | `40em` |
| `line-height-text` | `1.65` | `1.9` |
| `line-height-ui` | `1.4` | `1.5` |

正文字号不小于 16px：上面的下限英文是 18px，中文是 16px。英文一行约 90 个字符，多于多数指南给的 45 到 75 个；为什么这份文字这样更好读，见[静态网站](static-site.md)。需要比较的数字 (同一列里、上下行之间) 用等宽数字，在表格里靠末端对齐。

其余字号都相对正文字号，以 em 为单位：

| Token | 字号 | 用途 |
| --- | --- | --- |
| `text-xs` | `0.75em` | tag、按钮上的计数。 |
| `text-sm` | `0.875em` | 控件、侧边栏分组名、说明、目录。 |
| `text-md` | `1em` | 正文；规定列表里的每一行。 |
| `text-lg` | `1.25em` | 二级标题。 |
| `text-xl` | `1.5em` | 设计各页的一级标题。 |
| `text-2xl` | `1.75em` | 页面标题。 |

| Token | 英文 | 中文 |
| --- | --- | --- |
| `weight-regular` | `450` | `500` |
| `weight-medium` | `600` | `650` |
| `weight-bold` | `750` | `800` |

同样的字重下，Noto Serif SC 比 EB Garamond 显得细，所以中文另有一列。规定列表的各行字号相同，这样才排得成一张表；领域的行与规定的行靠字重和底色区分 ([组件](#components))：

| 行 | 字号 | 字重 | 颜色 |
| --- | --- | --- | --- |
| 领域的分组行，它的标号和路径 | `text-md` | `weight-bold` | `text`，第一层的底色是 `bg-active`，更深的是 `bg-panel-hover` |
| 框里 one-of 自己的那一行 | `text-md` | `weight-medium` | `text` |
| 规定的行 | `text-md` | `weight-regular` | `text` |
| 规定的标号 (`R13`) | 与所在行相同 | 与所在行相同 | `text-muted`，等宽数字，向末端对齐 |
| 表头 | `text-sm` | `weight-medium` | `text-muted` |

## <a id="space-radius-and-shadow"></a>间距、圆角和阴影

组件内部和组件之间的间距以 rem 为单位，按下面的阶梯取值；属于正文的间距 (段落之间、标题上下) 仍以 em 为单位，取正文字号的倍数。

| Token | 值 |
| --- | --- |
| `space-1` | `0.25rem` |
| `space-2` | `0.5rem` |
| `space-3` | `0.75rem` |
| `space-4` | `1rem` |
| `space-6` | `1.5rem` |
| `space-8` | `2rem` |
| `space-12` | `3rem` |

| Token | 值 | 用途 |
| --- | --- | --- |
| `width-popover` | `28rem` | 装着控件的弹出层的宽度，即筛选器的面板。 |
| `width-label` | `3.2em` | 规定列表里标号那一列。 |
| `width-mark` | `1.1em` | 那个列表里一行标记的四个位置，每个的宽度。 |
| `size-dot` | `0.6em` | 默认开启或关闭的圆点。 |

| Token | 值 | 用途 |
| --- | --- | --- |
| `radius-sm` | `0.25rem` | tag、输入框、按钮、一行的 hover 底色。 |
| `radius-md` | `0.5rem` | 弹出层、面板、one-of 的框。 |
| `radius-full` | `9999px` | 圆形的东西：计数徽标、标记里的圆点、状态胶囊。 |

| Token | Light | Dark | 用途 |
| --- | --- | --- | --- |
| `shadow-raised` | `0 1px 1px rgb(0 0 0 / 0.06), 0 1px 3px rgb(0 0 0 / 0.08)` | `0 1px 1px rgb(0 0 0 / 0.25), 0 1px 3px rgb(0 0 0 / 0.30)` | 立在页面上的控件。 |
| `shadow-popover` | `0 1px 2px rgb(0 0 0 / 0.08), 0 0.25rem 0.9rem rgb(0 0 0 / 0.14)` | `0 1px 2px rgb(0 0 0 / 0.30), 0 0.25rem 0.9rem rgb(0 0 0 / 0.45)` | 弹出层和筛选器的面板。 |

每种阴影都有两层：近的一层勾出边缘，远的一层把它托起来。放在页面上的框用边框或阴影，二者只用其一；弹出层两者都用，因为它压在颜色相同的文字上方。圆角框套在另一个圆角框里时，里面的圆角不大于外面的。

断点沿用网站现在的，以 rem 为单位：`22.5rem` (最窄的手机)、`48rem` (弹出层占满整栏)、`56rem` (侧边栏收到按钮后面) 和 `70rem` (目录收到按钮后面)。

## <a id="icons"></a>图标

页面上的图标全部来自一套图标库 Lucide (ISC 许可)，用以 `currentColor` 绘制的 SVG，在两套主题下都随所在文字取色。不手画图标，也不拿字符充当图标 (`ⓘ`、`▸`、`×`、`✓`、`+`)。文件从 Lucide 的发布版拷进仓库，旁边放上它的许可和拷贝时的版本号；不为此安装任何包。文件放在哪里、页面怎么引入，见[代码实现](#in-the-code)。

唯一的例外是指向源码的链接上的 GitHub 标志：它是 GitHub 的 logo，不是图标，仍用 GitHub 自己画的那个 (Octicons 的 `mark-github`，MIT 许可)。

| Token | 值 | 用途 |
| --- | --- | --- |
| `icon-sm` | `1em` | 一行文字里或 tag 上；随文字缩放。 |
| `icon-md` | `1.25rem` | 控件上和侧边栏里。 |
| `icon-stroke` | `1.75` | 所有图标，按 Lucide 的单位计 (24 个单位见方)。 |

所有图标用同一个线宽，按 Lucide 的画法只用描边，不用填充。图标在所在行里垂直居中，大小在 16px 到 24px 之间。图标全部放在一个目录里，每个都以来源加名字命名 (`lucide:funnel`)，不在用到它的页面标记里整段写出来。只起装饰作用的图标对读屏软件隐藏 (`aria-hidden`)；只显示图标的按钮要给读屏软件一个名称 (`aria-label`)。

现有图标换成什么：

| 现在 | 位置 | Lucide |
| --- | --- | --- |
| Octicons `law` | 侧边栏里宪法的分组 | `scale` |
| Octicons `checklist` | 规定的分组 | `list-checks` |
| Octicons `book` | 设计的分组 | `book-open` |
| Octicons `device-desktop`、`sun`、`moon` | 配色开关 | `monitor`、`sun`、`moon` |
| 手画的圆圈套 `i` | 长度后面的说明按钮 | `info` |
| 手画的圆圈套加号或减号 | 展开某条款下规定的按钮 | `circle-plus`、`circle-minus` |
| 主题自带的箭头 | 可以折叠的分组、语言切换 | `chevron-right`、`chevron-down` |
| 主题自带的 `translate` | 语言切换 | `languages` |
| 主题自带的 `menu` 和 `toc` | 窄屏顶栏上的两个按钮 | `menu`、`table-of-contents` |
| 文字按钮 | 筛选器 | `funnel` |
| tag 里状态的文字 | 列表和图例里规定的状态：生效、试行、草案、已取代、已废止 | `circle-check`、`flask-conical`、`pencil`、`arrow-right-left`、`circle-slash` |
| 无 | 列表和图例里规定的用途：指令、审查 | `scroll-text`、`scan-eye` |
| 无 | 分组行路径里领域之间 | `chevron-right` |
| 字符 ‹ 和 › | 列表的上一页和下一页 | `chevron-left`、`chevron-right` |
| 无 | 关闭弹出层 | `x` |
| 无 | 挂在另一条规定下的规定 | `corner-down-right` |
| 无 | one-of 的选项、默认选项 | `circle`、`circle-dot` |
| 无 | 规定清单启用了它 | `check` |

## <a id="motion"></a>动效

动效只用在三处：折叠的部分展开或收起，指针或键盘焦点带来的变化，弹出层打开或关闭；另外，锚点跳到的目标闪一次。没有东西会自己动，滚动时和页面加载时都不动。经常做的操作完全不动：列表翻页、改动筛选器，以及一切用键盘做的操作 (快捷键、Escape) 都立即生效。

| Token | 值 | 用途 |
| --- | --- | --- |
| `duration-fast` | `120ms` | hover 和焦点的颜色变化、弹出层或折叠部分关闭。 |
| `duration-base` | `200ms` | 展开、弹出层打开、箭头转向。 |
| `duration-flash` | `2000ms` | 锚点目标从 `bg-target` 淡出。 |
| `delay-tip` | `300ms` | 提示气泡，或只做说明的弹出层，在指针下等多久才打开。 |
| `ease-out` | `cubic-bezier(0.23, 1, 0.32, 1)` | 所有动效，打开和关闭都用它。 |

读者触发的动效都不超过 300 ms，闪一下的那个除外，读者不必等它。关闭比它所撤销的打开更短 (`duration-fast` 对 `duration-base`)，两者都用 `ease-out`：界面上没有慢慢起步的动效。动的只有透明度和 transform，状态变化时还有颜色；过渡要写明动哪些属性，不写 `all`。弹出层从 `scale(0.95)` 和 `opacity: 0` 进场，不从 `scale(0)` 进场，`transform-origin` 对准打开它的控件；箭头转四分之一圈。

在 `prefers-reduced-motion: reduce` 下，没有东西移动或缩放：去掉缩放和转向，弹出层和折叠部分在原地出现。表示状态变化的部分保留，改成淡入淡出：弹出层照样淡入淡出，颜色照样在 `duration-fast` 内变化，锚点目标照样从 `bg-target` 淡出。

## <a id="interaction-and-accessibility"></a>交互与无障碍

- **点击区域。** 能点的东西至少 24px 见方 (padding 算在内)，在粗指针 (`pointer: coarse`，即触屏) 上至少 44px 见方，即 token `target`；小图标靠 padding 达到这个大小，不靠把图画大。行文中间的链接不在此列：它的高度就是行高。
- **hover。** hover 样式只在有能悬停的精确指针时生效 (`@media (hover: hover) and (pointer: fine)`)，触屏上点一下不会留下 hover 样式。
- **名称和状态。** 控件就用本来是控件的元素 (`button`、链接、`details`、`select`)，只为元素本身表达不了的东西加 ARIA。打开或折叠某个部分的控件要说明它是否打开 (`aria-expanded`) 以及它打开的是什么 (`aria-controls`)。控件的名字说它作用于什么，不重复旁边链接的文字：侧边栏里折叠一组页面的按钮，名字是这一组的页面 (「设计的页面」)，因为组的链接已经写着组名。带控件的弹出层打开时接过焦点，关闭时把焦点交还给它的控件；按 Escape 关闭。
- **提示气泡。** hover 时显示的提示在 `delay-tip` 之后出现；已经有一个显示过之后，指针移到下一个上面时立即出现。提示里不放页面别处没有的内容：它说的内容也是元素的无障碍名称，或者写在图例里。
- **状态写进地址。** 列表的筛选、每页条数和页码都在地址里，列表可以直接链接，前进后退能回到原来的样子。
- **不靠脚本。** 内容默认可见：脚本出错或根本没运行的页面照样显示全部文字，靠脚本展开的部分保持展开。
- **空状态和加载。** 筛选后为空的列表要说明为空，并给出下一步：「恢复默认筛选」。如果以后页面要在读者等待时加载东西，加载提示约 150 到 300 ms 后才出现，出现后至少显示 300 到 500 ms，避免一闪而过；两个网站现在都没有这样的加载。

| Token | 值 | 用途 |
| --- | --- | --- |
| `target` | `24px`；在 `pointer: coarse` 下 `44px` | 能点击的东西最小的宽和高。 |

## <a id="components"></a>组件

下面每个组件都写出静止时的样子和各种状态：指针悬停 (hover)、按下 (active)、获得键盘焦点 (focus，即[颜色](#color) 一节的焦点框)、当前或选中 (selected)、禁用 (disabled)，有哪些写哪些。

- **标记。** 在规定列表里，一行的标记放在行尾的四个位置，每个宽 `width-mark`，合起来约四个字符宽，依次是：状态，默认是否启用，以及用途里的指令和审查。没有某项标记的位置空着，这样上下行的标记对齐；这一列没有表头，每个位置放什么由图例说明。每个标记是一个图标 (`icon-sm`) 或一个圆点，用它的词给读屏软件命名，hover 时也显示这个词 (`role="img"`、`aria-label` 和 `title`)：
  - 状态：Active 是 `status-active` 的 `circle-check`，Trial 是 `status-trial` 的 `flask-conical`，Draft 是 `status-draft` 的 `pencil`，Superseded 是 `arrow-right-left`、Repealed 是 `circle-slash`，这两个用 `text-muted`。
  - 默认是否启用：`size-dot` 的圆点，默认开启时用 `accent` 填满，默认关闭时是 `text-muted` 的空心圆圈；要看别的规定源才定得下来时，这个位置空着。
  - 用途：指令是 `scroll-text`，审查是 `scan-eye`，都用 `text-muted`。
  - 标记本身不响应指针，响应的是整行。
- **tag。** 小号，`text-xs`、`line-height-ui`，只占一行，不折行，用在一行的四个位置放不下的事实和规定自己的页面上：
  - 默认是否启用，在节点自己的页面上：标记里的那个圆点后面跟着词，用 `text-muted`。
  - 规定清单启用了它：词前加 `check` (`icon-sm`)，用 `accent`；没有启用：只有那个词，用 `text-muted`。
  - 规定源的象限：`radius-sm` 的方框，填象限的颜色，文字用 `text`。
  - 在列表里，网站上有的话，是否启用和象限跟在四个标记位置后面，各占一列。
- **图例**，放在规定列表上方，同 Ruff 的规则列表一样：列表里一行带的每种标记，照列表里的样子画出来，后面跟着它的词，再用一行说明它的含义，按标记位置的顺序：每种状态、默认开启和默认关闭、指令和审查；然后是 one-of 的默认选项，网站上有的话，还有已启用、未启用和象限。一个术语和它的含义占一行 (描述列表)，`text-sm`，术语用 `weight-medium`，含义用 `text-muted`；下面一行说明四个位置和它们的顺序。
- **筛选器**，做到 Ruff 规则列表那个程度为止：不做条件组合。收起时是一个按钮：`funnel` (`icon-md`)、「筛选」二字，读者改动了某些字段、与默认筛选不同时，再加一个徽标显示这样的字段有几个 (`radius-full`，`bg-selected` 上的 `accent`)；默认筛选不显示徽标。打开时是按钮下方的弹出层，宽 `width-popover` (窄屏上与栏同宽)，每个字段一组复选框：状态、默认开关 (开、关)、用途、领域 (按深度缩进)，有多个规定源时加象限。每组先是名字，然后是「全选」和「不选」两个普通按钮。同一字段里，规定要有一个勾上的值；不同字段之间，每个字段都要满足；所有值都勾上的字段不起筛选作用。勾上或取消一个领域时，它下面的领域跟着变。各组下面是「恢复默认筛选」，回到默认筛选，即地址里不带筛选时列表的样子；要看全部状态，读者在状态组里点「全选」。改动立即生效并写进地址，筛选后的列表可以直接链接；没有「应用」按钮。按 Escape、点击外面或焦点移出时关闭。不带脚本的网站 (本地网站就是) 在弹出层里显示同样的几组，用一个主按钮应用。本地网站眼下与此不同：它的复选框靠样式表立即生效，没有主按钮；它的几组是规定源的类别、用途，以及另外显示的未启用的规定和已被取代或已废止的规定；状态、默认开关和领域这三组，「全选」和「不选」，以及徽标，它都还没有 (见[现状](#status))。
- **规定表**，像 ESLint 的规则列表那样排成一张表：先是表头，然后一行一条规定，共三列：标号 (`width-label`，`text-muted`，向末端对齐)、标题 (超出一行时以省略号截断) 和标记；表头写前两列的名字，`text-sm`、`weight-medium`、`text-muted`，下面一条 `border-strong` 的线。每一行至少有一个点击区域那么高，下面一条 `border-subtle` 的细线，宽屏上各行一样高。挂在另一条规定下的规定不缩进，而是在标题前加 `corner-down-right` (`icon-sm`，`text-faint`) 表示。整行是一个链接：hover 时整行填上 `bg-hover`，标题变成 `accent`；按下时填上 `bg-active`，标题和标号都改用 `text`；获得焦点时焦点框围住整行。领域不单占一行，而是它的规定上方的一条分组行，横贯整张表的一条底色：第一层的领域用 `bg-active`，更深的用 `bg-panel-hover`；领域的标号在标号那一列，后面是它所在的各层领域组成的路径，中间用 `text-muted` 的 `chevron-right` 隔开 (`Documentation › Design`)，全都用 `text` 和 `weight-bold`；整个路径是到领域页面的链接，指针下加下划线，因为 `accent` 在这条底色上会低于 4.5。最近的领域与前一条显示的规定不同的地方，以及每一页的顶上，都显示分组行，所以列表从下级领域回到上级领域时，会再出现一次上级领域的分组行。表头吸在窗口顶上 (窄屏时在顶栏下面)，每条分组行吸在表头下面，直到下一条分组行把它盖住，这样屏幕上那些行属于哪个领域总写得出来；只有最近那个领域的分组行吸顶，因为它的路径已经写出了上面的各层。窄屏上标题需要几行就占几行，表头和分组行只占一行。
- **one-of。** 一个框 (`1px` 的 `border-strong`，`radius-md`) 围住它自己的那一行和各选项的行，各列与表格对齐，框的上边线上有一个标签 One of / 「择一」(`text-xs`、`weight-medium`、`text-muted`，底色与所在表面相同，边线在它两侧断开)。它自己那一行的标题用 `weight-medium`。每个选项是一行，与表里的一样，标题前是 `text-muted` 的 `circle`；默认选项的标题前是 `accent` 的 `circle-dot`，标题后跟着它的 tag Default option / 「默认选项」，用 `accent`，前面有 `check`，所以默认选项在形状上也区分得出，不只靠颜色。选项都关闭的 one-of，框用 `border-subtle`。
- **弹出层** (筛选器的面板、长度的说明、在正文上方展开的条款下规定)：`bg-panel`，`1px` 的 `border-subtle`，`radius-md`，`shadow-popover`，padding `space-3 space-4`，出现在打开它的控件下方。带控件的弹出层靠点击、Enter 或 Space 打开和关闭，控件的 `aria-expanded` 和 `aria-controls` 表明是否打开、打开的是什么；打开后焦点移进去，关闭时回到控件上。只做说明的弹出层 (长度的说明) 也可以在 hover 时打开，延迟与提示气泡相同。按 Escape 或点击外面都会关闭。打开时在 `duration-base` 内从 `scale(0.95)` 和透明进场，关闭用 `duration-fast`，两者都用 `ease-out` ([动效](#motion))。
- **侧边栏。** 照 Tailwind CSS 文档的侧边栏做 ([出处](#sources))，配色和字体用本文的。`bg-panel`，边上一条 `1px` 的 `border-subtle`，分组之间隔 `space-4`。分组名：图标 (`icon-md`) 加名字，`text-md`、`weight-bold`、`text`。分组下的页面沿一条细线排列，细线是 `1px` 的 `border-subtle`，从图标中线的正下方开始。页面：`text-md`、`weight-regular`、`text-muted`，padding `space-1 space-2`，文字与分组名对齐，压在细线上有一条 `2px` 的短线，平时透明。一行 (分组或页面) 在指针下：`bg-panel-hover`，`radius-sm`，靠细线的一侧不圆；页面还会变成 `text`，短线变成 `border-strong`。按下：`bg-active`。当前页：短线和文字都用 `accent`，文字 `weight-medium`，不加底色。没有页面的分组是当前项时，没有细线可标，就用 `bg-selected` 底，文字用 `accent`。当前页列在某个分组下面时，只标出这一页，不标分组。页面加载时，当前页不在侧边栏的可见范围内，侧边栏就滚到能看见它。可以折叠的分组在这一行末尾有一个普通按钮，收起时显示 `chevron-right`，展开时 `chevron-down`，转向用 `duration-base`；这个按钮和语言切换按钮都标明自己的列表是否展开 (`aria-expanded`、`aria-controls`)。
  - 右侧的目录：`text-sm`，标题用 `text-muted`；各个标题像侧边栏里的页面一样沿一条细线排列，`text-muted`，低一级的标题缩进 `space-4`，短线仍压在细线上。指针下：`text`，短线 `border-strong`。读者正在读的那个标题：短线和文字用 `accent`，`weight-bold`。它随滚动变化，所以这些标记立即切换，不加过渡。
  - 窄屏的抽屉。宽度在 `56rem` 及以下时侧边栏、在 `70rem` 及以下时目录，收到顶栏的一个普通按钮后面，按钮分别是 `menu` 和 `table-of-contents`，抽屉打开时显示 `x`，并标明抽屉是否打开、打开的是哪一个 (`aria-expanded`、`aria-controls`)。`56rem` 及以下，顶栏固定在窗口顶部，`bg-page` 底，下面一条 `1px` 的 `border-subtle`。抽屉在顶栏下面从各自的一边打开，侧边栏从起始边，目录从末尾边：`bg-panel`、`1px` 的 `border-subtle` 和 `shadow-popover`，因为它盖在正文上。滑入用 `duration-base`，滑出用 `duration-fast`，都用 `ease-out`，后面的页面淡到原来不透明度的四分之一；减弱动效时抽屉在原地淡入淡出。抽屉打开时，页面的其余部分不可操作：焦点移到抽屉里显示出来的第一个链接；按 Escape、再点按钮、点抽屉外面，或在抽屉里点了链接，抽屉都会关上，由键盘或按钮关上时焦点回到按钮。侧边栏不重复站名，顶栏已经有了。没有脚本时没有抽屉：侧边栏排在正文上面，目录排在正文下面，所有列表都展开：折叠侧边栏各组的按钮藏起来，语言切换的 chevron 转到展开的方向，点它的按钮没有反应。
- **按钮和输入框。** 主按钮 (每个弹出层至多一个)：`accent` 底，文字 `text-on-accent`，`radius-sm`，padding `space-1 space-3`；hover 时 `accent-hover`，禁用时半透明。普通按钮 (大多数按钮：配色开关、筛选器的按钮、只有图标的按钮)：无填充，`text`，hover `bg-hover`，按下 `bg-active`。输入框和菜单：`bg-page`，`1px` 的 `border-strong`，`radius-sm`，指针悬停时边框变成 `text-muted`；获得焦点时显示焦点框。原生控件在两套主题下都不保留浏览器自己的样子：菜单 (`select`) 末尾显示 `chevron-down`；浏览器允许页面自己画它的选项列表时，列表是一个弹出层，每个选项一行，选中的那项用 `bg-selected` 底，文字用 `accent` 和 `weight-medium`；浏览器自己画列表时，只有颜色跟随主题。复选框和单选按钮：`icon-sm` 大小的框，`bg-page` 底，`1px` 的 `border-strong`，复选框用 `radius-sm`，单选按钮是圆的；hover 时边框变成 `text-muted`；选中时框里填满 `accent` (指针悬停时 `accent-hover`)，复选框的勾是 `text-on-accent` 的 `check`，单选按钮的圆点靠框内一圈 `space-1` 宽的 `bg-page` 显出来；禁用时半透明。`summary` 显示 `chevron-right`，展开时转四分之一圈，取代浏览器自带的标记。

## <a id="not-done"></a>不做的事

- 只有一个强调色 `accent`，不为强调再加第二个。
- 不用渐变文字，不用装饰性的模糊或毛玻璃，不用发光效果。
- 标题上方不加小号大写的标签 (eyebrow)，没有顺序的内容不编 01、02、03 这样的号。
- 不用第二套图标库的图标，不拿 emoji 或字符当图标，描边图标里不混入填充图标。
- 不写 `transition: all`，不从 `scale(0)` 进场，界面上不用 `ease-in`，加载时和滚动时没有动效。
- 不能悬停的设备上没有 hover 样式。

## <a id="screenshots-and-review"></a>截图与审查

- **截图。** 改动页面样子的 pull request (改样式、模板、图标、组件或组件的状态) 要附上它改到的每个页面的截图：light 和 dark 两套主题，英文和中文，每页四张。截图由仓库里的截图脚本生成，脚本见[静态网站](static-site.md)。说明里写清截了哪些页面、哪些状态。
- **CI。** 每个 pull request 的 CI 构建网站、检查链接、运行截图脚本，并把截图保存为这次运行的 artifact ([静态网站](static-site.md))。
- **审查。** 审查方拿本文对照改动逐条核：该用 token 的地方没有直接写值，图标只来自 Lucide，动效只出现在[动效](#motion) 允许的地方、减弱动效时只剩淡入淡出，符合[交互与无障碍](#interaction-and-accessibility) 和[不做的事](#not-done)，改到的每个组件的每种状态都按这里的样子画、两套主题都有，四种截图齐全。改动需要本文没有的值或组件时，在同一个 pull request 里把它加进本文。
- **视觉对比。** 拿截图与 `main` 上的截图对比、找出没人想改的变化，现在不做；等 CI 能生成截图之后可以再做。

## <a id="in-the-code"></a>代码实现

- **token。** 本文的全部 token 定义在一份样式表 `crates/statute/assets/tokens.css` 里，每个是一个 CSS 自定义属性，名字是 token 名加前缀 `--t-` (`--t-text`、`--t-space-4`)；有了前缀，就不会与主题自己的属性混在一起，主题里就有一个取值不同的 `--space-1`。颜色只写一次，两个值一起写成 `light-dark(<light>, <dark>)`，随页面的 `color-scheme` 取值：跟随系统设置，页面根元素带 `data-theme` 时按读者的选择。英文和中文取值不同的 token 在 `:root:lang(zh)` 下重新设定，`target` 在 `pointer: coarse` 下重新设定。这份文件放在 crate 里，是为了让 `statute` 能用 `include_str!` 把它编进本地网站，同时照样能发布：发布的包不能引用 crate 目录以外的文件。公开网站从那里挂载它 (`web/hugo.toml`)。
- **样式表。** 公开网站的样式表在 `web/assets/styles/`，页面的每个部分一份，改不同的部分就改不同的文件：`site.css` (主题变量、链接、焦点框、图标、侧边栏、目录、顶栏)、`controls.css` (各处的原生控件)、`constitution.css`、`design.css` (设计文档和单条规定的页面)、`rules-list.css` (规定列表、它的标记和 tag)、`filter.css` (列表上方的图例、筛选器和每页条数，它们的标记在 `web/layouts/_partials/rules/`)。Hugo 按 `web/assets/styles/index.yaml` 的顺序，把它们接在主题的样式表和 `tokens.css` 后面合成一个文件。本地网站自己的样式表是 `crates/statute/src/site.css`，`statute` 把它接在 `tokens.css` 后面写进每个页面；复选框照 `controls.css` 的画法画，公开网站其余几份样式表是给主题的标记用的，它一概不取。
- **图标。** SVG 文件在 `crates/statute/assets/icons/`，每个来源一个目录，`lucide/` 和 `octicons/`，与发布版里的一样、不做改动，各目录里放着许可，版本号写在它的 `README.md` 里。页面按名字引入图标，`partial "icon.html" "lucide:funnel"`，把文件里的 SVG 压成一行内联进来，带上 class `icon`，并对读屏软件隐藏；这个 class 设定线宽 `icon-stroke` 和大小，大小默认 `icon-sm`，组件要求时改成 `icon-md`。会转向的箭头用 `chevron-right` 转四分之一圈，转过去就是 `chevron-down`。本地网站用同样的办法内联它要的那几个文件，借 `include_str!` 编进 `statute`。
- **脚本。** 逻辑超过几行的脚本 (目前是规定列表和它的筛选器，以及侧边栏和抽屉) 用 TypeScript 写，放在 `web/assets/ts/`，不依赖页面的逻辑 (筛选的勾选和地址) 单独放一个文件。每个脚本由 Hugo 的 `js.Build` (Hugo 自带的 esbuild) 打包，不用 Node，不用 npm 包；目标是 `es2018`，压缩，页面像别的脚本一样把它内联在 head 里。不生成 source map，因为脚本很小，源码就在仓库里。短小的脚本仍是 `web/assets/` 里的 JavaScript，由 Hugo 压缩。2026-10-15 用 Hugo 0.167.0 核实：`js.Build` 去掉类型和只导入类型的 import，把文件之间的 import 打包到一起，把 `?.` 和 `??` 降级到目标版本，遇到语法错误时构建失败。
- **类型和测试。** esbuild 不检查类型：有类型错误照样构建，构建照样通过。机器上和 CI 上都没有 `tsc`，装它要用 Node 和一个网站别处用不到的 npm 包，所以目前没有任何工具检查类型，由审查方读。行为改在构建出的页面上测：`tools/site` 的一个测试用虚构的规定构建网站，在 headless Chrome 里依次带着一组地址打开列表，读 Chrome 留下的页面 (`--dump-dom`)；没有 Chrome 时跳过，CI 里不跳过。`--dump-dom` 发不出点击和按键，这些用同样的办法手工检查：在页面的副本里加一个发出它们的脚本。

## <a id="status"></a>现状

公开网站上已经实现的：全部 token，在 `tokens.css` 里；样式表里的颜色、字号、字重、间距、圆角、阴影、时长和缓动都写 token 名，下面尚未实现的部分除外；[图标](#icons) 一节表里现有页面用得到的图标；[动效](#motion) 一节的动效，连同减弱动效，用在现在会动的地方 (长度说明的弹出层、箭头、锚点目标的闪烁、窄屏上的抽屉)；只在精确指针上用 hover；点击区域的大小；[组件](#components) 一节的侧边栏、目录和抽屉；原生控件；字体、正文字号和栏宽，以及断点；筛选器、图例、带标记的规定表及其吸顶的分组行和表头、one-of；筛选和页码写进地址，不靠脚本也能读的页面，等宽数字，以及长度说明按钮、条款下规定的按钮、侧边栏分组和语言切换的按钮、顶栏按钮上的 `aria-expanded` 与 `aria-controls`。本地网站上已经实现的：全部 token，样式表里的值都写 token 名；[组件](#components) 一节里的 one-of；筛选器的按钮和弹出层；复选框；只在精确指针上用 hover，点击区域的大小，焦点框，以及筛选弹出层的动效。尚未实现的，以及各自取代什么：

- 公开网站上，规定页面上的 tag，以及列表里的是否启用和象限，都是一模一样的方框；[组件](#components) 一节里的 tag 取代它们。选项都关闭的 one-of，框仍用 `border-strong`。
- 本地网站把规定排成一棵领域树，每个领域是一个标题，字号和字重表示层级，更深的一层缩进并带色条；每条规定的事实是行尾的 tag，状态是胶囊，另有说明这些 tag 的图例。[组件](#components) 一节里的规定表、标记、tag 和图例也要在那里取代它们。
- 本地网站上，筛选器里状态、默认开关和领域这三组，每组的「全选」和「不选」，按钮上的徽标，按 Escape 或点击外面时关闭；以及应用筛选的主按钮，眼下由立即生效的复选框代替。它的弹出层一下子关上，只能再点一次按钮来关。它的页面不加载字体，正文用本机的衬线字体。

## <a id="open-questions"></a>待定问题

目前没有。

## <a id="sources"></a>出处

上文有几处取自已发布的界面规范，2026-10-15 按下列 commit 读取：

- emil-design-eng，Emil Kowalski 讲动效的 skill (<https://github.com/emilkowalski/skills>，`2d228630`)：经常做的操作不加动效，界面用 `ease-out`、关闭更短，`ease-out` 的曲线，弹出层从 `scale(0.95)` 进场，减弱动效是减少而不是归零，hover 只在精确指针上生效，提示气泡的延迟。
- Vercel 的 Web Interface Guidelines (<https://github.com/vercel-labs/web-interface-guidelines>，`434b7f91`)：点击区域的大小，状态写进地址，两层阴影，嵌套圆角，等宽数字，加载提示的时机，状态提高对比度，不写 `transition: all`。
- UI Skills (<https://github.com/ibelick/ui-skills>，`c628a591`)：名称、`aria-expanded` 和 `aria-controls`、先用原生元素再用 ARIA (fixing-accessibility)；只动 transform 和 opacity (fixing-motion-performance)；空状态给下一步 (baseline-ui)。
- Impeccable (<https://github.com/pbakaus/impeccable>，`a231cc5b`)：不靠脚本内容也可见，只用一套图标库，正文字号的下限和行长，「不做的事」里的一部分。
- Anthropic 的 frontend-design skill (<https://github.com/anthropics/skills>，`dbd4588f`)：一个强调色，「不做的事」里的一部分。
- UI Design Brain (<https://github.com/carmahhawwari/ui-design-brain>，`38f04c5a`)：边框或阴影二选一，表格里的数字靠末端对齐，图标 16px 到 24px、描边和填充不混用，提示里不放页面没有的内容。
- Better Icons (<https://github.com/better-auth/better-icons>，`033316ec`)：图标以来源命名，放在一处。
- Tailwind CSS 的文档 (<https://tailwindcss.com/docs>，2026-10-15 时网站上的页面，没有 commit 可写)：侧边栏和目录的细线，以及标当前项的短线；平时淡、指针下变深的列表项；随滚动变化的当前标题；侧边栏把当前页滚进可见范围；抽屉在顶栏下打开时顶栏不动。
