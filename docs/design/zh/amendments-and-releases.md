<!-- en-sha256: 639a7960776d45d1099d0f0df34842227a85b766884f849dd12791f7bbba617d -->
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
- 推送发布 tag 就使一个版本可以获取。默认情况下，发布是 `must-ask` 的一个事项，由第 32 号规定 (`release-must-ask`) 加入，需要人同意；项目可以依照 `relaxations.project-overrides` 另行规定。
- `crates/` 下的 crate 发布到 crates.io，各自的版本写在自己的 `Cargo.toml` 里；发布一个 crate 也是一次发布。`statute` 0.0.1 是当初占住这个名字的占位版本。
- crate 的版本与仓库的版本各自独立，所以 crate 的 tag 带上 crate 的名字：推送 `statute-v<version>` 就是发布这一版的 `statute`，与上面的发布 tag 守同样的规矩。发布由 `.github/workflows/release.yml` 完成。这个 workflow 不持有 crates.io 的 token：crates.io 信任本仓在 GitHub environment `release` 里运行的 `release.yml` (Trusted Publishing)，job 用自己的 OIDC token 换一个短期的 crates.io token。这个 environment 只接受 `statute-v*` 形式的 tag。
- 发布之前，由一个单独的 job 运行 `check-release-tag`，它不在这个 environment 里，也拿不到 OIDC token；它通过了，负责发布的 job 才运行，所以检查用到的任何工具都拿不到这个 token。检查依次分三步，前一步通过才做下一步，哪一步不通过就停下：tag 等于 `statute-v` 加上 `crates/statute/Cargo.toml` 里的版本，manifest 里读不出版本也算不通过；打 tag 的提交在 `main` 上；这个提交由 push 触发的那次 `ci.yml` 运行已经结束，并且第一次就是 `success`。所以 tag 要等这次运行结束后再推；推早了检查不通过，等 CI 运行通过后重跑这个 workflow。
- 同一个 tag 还会在它的 GitHub Release 上挂出预编译的 `statute` 二进制，覆盖 `x86_64-unknown-linux-musl`、`aarch64-unknown-linux-musl`、`x86_64-apple-darwin` 与 `aarch64-apple-darwin`：每个 target 一个 asset `statute-<target>.tar.gz`，二进制在压缩包的顶层，旁边附一个 `sha256sum` 格式的 `statute-<target>.sha256`。Linux 的二进制静态链接 musl，在 glibc 的系统上也能跑。二进制要等 `check-release-tag` 通过才构建，构建它们的 job 既没有 OIDC token，也没有能写仓库的 token；另有一个不构建任何东西的 job，在 tag 还没有 Release 时建一个，再上传这些 asset。没有安装脚本；CI job 或者人用下面的命令装某一版，`version` 设为版本号 (比如 `0.1.17`，它是第一个带二进制的版本)，`target` 设为四个 target 之一：

  ```sh
  base=https://github.com/Luolc/agent-constitution/releases/download/statute-v$version
  curl -fsSLO "$base/statute-$target.tar.gz" -O "$base/statute-$target.sha256"
  sha256sum --check --strict "statute-$target.sha256"
  tar -xzf "statute-$target.tar.gz" statute
  ```

  在 macOS 上用 `shasum -a 256 --check` 代替 `sha256sum --check`。手动运行这个 workflow (`workflow_dispatch`，必须填 input `tag`，即一个已有的 `statute-v*` tag，没有默认值) 构建四个二进制，作为 workflow artifact 保留，不跑检查，也不发布任何东西。
- 同一个 tag 还把 Homebrew formula `Formula/statute.rb` 写进 tap `Luolc/homebrew-tap`，使 `brew install luolc/tap/statute` 装上 Release 里的二进制。Release 的资产齐了之后，一个跑在 macOS 上的 job 用 `tools/render_homebrew_formula.sh` 从四个 `.sha256` 文件渲染 formula：它指向 Release 的 tarball，Homebrew 不编译任何东西。这个 job 把 formula 放进一个本地 tap，用 `tools/check_formula_version.sh` 核对 Homebrew 读到的版本就是 tag 的版本，再跑 `brew audit --strict`、`brew install` 和 `brew test`。只有 tag push 时，它才用仓库级 secret `HOMEBREW_TAP_TOKEN` 把 formula 提交到 tap 的 `main`；formula 没变就不提交。dispatch 对 input `tag` 里的那个 tag 跑除这次提交之外的每一步，不写 tap。这个 job 失败时，crates.io 和 Release 都保持原样。
- 同一个 tag 还会把 `statute` 发到 PyPI：wheel 里原样装着 Release 的二进制，装好后就是 `statute` 命令 (`uv add --dev statute`、`pip install statute`)。每个 Linux target 出 manylinux 与 musllinux 两个 wheel，里面是同一个 musl 二进制；每个 macOS target 一个 wheel；没有 Windows 的 wheel。wheel 的版本取 tag 的。`pypi-wheels` 这个 job 没有能发布的 token，它下载 Release 的 asset，用 `tools/build_pypi_wheels.sh` 打出六个 wheel，再跑 `tools/check_pypi_wheels.sh`：每个 wheel 里二进制的 sha256 与它的 Release 压缩包里那份相同，两个值都打出来，并且 wheel 正好是预期的那六个。之后它跑 `twine check --strict`，装上 x86_64 Linux 的 wheel，核对 `statute --version` 打出的是 tag 的版本。`publish-pypi` 在它和 crates.io 之后跑，在 environment `release` 里，是这两个 job 里唯一拿 OIDC token 的：PyPI 信任本仓在这个 environment 里运行的 `release.yml` (Trusted Publishing)，所以仓库里也没有 PyPI 的 token。第一次上传之前，要由人在 PyPI 上把这个 workflow 和 environment 登记为项目 `statute` 的 pending publisher。上传之前，`tools/pypi_upload_set.sh` 去掉 PyPI 上已有、sha256 相同的 wheel，遇到 PyPI 上同名而 sha256 不同的就停下，所以上传中途失败的 run 可以重跑。dispatch 从 input `tag` 里那个 tag 的 Release 打出并检查它的 wheel，作为 workflow artifact 保留，不上传。没有哪个 job 只为检查 publisher 而向 PyPI 要 token：对 pending publisher，PyPI 在发 token 的那一刻就建出项目。
- overlay 固定一个 tag 及其提交 SHA，两者对不上就拒绝构建。它换用新版本，要在 overlay 里提一次变更，在那里审查。
