# Windows validation: 2026-10-21, round 10

Experimental source candidate, not an installer or a production release.
Builds, native Office controls and temporary app profiles remain on E.
The installed applications, system credentials and publishing remotes were
not replaced. No commit, tag, repository or upload is created by this work.

## Current executions

| Scope | Result | Boundary |
| --- | --- | --- |
| Pinned C++ engine, patches 1-63 | Build and link exit 0 | Incremental Windows build, not a clean multi-platform build |
| Actual client/worker against new engine | 93/93 selected tests pass, no skips | Isolated non-GUI manifest using real client sources; not full workspace |
| ely-files | 110 unit tests pass; 2 child helpers ignored at top level | Includes existing tests, not 110 new tests; helpers invoked by parent cases |
| Windows DACL integration | 7/7 pass | Includes two null-DACL variants; no deny-all empty-DACL claim |
| Recovery and lifetime warning | 7/7 pass | Actual ely-office library, injected keys, no real vault access |
| Bilingual catalogue | 5/5 pass | Actual ely-ui library, no native warning-layout claim |
| Independent chart readers | 7/7 methods pass | Includes corruption controls; not a substitute for worker execution |
| Independent equation readers | 13/13 methods pass | Includes prefix/QName-preserving test generation |
| Doc/Table/Show binaries | Rebuilt, exit 0 | Test binaries, not installed packages |
| Native Doc/Table/Show windows | 3/3 open, draw and close normally | Fresh profiles and synthetic copies; no forced cleanup or keyboard/save-dialog claim |

The 93 distinct cases cover the five new equation/chart regressions, the
previous fidelity/hyperlink/Word checks, worker lifecycle, references, format
round trips, formula/workbook operations, layout, safe input and rendering,
pivots, chart edits, CSV/text import/export and document print settings.
System clipboard actions, actual printer jobs, macros, external network/file
links and an extreme-sheet resource-stress case were deliberately excluded.
The three new native window captures were visually inspected; their synthetic
input copies were unchanged. Encrypted documents were not used in this GUI
trial, so it does not exercise a native credential entry or the warning layout.

The first expanded run had two equation harness failures. Its ElementTree
rewrite renamed the `office:mimetype` attribute and removed namespace
declarations referenced by QName values. The pinned flat-XML detector needs
the literal mimetype prefix. Only the test generator was changed to preserve
the namespace spelling/declarations. The three equation tests then passed,
followed by a complete fresh 93-case run with every selected case passing.
Initial logs remain available; failed or repeated runs are not extra distinct
passed tests. Product patch 0063 was not changed to accommodate this harness bug.

## Native Office comparisons

Owned Word/Excel instances opened synthetic inputs read-only. Input hashes
were checked after use. Existing Office processes were not automated or
closed. Each retained engine case saved OOXML, closed/reopened it and saved
again, with flat-model exports for independent checks.

In native Word, the complex original, new saved DOCX and twice-saved DOCX are
all three pages. All eight tracked paragraph text digests match. Their Y
positions have a maximum absolute difference of 0.0 pt from the original
for both new files. The old 61-patch save differed by approximately
6.85-7.30 pt. The legacy field was regenerated from the current model, not
restored by replaying its original XML or inserting a fixed paragraph offset.

Native PDF pages were rendered and visually inspected. Landscape sections,
the selected formula/body positions and page counts agree in this sample.
The footnote separator length still differs. A separate equation stress
fixture also shows default paragraph-spacing and unsupported-radical format
differences. These results do not establish general or pixel-perfect Word
compatibility.

In native Excel, omitted point-color settings in the tested modern bar/pie
charts again produce the intended blue/red/green points. The old save made
them uniformly blue. Explicit false/true, purple fill and multiple-series
controls remain checked. Titles, axes, chart styles and default widths still
show differences; the point-color repair is not a full chart-style repair.

Nine explicit-font control workbooks, each with five sheets, were measured
before/after the old engine in Excel. The corrected measurements query the
actual Normal style and the Z1 font, not the first style in the collection.
The 45-sheet investigation exposes different font/DPI quantization. No
fixed-width compensation was added because it would regress other cases.

## Isolation, permissions and review

The previous default credential service/account and account-home lock path
are unchanged. Explicit profiles select a separate canonical-path-based
credential identity before backend access. Fake stores prove independent
keys/locks and invalid-profile zero backend calls. Windows case/verbatim and
junction aliases passed; the E test volume did not provide a distinct 8.3
alias. The Unix case test now follows actual filesystem case sensitivity;
no macOS/Linux runtime pass is claimed.

The Windows save regression was demonstrated before the fix: 3 of 6 selected
integration checks failed because protected/explicit access was lost. The
final protected-at-creation implementation passes the expanded seven-case
matrix, two private unit checks and the existing long-name tests. A stale
platform compilation was explicitly rebuilt; about 296 MiB of regenerable
platform artifacts were removed, not source documents. Long native paths and
the already-locked SHA-256 dependency version were corrected during testing.

Independent local reviews: chart R2, recovery-key/lifetime R2, DACL R3 and
equation R3 passed within their stated scopes. They distinguish static/evidence
review from an independent second execution. Review records are kept under
tasks/reviews/local-round10-*.md and excluded from public source archives.

Source archives retain upstream attribution and licences. They exclude local
profiles, credentials, logs, .git, .work, engines and reference trees. A ZIP
blob/checksum check and limited credential-marker scan do not constitute a
complete publication/security/licence audit. Consult the publication checklist.

## Still open

Full workspace tests; end-to-end native keyboard/mouse/save dialogs and
system IME; native encrypted crash recovery and warning layout; native
FAT/exFAT/network ACL cases; owner/SACL/alternate-stream preservation;
install/uninstall/rollback; other platforms; complete complex-document and
chart/default-width compatibility. Three round-9 application editing/save
logic tests remain historical evidence, not newly rerun round-10 GUI editing.
