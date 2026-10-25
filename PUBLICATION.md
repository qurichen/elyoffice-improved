# Experimental source preview / 实验性源码预览

Updated 2026-10-25: cumulative ElyOffice improvements through round 17,
following the round-10 import. This is a source update, not an upstream
release, installer or claim of complete Microsoft Office compatibility.

2026-10-25更新：在第10轮公开版本上整合截至第17轮的累计源码改进。
本次发布源码，不发布安装包，也不代表原作者的官方版本或全面兼容微软Office。

## Changes since round 10 / 第10轮之后的更新

- Safer recovery validation, no-overwrite restoration and scratch-file ownership;
  shared test-process cleanup and repeatable Windows native save/reopen checks.
- Windows logical-DPI rendering and the reproduced deep-profile startup repair.
- Selective upstream office-core backports, retaining this fork's save/recovery
  safeguards, OOXML work and separate update channel.
- Table merge decisions include empty-result formulas and remain bound to the
  original document/selection. Delayed commands reject stale or foreign targets.
- Shared command dispatch is separated from lifecycle and painting. Explicit
  slide notes retain their original page across plain page switches, while
  edits or page reordering invalidate old requests. Apps and worker must update
  together for the added selection-context protocol check.

- 加强恢复校验、无覆盖恢复及临时文件归属保护，完善测试退出回收和可重复的
  Windows原生保存、重开验收。
- 修复已复现的高缩放重复DPI渲染和深层资料目录启动问题。
- 选择性整合上游办公核心更新，保留本分支的保存恢复、OOXML修复和独立更新源。
- 修复空结果公式的合并确认，并将确认绑定原文档、原选区，拒绝过期或跨文档命令。
- 拆分共用命令调度；翻页后备注仍写回原页，内容或页序改变则拒绝旧请求。
  新增上下文检查协议要求应用和工作进程配套更新。

## Evidence and limits / 验证与限制

The engine patch series now contains 72 patches, compared with 63 in the
previous public snapshot. The latest round passed 224 distinct tests in eight
groups, rebuilt the three office apps and worker in optimized mode, and passed
the bounded Table/Show native save-and-reopen checks. No test was skipped or
force-cleaned in those accepted groups. Earlier rounds retain their own results;
historical counts are not added together as a current test total.

补丁序列从此前公开的63项增加到72项。第17轮最终8组224项不同测试通过，
三应用与工作进程优化构建完成，表格和演示的限定原生保存、重开流程通过。
验收组无跳过或强制清理；旧轮次结果分别保留，不累加为当前测试总数。

See [round 17 validation](docs/windows-validation-round17.md),
[the independent review](docs/reviews/round17-command-r1.md),
[upstream integration](docs/upstream-integration-batch1.md), and
[Office pain points and architecture](docs/office-pain-points-architecture.md).
The Table native log retains an invalid-window-handle message after closing a
window; its cause is not claimed fixed. General visual fidelity, system IME,
other DPI configurations, printing, encrypted native recovery, multi-day use
and installer/upgrade/uninstall acceptance remain open. Ordered batches are
not atomic transactions. No extension SDK or performance advantage is claimed.

表格关闭窗口后的一条句柄报错保留为待调查诊断，不声称根因已修复。
复杂文档视觉、系统输入法、更多DPI、打印、原生加密恢复、多日使用和安装升级
仍有待验收；顺序批次不等于原子事务，也未提供完整插件SDK或性能优势承诺。

## Origin and licence / 来源与许可

- Original baseline: [ZacharyZhang-NY/ElyOffice](https://git.zacharyzhang.com/ZacharyZhang-NY/ElyOffice),
  `0d991e1c715859abd1282981fd46989e7e2319c7` (0.1.8).
- Selected newer office-core sources are identified in the integration record;
  this is not a full upstream 0.1.11 upgrade.
- Original attribution and root AGPL-3.0-or-later remain. Imported MIT notices
  are retained in `LICENSES/elyoffice-upstream-MIT.txt`; third-party engines
  and resources keep their respective licences.
- 保留原作者署名、根AGPL许可、选择性引入代码的MIT声明及第三方许可。

## Publication hygiene / 公开文件清理

This update preserves the existing GitHub history and adds a normal descendant
commit. It exports the current working source rather than the old staged index,
leaving the local source tree and staging area intact. Git text line endings
are normalized where needed; code, test assertions and engine patches receive
no publication-specific semantic edits.

The source export excludes local `.git`/`.work`, engines/reference downloads,
build outputs, caches, Office lock files, internal planning/progress records and
the inherited runner workflow. The bilingual README and this document describe
the current scope; the publication checklist is brought up to date. The final
round-17 review is copied to `docs/reviews/`, and its validation link is adjusted.
Raw local build/test logs and local Git history are not uploaded.

延续上次公开清理规则，排除构建缓存、资料、临时锁文件、内部计划和上游机器
发布流程。首页和本文更新双语说明，发布清单校正现状，最终审查报告单独公开。
本地源码、暂存内容及原作者远程配置保持原样，公开仓库保留既有提交历史。

Limited credential/privacy pattern checks and per-file Git blob/mode verification
are not a comprehensive security audit. Existing public self-signed signing
fixtures remain test-only: `CN=Ely Test Signer, O=Ely Office`, password `ely`.
Their bytes are unchanged from the previous public import; never reuse them
as production credentials. No binaries, tags or GitHub Release are published
by this source update.
