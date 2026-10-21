# OOXML fidelity and instance startup: candidate fixes

Base: `0d991e1c715859abd1282981fd46989e7e2319c7` (ElyOffice 0.1.8).
Local branch: `fix/ooxml-fidelity-windows-startup`.

These are source changes, not a rebuilt release. No changes have been published.

## Changes

### Instance publication

`instance::door` no longer renames `instance.door.new`. The holder writes the
small discovery record directly under the existing exclusive instance lock.
The reader already retries file, parse, connect and authentication failures
before delivering any paths. Delivery itself is never retried. This avoids
same-directory rename failures seen in a managed Windows profile without
allowing a partially written record to deliver paths.

Tests cover an incomplete record, a longer stale record, an unusable old
`.new` path, concurrent holders, wrong tokens and stalled clients. Unix also
restores mode 0600 on an existing discovery record; that Unix test was not run
on this Windows host. Filesystem errors still propagate; no copy fallback or
suppression of write errors was added.

The AppId packaging assertion now accepts both LF and CRLF checkouts.

### DrawingML hyperlink color

Patch `0052-drawingml-hyperlink-colors.patch` changes LibreOffice import and
export. Import interprets `ahyp:hlinkClr` by its namespace and `val`, rather
than treating any `a:extLst` as a request for text color. Only `tx` selects
the run color; `hlink`, an absent mode, and unrelated extensions select the
theme hyperlink color. Both Transitional and Strict namespace tables and
XML tokens are updated.

PPTX URL text fields export `hlinkClr=tx` so PowerPoint uses their actual
model color. A theme-derived color remains `schemeClr=hlink`; the regression
reader explicitly rejects flattening it to an RGB blue. Fixed black/red
cases must retain `srgbClr`. No original document XML is restored over edits.

The extension's meaning and values are defined in Microsoft's
[hyperlink color specification](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-odrawxml/014fbc20-3705-4812-b8cd-93f5af05b504).

## Regression fixture

`tests/fixtures/office/hyperlink-colors.pptx` is synthetic and contains no user
documents or personal metadata. Its five text runs are:

| Marker | Mode | Expected appearance |
| --- | --- | --- |
| KEEP_BLACK | tx | Black |
| KEEP_RED | tx | Red |
| THEME_BLUE | Absent | Theme hyperlink blue |
| HLINK_MODE | hlink | Theme hyperlink blue |
| UNKNOWN_EXTENSION | Unrelated extension | Theme hyperlink blue |

`client/tests/hyperlink.rs` loads it, saves flat ODF and PPTX, closes and
reopens the PPTX, and saves another flat ODF. The independent Python reader
checks all five imported colors, exported colors/modes and reopened colors.
The fixture can be regenerated with `hyperlink-fixture.cjs` followed by
`hyperlink-fixture.py`. Generation requires pptxgenjs and defusedxml; reading
the regression results uses only the Python standard library.

## First-round verification

| Check | Result |
| --- | --- |
| Actual instance/AppId/dirs/env source, isolated Windows Rust harness | 18 passed, 0 failed |
| Actual LOK client and new Rust regression test, isolated manifest | Compiled successfully |
| New regression against unmodified official 0.1.8 worker | Fails for the expected HLINK_MODE and UNKNOWN_EXTENSION color defects |
| All 52 LibreOffice patches, pinned upstream source and original LF patches | Applied successfully in order |
| Upstream namespaces.py using both patched namespace tables | Generated successfully, including ahyp |
| Fixture structure validator | Passed |
| Microsoft PowerPoint read-only rendering of five-case fixture | All five match the stated colors |
| Serialization-only experiment on the earlier four-slide roundtrip | Restoring only the tx marker restores the black link in PowerPoint |
| Four-slide experiment structure and visual QA | Passed; no other visible slide changes |
| Comment rules; source whitespace outside nested patch context | Passed |
| Rebuilt C++ LibreOffice engine / complete GUI / installer | NOT performed |

The serialization experiment verifies the format-level cause and proposed
output, not execution of the patched C++ exporter. The full Rust workspace
was not built; isolated manifests avoid GPUI and platform SDK dependencies.
The tests used Rust 1.96.0 with incremental caches disabled.

## Required before merging or releasing

Build the pinned LibreOffice engine with all current patches using the project's
normal build environment. Build the worker and run the regression against
that newly built engine, not the stock 0.1.8 release:

```text
cargo test -p ely-platform instance::
cargo test -p ely-platform app_id::
cargo build -p ely-lok --bins
cargo test -p ely-lok hyperlink_colors_keep_text_and_theme_modes -- --nocapture
cargo test -p ely-lok manual_row_heights_keep_fractional_points_and_hidden_state -- --nocapture
cargo test -p ely-lok word_character_compression_survives_import_and_export -- --nocapture
cargo test -p ely-lok chart_placement_keeps_page_cell_and_resize_modes -- --nocapture
cargo test -p ely-lok theme_fonts_keep_major_and_minor_script_mappings -- --nocapture
python tests/scripts/fidelity-readers-test.py
python tests/scripts/chart-anchor-reader-test.py
python tests/scripts/theme-font-reader-test.py
```

Use `ELY_LOK_DIR` for the new engine when needed. Python must be on PATH.
Also run existing LOK tests, real GUI single-instance checks, Windows
installer checks and native PowerPoint roundtrips. Test internal slide links,
presentation actions, theme changes and edited hyperlink colors before
claiming those paths are verified. Unix permission tests need a Unix host.

## Remaining layout work

Word line wrapping/default style changes and Excel chart/column layout
differences from the earlier complex-document test are not fixed here.
Native Excel measurement found unchanged print area `$A$1:$M$18`, but default
columns changed from 8.54 to 8.00 character units, and the serialized default
row height changed from 14.15 to 13.5 points. Chart position/size and title
font also changed. These require font metrics, style inheritance and anchor
analysis in the engine; global font substitutions would risk other files.

No claim is made of general Microsoft Office layout equivalence.

## Second round: manual Excel row heights

Patch `0053-calc-preserve-manual-row-heights.patch` excludes explicit manual
row heights and manual default heights from the Microsoft-document-only
0.75-point flooring path. Automatic heights retain the previous behavior.
The existing point-to-twip rounding remains unchanged.

The synthetic workbook contains seven manual rows, a hidden row, a custom
default height with an explicit override, and nine calculated formulas.
An independent reader verifies dimensions, flags, formulas and value caches.
The Rust test checks the first save and a second save after reopening.

The official worker changed manual heights `23.2, 0.5, 12.75, 14.2, 29.8,
409.0, 23.2` into `22.5, 15, 12.75, 13.5, 29.25, 408.75, 22.5` points.
Changing only the workbook's Application metadata to a non-Microsoft name
avoided that branch and preserved every explicit height. All other archive
entries in the input pair are identical. Excel opened and recalculated all
nine formulas correctly; the control retained the original manual row sizes
under Excel's own display rounding. This is branch-control evidence from
the old engine, not execution of the new C++ patch.

Excel did not visibly apply the custom default height to automatic rows in
this fixture; the default-height check demonstrates model preservation only.
The patch does not fix default column widths or chart anchors. Excel defines
row heights in [points](https://learn.microsoft.com/en-us/office/vba/api/excel.range.rowheight).

## Second round: Word character compression

Patch `0054-writer-character-compression-roundtrip.patch` maps the DOCX
`characterSpacingControl` string onto the existing document setting and
exports the actual edited model value. It supports `doNotCompress`,
`compressPunctuation` and `compressPunctuationAndJapaneseKana`. An absent
value does not modify the existing default or the shared RTF import path.
Import reads a string because the pinned writerfilter schema defines this
resource as a string, not an integer enumeration.

The four fixtures cover all three values and omission. The independent
reader checks the actual flat-ODF model before saving, the DOCX setting,
the body marker, and the reopened model. The official worker imports both
enabled compression modes as zero and drops the DOCX setting. None and
omission remain semantically correct in the stock engine.

Microsoft's [OOXML setting reference](https://learn.microsoft.com/th-th/dotnet/api/documentformat.openxml.wordprocessing.characterspacingcontrol?view=openxml-2.12.0)
defines the modes and omitted default. The existing LibreOffice
[model constants](https://api.libreoffice.org/docs/idl/ref/CharacterCompressionType_8idl_source.html)
are zero, one and two respectively.

Native Word reported mode one for the original punctuation-only fixture
and zero after the stock roundtrip. It reported zero for the original kana
fixture too; setting its VBA property to two also read back as zero on this
host. Kana-mode behavior therefore remains unverified in native Word here.
All eight native PDF renders were pixel-identical within their original /
roundtrip pairs. No visible wrapping improvement is claimed from this sample.
The earlier complex document already used `doNotCompress`, so this patch
does not explain or fix its remaining vertical-spacing differences.

## Second-round verification and limits

- All 54 patches applied in order to raw LF files from the pinned engine.
- Four Word fixtures passed the document skill's structural validator.
- Four independent-reader unit tests cover 16 normal / mutation cases.
- Review caught and fixed a missing-row false pass in the row-height reader.
- Actual stock-worker roundtrips reproduce both new model-preservation bugs.
- Native Excel checks include recalculation, hidden state and manual heights.
- Native Word PDF pages were rendered and visually inspected.
- Comment rules and whitespace outside nested patch contexts passed.

The new Rust fidelity test module has not been compiled in this second round.
The earlier hyperlink module's isolated compilation does not validate it.
No C++ engine, GUI or installer was rebuilt in either round. Build all 54
patches and run the new tests before merging or distributing binaries.

## Third round: chart attachment behavior

Patch `0055-calc-chart-placement-modes.patch` replaces the constant
`editAs="oneCell"` in chart export with the current drawing object's anchor
type. Cell-fixed charts use `oneCell`, resizing charts use `twoCell`, and
page-fixed charts use `absolute`. Missing objects or invalid types raise an
error. Existing from/to geometry is unchanged; original XML is not restored.

The synthetic workbook covers six encodings of the three modes: one-cell,
two-cell explicit resize, two-cell omitted default, absolute, two-cell
absolute override, and two-cell fixed-size override. The reader checks the
imported and reopened flat-ODF attachment, the saved DrawingML behavior,
source values, six SUM formulas and caches, and chart series caches. Workbook,
drawing and chart parts are followed through their actual relationships.

The stock engine imports the six modes correctly, then exports all six as
move-only charts. Four cases consequently change behavior. Changing only the
six exported editAs values restores native Excel Placement values
`2, 1, 1, 3, 3, 2`. Native Excel also confirms the behavior when row/column
sizes outside and inside the chart's anchor are changed: resizing charts
resize, fixed-size cell charts only move, and page-fixed charts do neither.
Values and formula results remain correct in all three copies. This is an
export-serialization control, not execution of the patched C++ function.

Microsoft defines these behaviors in the [DrawingML mode reference](https://learn.microsoft.com/ko-kr/dotnet/api/documentformat.openxml.drawing.spreadsheet.editasvalues?view=openxml-2.11.1)
and the [native Excel enumeration](https://learn.microsoft.com/en-us/office/vba/api/excel.xlplacement).
The patch does not correct the earlier complex chart's coordinate, font or
size differences; its original placement was already move-only.

## Third round: supplemental theme fonts

Patch `0056-theme-supplemental-fonts.patch` exports the existing model's
major and minor supplemental font lists after latin/ea/cs. Import already
populates these lists; the shared ThemeExport previously omitted all of
them. The new loops preserve the actual current model, including edits and
arbitrary script names. They do not reinsert original theme XML.

The DOCX regression uses eight script mappings in each collection: Hans,
Hant, Jpan, Hang, Arab, Hebr, Thai and Deva. Major/minor fonts differ so
swapping lists is detected. The independent reader follows the document's
theme relationship, checks all sixteen mappings and multilingual paragraphs,
and allows equivalent run splitting. Rust checks the first save and the
second save after reopening. Microsoft documents the [script mapping](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.supplementalfont.script?view=openxml-3.0.1).

Source inspection confirms the same ThemeExport is called by Writer's DOCX
export, Calc's workbook export and Impress's per-master PPTX theme export.
The new executable regression covers DOCX only; rebuilt XLSX/PPTX theme
roundtrips and master/theme-override cases are still required before release.

The stock engine drops all sixteen mappings on both saves. A control that
adds only the missing mappings passes the reader and Word's structural and
native rendering checks. The complex sample's paragraph 1-to-8 vertical span
is 234.85 points originally, 230.10 after the stock save, and 234.85 in the
mapping-only control. Its first paragraph is still about 6.85 points earlier
than the original; headings, object placement and other spacing still need
work. All three versions remain three pages. This demonstrates a partial
format-level cause, not a completed engine or general layout equivalence.

Other one-factor Word controls for kerning, explicit East Asian font name,
grid defaults, inherited paragraph spacing, auto-hyphenation and useFELayout
did not restore those paragraph advances. A whole-settings transplant was
rejected by Word and is not used as a fix or delivered as a document.

## Third-round verification and limits

- All 56 engine patches applied in order to the fixed upstream revision.
- Ten new reader test methods cover 32 normal / mutation scenarios.
- Review caught and fixed source-cell deletion passing stale caches.
- Two independent review rounds have no outstanding explicit findings.
- New Word fixture, stock/control saves and complex control passed structure checks.
- All twelve native Word PDF pages and six Excel fixture pages were inspected.
- Native Excel recalculated all six fixture formulas to six.
- Native behavior and format controls are recorded separately from source tests.

The two new C++ patches and new Rust test module are still not compiled.
Build all 56 patches and execute the regressions before merging or releasing.
Do not use these serialization controls as evidence of a rebuilt application.

## Fourth round: cross-format theme regressions

Supplemental-font roundtrips now have separate DOCX, XLSX and PPTX Rust
tests. The two new fixtures change only the existing chart workbook's and
hyperlink presentation's theme part. The independent reader follows workbook
relationships and every presentation-referenced slide master's theme; it
rejects external targets, missing relationships and lost mappings. Eleven
reader test methods include a second referenced master and relocated parts.
The multi-master case is a reader unit test, not an engine roundtrip fixture.
Theme overrides still need dedicated rebuilt-engine coverage.

Native Excel opened all six sheets, recalculated their SUM formulas to six,
and retained source values, chart series and original placement modes.
Native PowerPoint opened the one-slide fixture with all five text markers.
Structure checks and exported images were inspected. These are fixture
checks, not output produced by the new C++ patch.

## Fourth round: actual Rust compilation and stock failures

A portable Rust 1.96.0 GNU toolchain compiled the actual LOK library's unit
test code, including all 211 listed tests. The isolated manifest points
directly at the repository library and real test modules; it excludes the
optional GPUI view and GUI development dependencies. Separate integration
wrappers compiled the actual fidelity and hyperlink modules too. This does
not establish a full-workspace or GUI build.

Six fidelity tests and one hyperlink test were executed against the
unmodified official 0.1.8 worker, both through the library and integration
wrappers. All seven reached their intended defect assertions and failed;
none skipped or failed to start. The three theme formats all lose their
supplemental mappings. These negative controls establish detection of stock
defects, not success of the patched C++ code. The other 204 library tests
were compiled but not executed in this round.

## Fourth round: remaining Word equation offset

After the earlier theme-only control, paragraph positions agree through the
legacy EQ paragraph. The next paragraph starts at 186.85 points instead of
193.70. Restoring only the original math settings leaves the offset intact.
Restoring only the original EQ paragraph makes all eight following paragraph
positions match the original, while both controls remain three pages.

The remaining 6.85-point displacement is therefore localized to that EQ
conversion in this sample, not a uniform paragraph-spacing defect. The two
diagnostic controls passed structure checks and their page-two PDFs were
rendered and inspected. Existing tracked changes were not separately
attributed or validated. No constant spacing adjustment or original-XML
restoration is proposed as an implementation. Legacy-EQ/OMML layout and
default spreadsheet column-width differences remain unresolved.

## Current release gate

The pinned Windows engine with all 61 patches, the real worker and all three
office GUI binaries now build. Seven targeted fidelity and five worker stability
tests pass on that new engine without skips. Native Office comparisons remain
limited to synthetic fixtures and reveal unresolved layout differences.
Isolated native GUI launch/draw/close checks now pass for all three office apps.
Full editing/recovery, installer checks and distribution licensing remain
release gates. No installable replacement is delivered here. See
[round 8 validation](windows-validation-20261020.md) and
[round 9 validation](windows-validation-round9.md).

## Windows build preparation on 2026-10-20

This section records the earlier build preparation, not the final round-8 result.

The isolated build snapshot is at `E:\ElyOfficeBuild\elyoffice`, with tools,
downloads, caches and output on E. VS2022 Build Tools 17.14 is now registered
complete; an actual MSVC 19.44 C++ program compiled, linked against the
Windows SDK, and ran successfully. This smoke test is not the office engine.
The project-pinned Rust 1.96.0 MSVC toolchain is task-local too; Windows PATH
was not changed and Cargo incremental compilation is disabled.

`ELY_BUILD_JOBS` accepts positive decimal integers only. It controls both
top-level make and configure's external-build parallelism. Configure arguments
are forwarded as individual arguments before that explicit concurrency cap.
The first build uses four C++ jobs; the separate worker build uses two Rust
jobs. The patch-prefix helper now recognizes an unmodified tree first,
avoiding exhaustive reconstruction of every possible prefix on a new clone.
Existing prefix, repeat and unrelated-edit checks are preserved.

All 56 earlier patches were actually applied to the fresh E-drive engine
copy, and the dictionaries submodule is at its required `9925407` revision.
The first configure stopped because the upstream antivirus diagnostic
generated an EICAR test file, which Defender blocked. No protection settings
or exclusions were changed, and no quarantined file was restored.

Patch `0057-optional-antivirus-build-probe.patch` adds an explicit
`--disable-antivirus-check` configure option. The default remains enabled;
values other than yes/no are rejected. Only this upstream build-directory
probe can be skipped, not Windows protection. The current isolated build
passes that option, and real configure now completes. A read-only Defender
check still reports antivirus, real-time and behavioral protection enabled.

Four job/argument tests, three patch-prefix tests and four probe-control tests
passed. The last group runs autoconf on the real option/guard code but replaces
the probe body with a harmless marker: it does not exercise the original
antivirus probe or generate its test content. The actual full configure run
separately validates the explicit-skip path. Independent source review results
are recorded under `tasks/reviews/local-*-r*.md`.

The real-workspace Rust worker build completed successfully, without the
optional GUI view. Its binary is under `E:\ElyOfficeBuild\rust-target\debug`.
The C++ build has passed configuration, downloaded its pinned dependencies,
and produced actual C/C++ object files, but the full engine remains pending.
An active process or completed configure is not a successful engine build.
Use the per-attempt JSON/log files under the E-drive `logs` directory for the
current outcome. GUI compilation is deferred while available RAM is low.
New-kernel regressions, GUI/installer checks, native layout checks and safe
parallel trial remain release gates.

## Round 7: native build encoding and publication preparation

The preceding full build ended with Meson 1.8.3 attempting to decode the
Visual Studio activation output as GBK. Patch `0058` transfers the child
environment through a UTF-8 JSON file instead of decoding console output.
Batch contents are ASCII; Unicode paths travel through inherited variables,
with quoted expansion and CALL percent escaping. Command failure and bad
JSON remain errors; temporary files and control variables are cleaned up.

Eight tests passed, including actual VS activation with an MSVC/SDK
compile-link-run, actual Unicode/space/ampersand/percent/exclamation paths,
and a real activation failure preserving exit code 23. Independent review
found path defects in the first design and a test-wrapper encoding defect
in the second; these were fixed and R3 passed. The reviewer did not execute
the tests. The parent agent ran all eight without skips.

The full build then passed its old Meson blocker but stopped at XMLSec:
UTF-8 headers were interpreted using code page 936, producing C4819 and
fatal C2220 under the existing `/WX`. Patch `0059` adds `/utf-8` only to
the Windows XMLSec compiler flags. It does not suppress warnings. A real
MSVC control failed with explicit code page 936, and the fixed build
compiled, linked and ran with UTF-8 bytes intact. Both XMLSec tests passed;
independent static review R1 passed. All 59 patches were verified as the
current prefix before the next incremental native build started.

This iteration reran 41 test methods: eight Meson, two XMLSec, eleven
existing build-control checks and twenty fidelity-reader mutation checks.
The first reader invocation could not write to the sandbox temp directory;
it passed after assigning a task-local temp directory. The first comments
scan used a GBK default and failed reading UTF-8 source; running Python
with `-X utf8` passed. These are execution-environment corrections, not
new engine compatibility results. Initial failure logs were retained.

README now identifies the upstream author and baseline and agrees with
the existing AGPL-3.0-or-later workspace licence. Windows build notes and
a GitHub publication checklist distinguish a source preview from a tested
software release. The upstream Gitea workflow, publisher identifiers and
update service have not been repurposed or executed. No GitHub repository,
commit, tag, issue, pull request or release was created by this iteration.

At the end of round 7, full engine success, new-engine regressions, GUI/installer
tests, native layout checks and distribution licensing remained pending. Local
source candidate archives must retain these limitations; they are not an
installable replacement for the original app.

## Round 8: actual new-engine execution

The 59-patch engine completed its native build. Six fidelity tests passed, but
the hyperlink test exposed two real gaps that static token generation missed.
Patch `0060` registers the hyperlink-color namespace in XMLFilterBase's runtime
namespace whitelist. Import then passed, but export flattened theme blue to RGB.
Patch `0061` pairs the existing `setSchemeClr(XML_hlink)` with `setSchemeName`
so the complex-color model retains theme identity. Explicit tx colors are not
changed. Both fixes received independent static review before the final run.

The complete 61-patch pipeline exited zero, and the actual workspace worker
loaded that engine. All seven fidelity regressions passed, covering import,
save and reopen. Five additional worker error/session/cleanup tests passed.
The isolated non-GUI manifest uses the actual client library sources; it is
not a successful `cargo test --workspace` run. Thirteen synthetic roundtrips
were retained separately for inspection. Initial 59/60-patch failures remain
in the local evidence, not reclassified as passing results.

Word retained the sample's three pages and eight paragraph texts, but the
known 6.85-point EQ/OMML-related offset persists. Native Excel confirmed the
fractional row heights, hidden row, formula values and six chart attachment
modes. Default column width still changes from 8.54 to 8.00; chart appearance,
including colors, title, axes and legend, is not preserved by that check.
PowerPoint retained four slides and text; its rendered five-case hyperlink
colors match the source. Raw font-color properties were mixed, so actual
rendered pixels and visual inspection, not those properties, verified colors.

All three office GUI binaries compiled with the locked workspace dependencies
and one build job. They were not launched: current profile and instance paths
are shared with the installed app, so a safe isolated trial is still required.
No installed app, security setting or publishing endpoint was changed.

## Round 9: explicit profile isolation and native GUI trial

`ELY_DATA_DIR` now redirects the shared suite root, including settings,
recovery, worker profiles and instance discovery/locks. Unset preserves the
original location; empty, relative, root and parent-traversal paths fail
instead of silently using the installed profile. Paths retain OsString data.

Five directory tests and five real cross-process tests pass on the current
platform source. The process tests cover settings/lock placement, invalid
paths, recovery clearing within one profile and simultaneous independent
holders whose later copies hand off only within their own profile. Review R1
found a test-helper pipe lifetime defect; it was fixed and R2 passed. The
fresh final run includes the renamed root assertion, not a stale cached test.

The three profile-aware GUI binaries were rebuilt, hash-checked and launched
against disposable copies of the complex fixtures. Each drew a document tile,
produced a nonblank window capture and closed with exit 0 without forced
cleanup. All three captures were inspected. Sample hashes and the four checked
default settings locations were unchanged; no worker remained after exit.
This is launch/render/close evidence, not complete interactive editing QA.
See the round-9 validation record for remaining gates and test scope.
