# Windows validation: 2026-10-20, round 8

Historical snapshot. Later profile isolation and native GUI checks are recorded
in [round 9 validation](windows-validation-round9.md).

Experimental source candidate, not an installer or production release.
Upstream ElyOffice baseline: `0d991e1c715859abd1282981fd46989e7e2319c7`.
LibreOffice revision: `8221e31b3ac356a1623c672912a3d2b492f7e3d1`.
All 61 patches were present in the final successful build pipeline.

## Builds actually completed

| Component | Result and scope |
| --- | --- |
| Patched LibreOffice | Native Windows MSVC build, exit 0; final incremental pipeline completed at 22:08 BST |
| ely-lok-worker | Actual workspace, no optional GUI view; exit 0 |
| ely-doc, ely-table, ely-show | Locked actual workspace, debug builds, one job; exit 0 at 22:22 BST |
| Installer, signed release, other apps | Not built or verified in this round |

The engine build started from a fresh pinned source tree and incrementally
incorporated fixes after earlier native build failures. The final result is
not a second clean-from-scratch rebuild. Build products and caches stay on E;
they are excluded from source archives. No installed application was replaced.

Final `mergedlo.dll` SHA-256:
`808BB3321A14EBF1DB02A83EC4FB4C74DABFBFCFD625975D5561209E2F62EFA7`.
Worker SHA-256:
`88E680AC1D53EDF7147F1BDBBE11EADAD34F03C688157D84DC5C36D6A872D7F3`.

## Targeted runtime tests

All seven targeted fidelity tests passed, with zero skips:

- Manual fractional row heights and hidden-row state.
- Chart page/cell attachment and resize behavior.
- Word character-compression modes, including omitted mode.
- DOCX supplemental theme-font mappings.
- XLSX supplemental theme-font mappings.
- PPTX supplemental theme-font mappings.
- Hyperlink import colors, exported theme identity and reopened colors.

Five existing worker stability tests also passed without skips: missing-file
errors, permanent refusal after self-check failure, shutdown cleanup, separate
load sessions and cross-worker document ownership.

The executable compiled the actual `ely-lok` client sources through an isolated
non-GUI manifest and used the actual workspace worker. `ELY_LOK_DIR` and
`ELY_LOK_WORKER` selected the rebuilt engine and worker explicitly. The standard
workspace test suite was not run; 211 compiled tests does not mean 211 passed.
Thirteen synthetic document cases also completed retained save/reopen cycles;
those cycles are not thirteen additional exhaustive compatibility assertions.

Before patches 0060 and 0061, the hyperlink regression failed first at import
and then at exported theme identity. Both failure stages were preserved locally.
The final run passed all stages rather than merely checking for a blue RGB.

Twenty independent reader/mutation tests and the comment-rule check were also
rerun successfully. They are source-level checks, not extra new-engine tests.
Independent review R3 checked the final evidence and claims without rerunning
the tests; its PASS is not a second independent execution.

## Native Microsoft Office observations

Word and Excel used newly created, ownership-checked automation instances.
PowerPoint reused an existing process, with no global-setting changes and no
application Quit; only presentations opened by the test were closed. Fixtures
were synthetic, read-only and checked for macro/ActiveX content. Their embedded
chart workbook was also checked. No personal documents were used.

| Check | Observed scope and limit |
| --- | --- |
| Word complex fixture | Three pages, two tables, one inline shape and eight sample paragraph texts retained; trailing paragraph/section controls recorded separately |
| Word rendered pages | All three original/old/new page comparisons inspected; the known approximately 6.85-point equation-related offset remains |
| Excel row fixture | Six fractional manual heights, hidden seventh row and recalculated formula values match |
| Excel six chart cases | Formula values, source/series values and attachment modes match; this does not assert chart visual equivalence |
| Excel layout | Default width remains 8.54 to 8.00; chart colors, axes, titles and legend visibly differ |
| PowerPoint complex fixture | Four slides and their text retained; all four original/old/new slide comparisons inspected |
| PowerPoint hyperlink fixture | Five expected colors verified in native PNG output and visually; exported theme identity is separately asserted by the regression reader |

PowerPoint's whole-text-range font-color properties returned mixed values,
not authoritative rendered hyperlink colors. The first native color assertion
was a test-harness false positive; raw readings and failing logs remain local.
The corrected assertion checks nonempty text bands in actual native-rendered
PNGs. It is fixture-specific and is not a general screenshot comparator.

The first Word text-hash assertion also detected a terminal section-control
change, not lost body text. The corrected check normalizes only trailing
paragraph/section/table controls and separately records their original codes.
Page counts, visual comparisons and the unresolved offset remain separate.

## Unfinished release gates

GUI binaries have not been launched, edited in, or recovery-tested. Their current
settings, recovery and instance paths overlap the installed app's profile.
Do not assume an environment-only LOCALAPPDATA override safely isolates them.
Implement and verify isolation or use a separate Windows test account first.

General layout equivalence, legacy equations, theme-change/internal-link cases,
complete application tests, installer/uninstaller, signing, licence collection,
fork identity and update-channel separation remain open. No GitHub repository,
commit, tag, pull request or release was created. Earlier source candidates and
failure evidence are preserved, not overwritten.
