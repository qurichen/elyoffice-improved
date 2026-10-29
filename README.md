# ElyOffice Improved / 离线办公改进分支

Experimental source fork of ElyOffice, preserving the original authorship and
AGPL-3.0-or-later licence. This is not an official upstream release.
保留原作者署名与AGPL许可的实验性源码改进分支，不是原作者的官方发行版。

## Latest source update / 最新源码 — 2026-10-29

Includes work through round 26 and the latest source audit: safer Windows file
opening and attachment names, macro detection, bounded logs and fewer event
copies; recovery/lifecycle protections; text-preserving CSV import; opt-in
selected-text translation with user-configured model connections.

已同步至第26轮及最新源码审计：修复特殊文件名、附件边界和宏漏检，减少日志和
事件处理的内存浪费，完善保存恢复与关闭保护，增加CSV按文本保留和默认关闭的
选区翻译。文件仍由本地文档引擎处理；翻译只有明确请求时才发送预览选区。

Latest audit: **501 distinct Rust tests + 15 website tests passed**, with
independent R1 review. Full-workspace checking remains blocked by missing MuPDF
build libraries; the other 12 first-party members passed all-targets checking.
No installer is provided. Real-model translation quality and general Office
compatibility are not established by fixture tests.

最近审计501项Rust和15项官网测试通过，独立R1审查通过。全工作区检查受MuPDF
编译依赖缺失限制，其余12个自有成员检查通过；本次不提供安装包，不把测试服务
结果写成真实模型质量或全面Office兼容性承诺。

See [publication notes / 发布说明](PUBLICATION.md),
[audit report / 审计记录](docs/source-audit-20261028.zh-CN.md), and
[roadmap / 改进路线](docs/optimization-path-after-round18.zh-CN.md).

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

[Round 18](docs/windows-validation-round18.md) carries the original read state
through Table's sort, standard filter, text-to-columns and column-button menus.
Selection or sheet changes invalidate old confirmations. A shared explicit-read
batch keeps column-menu positioning and mutation together; it does not provide
atomic rollback. Filter menus keep their first client request context while
waiting for the paired pointer release; switching sheets during that wait
invalidates the request. Slow and quick clicks retain their original target.
This round's validation and retained diagnostics are recorded separately from
earlier test totals.

[Round 26](docs/windows-validation-round26.md) adds opt-in English/Chinese
selection translation in ElyDoc. Users configure a Chat Completions URL, model
and optional system-stored API key. Only the previewed selection is sent on
request; results can be previewed and copied. Local endpoints may relay to cloud
services. This is independent of the unfinished ElyAgent; it does not edit the
document. Protocol and UI fixtures do not establish real-model translation quality.

[Round 25](docs/windows-validation-round25.md) adds an optional **Keep all fields
as text** CSV import mode. It preserves numeric and date-like spellings across
import and save, covers columns discovered beyond the preview, and restores prior
column choices when switched off. Ordinary numeric import remains the default.
Per-column conversion differences and risk previews remain roadmap work.

[Round 24](docs/windows-validation-round24.md) fixes the Windows native validation
reader when a growing log produces multiple raw output parts. Parts are joined
without inserted characters, while truncation and changes to the previous log
prefix remain errors. This updates the verification tools; application binaries
are reused. The earlier native failure remains recorded without a retrospective
claim that its unique cause is proven.

[Round 23](docs/windows-validation-round23.md) corrects Table's default CSV
delimiter inference for quoted fields, multiline records and encoded text.
Quoted punctuation is excluded in later columns as well as the first; TSV file
extensions keep their existing meaning. Import previews remain available for
ambiguous files; this is not a strict no-conversion import mode.

[Round 22](docs/windows-validation-round22.md) addresses a reproduced text-input
abort caused by using an old layout to position the caret in replacement text.
Character hits now require the text and masking state that were actually shaped;
drag and Markdown consumers preserve that validity check. Existing pre-frame IME
candidate positioning remains available. The pinned component source, licenses,
focused regressions, and a portable upstream patch are included in the repository.

[Round 21](docs/windows-validation-round21.md) keeps recovery-point order across
equal or backward clocks, avoids reusing unrelated metadata, and coordinates
point writes with maintenance and discard. On clock correction the point time
can be a logical ordering time; the last-success time remains the supplied wall time.
Its native validation retains the text-input abort before the planned crash
injection. Round 22 addresses the deterministically reproduced layout mismatch;
the initiator of the earlier Minimize event remains unidentified.

[Round 20](docs/windows-validation-round20.md) fences recovery-index writes during
shutdown and keeps the crash marker when the final index cannot be committed.
Closing a LOK view also cancels its event watch before its document is released.
Validation separates recovery storage and process-termination tests, Windows save
failure controls, application regressions, and native-window recovery evidence.

[Round 19](docs/windows-validation-round19.md) synchronizes native Windows window
retirement with the refresh registry before destruction completes. Redraw errors
remain visible; the scope and native before/after evidence are recorded separately.

The [post-round-18 optimization path](docs/optimization-path-after-round18.zh-CN.md)
maps seven office projects to proposed work and acceptance gates. It is a roadmap,
not a record of completed integrations or a new release.

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
