# Source update / 源码更新 — 2026-10-29

This experimental fork now includes completed work through round 26 and the
2026-10-28 source audit. It updates the source previously published through
round 17. No installer, compiled application, tag or GitHub Release accompanies
this commit; the upstream version number is not a new fork release number.

本次将此前公开到第17轮的源码更新至第26轮，并纳入2026年10月28日源码审计的
六处修复与优化。这是实验性改进分支的源码更新，不提供安装包或编译程序，
不创建标签或GitHub Release，也不把上游版本号当作本分支的新发行版本。

## Included improvements / 更新内容

- Document and selection context checks for delayed Table operations; Windows
  close lifecycle repairs; recovery index, shutdown and clock-order protections.
- A pinned, locally patched GPUI component source tree fixes stale text-layout
  hit testing. Its upstream MIT licence and resource notices remain included.
- CSV delimiter handling and an optional **Keep all fields as text** import mode
  preserve numeric/date-like spellings. Ordinary numeric import remains default.
- Opt-in English/Chinese selected-text translation in ElyDoc: configurable Chat
  Completions endpoint/model, system credential storage, preview and copy.
  It sends only the previewed selection on explicit request. A local endpoint
  may still relay to a cloud service. ElyAgent remains unfinished.
- The source audit fixes Windows shell interpretation of file names, attachment
  name boundaries and the final OLE macro-directory slot. It bounds worker-log
  records, reads only the required log tail, and avoids discarded event copies.

- 加强表格延迟操作的原文档与选区校验、Windows关闭生命周期、恢复索引、退出
  保存和时钟顺序保护。
- 纳入固定版本及本地修复的GPUI组件源码，修复旧文本布局与新文本不匹配的问题，
  保留MIT许可和资源声明。
- 修复CSV分隔符处理，增加可选的全部按文本导入，保留数字与日期原始写法，
  普通数值导入仍是默认行为。
- ElyDoc提供默认关闭的中英选区翻译，可设置地址、模型及系统保存的密钥。
  仅在明确点击时发送预览选区，译文可预览和复制。局部服务可能转发到云端，
  不宣称只要使用本地地址就完全离线；ElyAgent仍未完成。
- 审计修复特殊文件名解释、附件名称越界和OLE宏末目录项漏检；限制长日志读取，
  裁剪日志时仅读尾部，减少事件快照的无用复制。

## Validation and remaining limits / 验证与限制

The latest source audit passed **501 distinct Rust tests and 15 website tests**,
including 12 new Rust regressions. The Rust groups were IPC 8, files 132,
platform 62, Office shell 34, event Feed 2, Doc 79, Table 87, Show 92 and i18n 5.
Two existing subprocess entry points remain marked ignored and are called by
their parent tests; they are not added to the pass count. Historical/repeated
runs are not accumulated. The frozen regressions, before/after Windows filename
launch fixture, source protection checks and independent R1 review passed.

最近一次源码审计通过501项不同Rust测试及15项官网测试，其中新增Rust回归12项。
已有的两个子进程测试入口仍标记ignored，由父测试调用，不计入通过数量。
修前失败、旧构建缓存运行及历史重复运行均未累加。Windows特殊文件名实际启动
对照、原文件保护及独立R1审查通过；发布准备期间没有重复运行产品测试。

Full-workspace Clippy could not finish because the Windows MuPDF build library
was absent. All targets of the other 12 first-party workspace members passed,
with seven pre-existing warnings. PDF/Mark/MuPDF-dependent targets, full UI/LOK
suites, website production build and browser E2E are not claimed validated.
Native translation trials used a fixed local fixture, not a real model service.
Macro scanning still reads full files on the window callback and can delay large
file opening. General Office fidelity, system IME, multiple displays, extended
use, installation/upgrade/uninstall, and macOS/Linux runtime acceptance remain
open. The earlier round-23 failure keeps its historical attribution limit.

全工作区静态检查因缺少MuPDF的Windows编译库未完成；其余12个自有成员的全部
目标检查通过，保留7条旧警告。不据此验收PDF/Markdown、整套UI/LOK测试、官网
生产构建或浏览器端到端流程。翻译验证使用固定的本机测试服务，未验证真实模型
质量。宏扫描仍可能因同步整文件读取拖慢大文件打开。复杂文档、输入法、多屏、
长期使用、安装升级和其他平台等仍有待验收，旧失败记录没有被改写为通过。

See the [source-audit report](docs/source-audit-20261028.zh-CN.md),
[its independent review](docs/reviews/source-audit-20261028-r1.md),
[round 26](docs/windows-validation-round26.md),
[round 25](docs/windows-validation-round25.md), and
[the improvement roadmap](docs/optimization-path-after-round18.zh-CN.md).
Historical records saying “not published” describe their round's completion;
this later commit publishes the reviewed source, not their local build artifacts.

## Origin, licences and publication / 来源、许可与公开清理

The baseline remains [ZacharyZhang-NY/ElyOffice](https://git.zacharyzhang.com/ZacharyZhang-NY/ElyOffice)
at `0d991e1c715859abd1282981fd46989e7e2319c7`. Original authorship, the root
AGPL-3.0-or-later licence, the imported MIT notice and third-party resource
licences remain. The 72 LibreOffice patches are preserved. This is not an
official upstream release or a full upstream-version upgrade.

This descendant commit preserves GitHub history. The export excludes local
Git metadata, `.work`, downloaded engines/reference trees, build outputs,
caches, personal documents, raw diagnostics and internal planning files.
Referenced review reports are explicitly copied to `docs/reviews` and their
documentation links updated. Text line endings may be normalized. Production
implementation and Office regression assertions receive no publication-specific
edits. At the owner's request, remaining credential-shaped gallery and component-test
samples are replaced with unmistakably invalid demo placeholders. Matching test
expectations are updated consistently; test control flow is unchanged. The
eight-character password boundary and six-digit code cases keep their original
length/character-class conditions. These fixtures do not authenticate to services. The original local working tree, staging area and upstream
remote are left unchanged.

原作者署名、AGPL根许可、已引入代码的MIT声明、第三方资源许可和72份内核补丁均
保留。清理后的源码排除构建缓存、引擎下载、个人资料、原始日志和内部进度文件；
公开所需的审查报告单独保留并修复链接。按用户要求，剩余具有凭据外形的组件展示及单元测试
样例统一替换为无效占位文字，相应测试期望同步替换，测试分支和边界条件不变。
办公程序及其回归断言保持；组件样例清理未重跑整套测试。本地工作区和暂存内容不改写。

Limited credential/path-pattern checks are not a comprehensive security audit.
Existing public self-signed test fixtures retain their original bytes.
They are for automated testing and must never be used as production credentials.
Bundled component fonts and example images are upstream assets, not personal
documents. Installer publisher IDs, signing and inherited upload scripts still
require review before any binary release; do not run upstream publishing jobs.
