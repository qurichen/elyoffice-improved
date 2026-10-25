# ElyOffice Improved

ElyOffice 的实验性改进分支 / An experimental ElyOffice derivative.

## 为什么关注这个项目 / Why this project

- **开源、可审查、可定制**：遵循 AGPL-3.0-or-later，可研究、修改并按许可要求再分发。
- **离线优先、文件由你掌握**：核心本地文档处理不依赖云端账号，不要求把正文交给云端服务。
- **面向常用办公格式**：文字、表格和演示使用 LibreOfficeKit，支持 DOCX/XLSX/PPTX 与 ODF；支持格式不等于完整兼容所有功能。
- **原生桌面界面**：基于 Rust 与 GPUI，可从源码调整界面和本地工作流。
- **改进有据可查**：保留补丁、合成测试样本和验证范围，重点修复 Windows 运行、文档往返及保存恢复问题。

- **Open and adaptable**: inspect, modify and redistribute under AGPL-3.0-or-later and applicable third-party licences.
- **Offline-first, user-controlled files**: core local document workflows need no cloud account and do not require sending document contents to a cloud service.
- **Familiar office formats**: LibreOfficeKit powers DOCX/XLSX/PPTX and ODF workflows; format support does not mean complete feature compatibility.
- **Native desktop UI**: Rust and GPUI provide a source-customizable desktop interface and local workflow.
- **Evidence-backed improvements**: patches, synthetic fixtures and scoped validation document Windows, document-roundtrip and save/recovery repairs.

目前为源码预览，不是安装包，也不保证完全替代 Microsoft Office。
尚未证明比其他办公软件更快或更省内存；完整交互、恢复和安装验收仍未完成。
This is a source preview, not an installer or a claim of full Microsoft Office parity.
No speed or memory advantage is claimed; complete interactive, recovery and installer validation remains open.

发布范围、清理事项与原作者信息 / Publication scope, hygiene and attribution:
[PUBLICATION.md](PUBLICATION.md).


## 最新更新 / Latest update — 2026-10-25

已整合截至第17轮的累计改进：72项内核补丁、保存恢复保护、Windows高缩放与
深层资料目录修复、上游办公核心补全，以及原文档/选区绑定的安全命令调度。
第17轮224项测试通过，三应用及工作进程优化构建、表格/演示原生保存重开通过，
独立审查R1通过。应用和工作进程需配套更新。

Cumulative improvements through round 17 include 72 engine patches, safer
save/recovery, Windows DPI and deep-profile repairs, selected upstream office-core
backports, and commands bound to their original document/context. Round 17 passed
224 distinct tests, optimized app/worker builds, bounded Table/Show native save
and reopen checks, and independent R1 review. Update apps and worker together.

本次是源码更新，暂不提供安装包。已知限制及关闭窗口后的保留诊断见
[发布说明 / Publication notes](PUBLICATION.md) 和
[第17轮验证 / Round 17 validation](docs/windows-validation-round17.md)。
This is a source update; no installer is provided. Known limits and retained
window-close diagnostics are documented in the linked records.

---

# Ely Office

This working branch improves Windows build reliability and OOXML fidelity.
It is based on [ZacharyZhang-NY/ElyOffice](https://git.zacharyzhang.com/ZacharyZhang-NY/ElyOffice)
at `0d991e1c715859abd1282981fd46989e7e2319c7`, not a new implementation
or an official upstream release. Original authorship and licences are retained.

The native Windows engine now builds with 72 patches, including a Windows LOK
logical-DPI correction for enlarged and cropped tiles on high-scaling displays.
The earlier 93 selected client/worker regressions passed against the 63-patch
engine; these are historical results, not the full workspace suite.
Supported legacy EQ fractions retain editable field semantics,
and the complex native Word sample's eight following paragraphs now match the
original positions after saving and reopening. Modern chart point-color defaults,
Windows save/backup DACLs and encrypted recovery profile scoping are repaired.
Session-only encrypted recovery has a one-time user warning. Doc/Table/Show
GUI binaries also rebuild successfully. Complete interactive/recovery and
installer checks remain open; native Office still shows other layout differences.
See [round 8 validation](docs/windows-validation-20261020.md) and
[round 9 GUI/profile validation](docs/windows-validation-round9.md).
Historical compatibility scope is in [round 10 validation](docs/windows-validation-round10.md)
and [round 10 implementation notes](docs/ooxml-fidelity-round10.md).
Later [round 12 validation](docs/windows-validation-round12.md) records the complete
Doc/Table/Show application runs and bounded native Doc save/recovery checks.
[Round 13 validation](docs/windows-validation-round13.md) adds repeatable native
Table/Show save-and-reopen trials, strict content-reader controls and final
test-process cleanup review. [Round 14 validation](docs/windows-validation-round14.md)
fixes the reproduced duplicate-DPI rendering defect at 175% scaling and reruns
all 209 Doc/Table/Show application tests plus three new image regressions.
Other DPI configurations, general visual fidelity, broader long-path coverage, encrypted
native recovery and installer acceptance remain open; these are not general
Office-parity or long-term stability certifications.
See [repair evidence](docs/ooxml-fidelity-fixes.md),
[Windows build notes](docs/windows-build.md) and
[publication gates](docs/github-publishing.md).

[Upstream integration batch 1](docs/upstream-integration-batch1.md) selectively
backports office-core fixes from the current GitHub upstream, retaining our
original 64 patches and local save/recovery protections. The combined build
passes 284 focused/application tests and native Table/Show save-and-reopen
trials. This is not the whole upstream0.1.11, an installed update, or a release;
new components/Mail remain open; at that snapshot the separate deep-profile
startup repair was still pending.

[Round 15](docs/windows-validation-round15.md) fixes the reproduced Windows
deep-profile startup failure by correcting directory-mask separators in SAL.
Requested profile locations are preserved; this is not a system-policy change
or unrestricted long-path/UNC support. It follows the integration snapshot above.

[Round 16](docs/windows-validation-round16.md) completes Table's merge-content
check using the existing kernel count. Empty ranges can merge, and formulas
that display an empty result still require confirmation alongside other data.
The complete Table suite passes 58 tests, including four new merge regressions.

[Round 17 research and architecture](docs/office-pain-points-architecture.md)
maps primary Office feedback and extension designs to Ely's command layer.
Client and view dispatch are separated; read-derived operations check their
document and revision before running, including merge confirmation after a
selection or sheet change. Current-selection input keeps its ordered queue.
Explicit slide notes keep their original page across a plain page switch,
while edits and page reordering invalidate the old request. Apps and the worker
must be updated together for the new selection-context check.
See the [round 17 validation record](docs/windows-validation-round17.md) for
the tested scope and remaining extension, undo, performance and layout work.

Offline-first desktop apps in Rust and GPUI. Files stay in your folders; no account, no cloud service.

| App | Does | Engine |
| --- | --- | --- |
| ElyDoc | Word processing (DOCX, ODT) | LibreOfficeKit |
| ElyTable | Spreadsheets (XLSX, ODS, CSV) | LibreOfficeKit |
| ElyShow | Presentations (PPTX, ODP) | LibreOfficeKit, system media |
| ElyPDF | PDF reading and editing, OCR, signatures | MuPDF, Tesseract |
| ElyTree | Git client | Git |
| ElyMark | Markdown | markdown-rs, RaTeX, mermaid-rs-renderer |
| ElyMail, ElyAgent | Second release | — |

## Build (macOS)

```sh
scripts/doctor.sh                 # toolchain versions
scripts/fetch-references.sh       # upstream sources at pinned commits
scripts/build-libreoffice.sh      # patched LibreOffice 26.2.6.3 (about an hour)
scripts/build-mupdf.sh            # patched MuPDF 1.28.5
scripts/build-git.sh && scripts/fetch-git-lfs.sh
scripts/fetch-tessdata.sh
cargo build -p ely-lok --bins -p ely-pdf-engine --bins
cargo run -p ely-doc              # or ely-table, ely-show, ely-pdf, ely-tree, ely-mark
```

Upstream packaging: `packaging/macos/release.sh` writes each app's `.app` and `.dmg`, the source archive and the upstream source archive (patched MuPDF, Git) to `dist/`. Linux and Windows recipes are in `packaging/linux` and `packaging/windows`. This branch has not validated a complete application package on any platform.

## Tests

`cargo test --workspace`, the window scripts in `tests/scripts` (macOS, Accessibility permission), and the benchmark `tests/perf/bench.py`.

## Documents

| | |
| --- | --- |
| `docs/help.md` | Help: opening and saving, recovery, offline limits, repositories |
| `docs/compatibility.md` | Versioned compatibility list |
| `docs/accessibility.md` | Accessibility and language review |
| `docs/perf/` | Benchmark records |
| `docs/licenses.md` | Licences and sources |

## Licence

AGPL-3.0-or-later (`LICENSE`, matching the workspace licence field).
Third-party components keep their own licences; see `docs/licenses.md`.
