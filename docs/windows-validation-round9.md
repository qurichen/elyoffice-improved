# Windows validation: 2026-10-20, round 9

Experimental source candidate, not an installer or production release.
Build products and temporary trial data remain on E. The 61-patch C++ engine
and actual worker are unchanged from [round 8](windows-validation-20261020.md).

## Explicit trial profiles

`ELY_DATA_DIR` selects the suite's shared state root. Shared/app settings,
logs, recovery, worker profiles and instance locks/discovery use that root.
Unsetting it preserves the original default. Invalid paths are errors rather
than a silent fallback. Use a fresh folder without symlinks or junctions.
This does not sandbox source documents or change installer/registry identity.

Five current-source directory unit tests and five actual cross-process
integration tests passed. The latter cover default-path queries without writes,
Unicode profile state placement, invalid/unwritable paths, per-profile recovery
clearing and two simultaneous holders with handoff within their own profiles.
The integration helper is explicitly called in child processes, not counted
as an extra passed test. None of the ten selected checks was skipped.
The five integration checks also passed twenty consecutive reruns: 100 repeat
executions, not 100 additional distinct tests.

R1 found a test-helper stdout read-end lifetime defect. The fix keeps and drains
the pipe until child exit; a release handshake also prevents exit before the
instance response. R2 static review passed. Initial failing logs are preserved.
An intermediate lib-test result used an old test name because copied source
timestamps preceded the cached compilation. Refreshing the E-copy timestamp
and rebuilding produced the current-source five-test result; source bytes did
not change during that correction. It was not counted as a fresh-source pass
until that final rebuild.

## Actual native GUI checks

The actual locked workspace rebuilt ely-doc, ely-table and ely-show with one
build job. Before launching, their hashes and reviewed profile-source hashes
were checked against the successful build record. Each trial received its own
fresh profile and only a disposable synthetic complex-document copy.

| Native application | Verified observation |
| --- | --- |
| ElyDoc | Opened three-page DOCX sample; native window and document tile drawn; normal close, exit 0 |
| ElyTable | Opened XLSX sample with its worksheet tabs; native window and grid drawn; normal close, exit 0 |
| ElyShow | Opened four-slide PPTX sample, including hidden-slide indication and notes; native window and slide drawn; normal close, exit 0 |

Three nonblank native-window captures were inspected. No forced process cleanup
was required; no ely-lok-worker process remained afterward. The three sample
copies were byte-identical after closing. The suite shared settings and three
app-settings locations were absent before and after; this limited observation
is not a full audit of every installed-profile file or a claim about contents
of settings that did not exist. The existing installed app was not replaced.

These checks establish native launch, document rendering and clean close.
An additional ElyDoc desktop scenario confirmed same-profile file handoff,
retention of a work-folder sentinel, and two different profiles rendering
simultaneously. Both primary windows closed normally, without forced cleanup.
They do not establish keyboard/mouse editing, input methods, save dialogs,
crash recovery, all layouts, all application functions or installer safety.
Automated application-logic tests use GPUI's test context, not the real desktop;
their result must be recorded separately from these native-window checks.

## Application-logic regressions

All three selected real-workspace tests passed against the rebuilt engine and
worker, without skips, in GPUI's test context. They are not native input tests.

| App | Assertion scope |
| --- | --- |
| Doc | Create a report, insert text, save DOCX and return to clean saved state; this selected test does not reopen the file |
| Table | Edit a cell, add a sheet, save XLSX, close/reopen and retain both sheet names; the selected assertion is not a complete cell-data comparison |
| Show | Insert a slide, save PPTX, close/reopen into a new tab, retain two slides and two thumbnails, and return clean |

The first run passed Doc but stopped while building Table at resource-compiler
auto-discovery. The active SDK compiler independently compiled the same GPUI
resource. Selecting that verified compiler explicitly through process-local
`RC` allowed the full three-test run to finish. Neither SDK registry state nor
third-party dependency source was modified; the failed attempt remains local.

Independent review is static/evidence review, not a second execution. The
full-workspace suite and other compiled but unselected tests were not run.

## Remaining gates

Word legacy-equation offset, Excel default widths and chart appearance remain
unresolved. Complete interactive editing, input methods, recovery, installer/
uninstaller, signing, distribution licences, fork identity and update-channel
separation remain open. This round does not claim general Office compatibility
or a successful full-workspace test run. No GitHub repository, commit, tag,
pull request or release was created. Earlier source candidates are preserved.
