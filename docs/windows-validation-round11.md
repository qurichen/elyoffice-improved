# Windows long-trial validation: round 11

Historical closeout. Subsequent scoped fixes and newer runtime results are
recorded in [round 12](windows-validation-round12.md); failed round-11 runs
below remain part of the evidence history, not the latest candidate result.

This is an experimental local source candidate, not an installer or a
production-release declaration. Existing installed applications and normal
user profiles are unchanged. No round-11 commit, tag or upload is implied.
The pinned Windows C++ engine remains the existing 63-patch build; this round
does not introduce another engine patch or claim another clean engine build.

## Repairs

- Windows Save As supplies an explicit native default extension. Generic
  path/directory prompts do not infer an extension from a suggested name.
  The native dialog resolves the final name and overwrite confirmation;
  application code does not append a suffix after confirmation.
- Recovery restoration authenticates and validates encrypted metadata before
  publishing a destination. A same-directory temporary file is synced and
  published without replacing an existing target. Failed copies, missing
  sources and incorrect keys leave no empty destination blocking a retry.
- Recovery index entries are validated before pruning or taking the session
  marker. Invalid path components, malformed slots, duplicate slots and
  invalid formats fail visibly instead of being used as filesystem paths.
- Automatic reopening reads only the latest recovery point's metadata. A
  corrupt older point cannot block an intact latest point. A corrupt latest
  point still reports an error; it does not silently fall back to older work.
- Closing a document waits for the kernel to release its handles before
  deleting scratch files. Cleanup accepts only a canonical strict child of
  the owned scratch root. User-facing Open/Save As refuse disposable scratch
  locations; internal template and recovery copies remain supported.
- Workers explicitly allow the bundled English dictionaries. The pinned
  engine disables language support when its language allowlist is absent.
  The setting belongs to the worker child process, not the user's system
  environment. This does not add Chinese grammar or non-English dictionaries.
- Manual GPUI test contexts run application cleanup on scope exit, including
  unwinding. The helper is shared through the existing test-support feature;
  it is not part of normal release builds.

## Closeout status: 2026-10-23

This session is closed at the user's request. The source candidate is not
release-ready: Table and Show still have failed baseline cases whose final
test-harness corrections have not been rerun, the latest native recovery
attempt timed out, and final independent reviews are incomplete. Keep these
failures alongside earlier successes. Do not use this document as a release
approval. See the [Chinese closeout report](windows-round11-closeout.zh-CN.md).

## Recorded executions

| Scope | Observed result | Limit |
| --- | --- | --- |
| Native default-extension helper | 4 tests pass | Windows COM setup/validation, not a complete GUI suite |
| Files library | 118 tests pass, 2 ignored child helpers | Includes the 29 recovery tests below; not 120 independent passes |
| Recovery library | 29 tests pass | Subset of Files; synthetic contents and keys; no real credential store |
| Windows ACL integration | 7 tests pass | Synthetic filesystem permissions, not a whole-device audit |
| Office library | 24 tests pass | Includes metadata, retry, path and junction checks |
| Doc save/close/path safety | 6 tests pass | Included in the 64-case Doc run; actual worker, mock application context |
| Doc application tests, v3 | 64 pass, 0 fail, no skips | Includes one 24-cycle edit/cancel/save/close soak; not 24 extra test cases |
| Spelling client | 2 tests pass, including US/GB/AU/CA | Included in the 97-case client collection; inherited `zz` is replaced worker-locally |
| Selected kernel-client regressions | 97 distinct tests pass, no failed groups | Actual client sources, isolated non-GUI manifest; bounded chunks and resume with the same binary/worker/engine hashes, not a single uninterrupted run or full workspace |
| Shared context cleanup guard | 1 unwind test passes | Test-support only; full app runs peak at one worker each |
| Bilingual UI catalogue | 5 tests pass | Catalogue checks, not all translated screens |
| Native Doc Save As, latest debug | Complete scripted workflow passes | Owned windows, fresh profiles, synthetic documents, 168 DPI; exit 0, no forced cleanup |
| Native Doc crash/reopen, earlier debug | Complete scripted workflow passes | Earlier artifact; intentional termination of its own process tree only; plain recovery |
| Native Doc crash/reopen, latest debug | Fails: timeout at `save-recovered-copy` | Recovery point and crash detection observed; no verified recovered save/clean restart; owned process cleanup was forced |
| Table application tests, v3 | 49 pass, 4 fail | Three CRLF expectations and a second context-menu click; corrections not rerun |
| Show application tests, v3 | 91 pass, 1 fail; PDF page-count subcheck skipped | Path separator expectation fails; missing `pdftotext` caused the subcheck skip; corrections not rerun |
| Runner synthetic controls | 3 controls pass | Argument quoting, orphan cleanup and renamed-worker limit; not application regressions |
| Later test-only rebuild, v4 | All three test executables compile | Includes Show's in-process PDF parser; final Table menu refresh edit was made later and is uncompiled |
| Optimized release build | Earlier source builds successfully | Predates final spelling/helper changes; not the final candidate and not native-release verified |

Counts overlap as identified above and must not be added into a single
"total passed" figure. Later compilation does not upgrade old runtime results
to the later artifact. No full-workspace test or multi-day soak was completed.

The native save workflow checks ordinary save, Chinese/emoji preservation,
an extensionless name becoming DOCX, cancel producing nothing, declining a
real overwrite confirmation without changing its target, explicit ODT,
unsupported PDF refusal and fresh-process reopening without rewriting the
file. The overwrite confirmation and reopened document were visually
inspected. Input text was delivered through owned-window Unicode messages,
not the system IME. No global keyboard, system clipboard or other person's
window was automated. These are not multi-DPI or accessibility results.

The crash workflow is intended to wait for a recovery point containing an unsaved synthetic
draft, intentionally kills its own app tree, restarts the same isolated
profile and waits for the recovered tab to load and draw. It then saves the
recovered text outside scratch, checks that the original remains unchanged,
closes normally and confirms a clean restart does not restore it again.
The earlier successful run used no unexpected forced cleanup and exited zero.
However, the latest debug-artifact rerun timed out while waiting for the save
dialog at `save-recovered-copy`, before a recovered output was verified. It
then cleaned up its owned app tree forcibly. The evidence does not yet
distinguish an automation/timing problem from an application defect; neither
an end-to-end recovery pass nor a proven recovery data-loss bug is claimed.
This does not establish recovery from disk failure, power loss, an encrypted
native document, or edits made after the last ten-second recovery point.

Earlier GUI harness attempts did not reliably submit the native filename
field. They are retained as failed experiments, not claimed product bugs or
pre-fix reproductions. In particular, the old round-11 `native-before` dialog
was never effectively submitted. A separate COM probe demonstrates the
default-extension API behavior, but is not an old ElyDoc GUI regression run.
The valid harness uses the edit control's replace-selection operation and
native owned dialog commands; it waits for the recovered document rather
than mistaking the initial blank tab's first frame for recovery completion.

## Resource-aware Windows regression runner

PowerShell 7.2+ can run an already built application test executable with an
isolated profile and temporary directory:

```powershell
& tests/scripts/windows-trial-regressions.ps1 `
  -Binary '<target>/debug/deps/<application-test>.exe' `
  -KernelDirectory '<engine>/instdir/program' `
  -Worker '<target>/debug/ely-lok-worker.exe' `
  -TemporaryDirectory '<existing-short-temp-folder>' `
  -OutputDirectory '<existing-output-folder>' -Filter '' -Label doc
```

Build the test executable using `cargo test --locked -p ely-doc --no-run`
(or `ely-table` / `ely-show`) and use the exact executable Cargo reports.
Do not guess the hash-bearing filename or run the macOS-only Show compose
target on Windows. The runner serializes tests, records the executable hash,
exit code, skips and process-tree memory/worker measurements. It terminates
only its own test tree if worker count, memory headroom or duration limits
are exceeded. A guard abort, skip, partial run or repeated case is not a
distinct passed regression. Logs can contain local paths and must be checked
before sharing. A Windows job owns the launched root and descendants, with a
short exit-accounting grace period; lingering descendants still fail the run.
Three synthetic controls passed after fixes for renamed workers and orphaned
descendants. The last requested independent review did not execute because
the service usage limit was reached; this is not a review PASS or a finding
that the code is unsafe.

## Evidence index

Local private build evidence is retained under the QA root, not uploaded:

- Doc: `ely-trial-doc-full-v3-20261023-071949-e18f7b3a`.
- Table: `ely-trial-table-full-v3-20261023-071951-262f2c75`.
- Show: `ely-trial-show-full-v3-20261023-071950-28b223c2`.
- Latest native save: `win32-debug-20261023-074248/results.json`.
- Earlier native recovery pass: `crash-20261023-062643/results.json`.
- Latest native recovery timeout: `crash-debug-20261023-074630/results.json`.
- Runner controls: `ely-runner-controls-7dd151364c9e486988aa49fb65cd833d`.

Local workspace evidence includes `.work/round11/final-libraries.log`,
`core-short-temp-final/core-new-results.json`, `test-only-v4.log` and
`release-build.log`. The 97-case collection records 45 verified groups,
97 unique selected tests, and matching artifact hashes across its resume.
Earlier invalid/failed harness runs are retained and excluded from that count.

## Known deep-profile startup limitation

Some kernel startup tests fail when their temporary/profile paths are deeply
nested. The failure also reproduces with the pre-allowlist client/worker, so
it is not evidence that the spelling change introduced it. The same focused
case passes with a separate shallow temporary root. The runner now separates
verbose log labels from short temporary/profile paths, but this does not fix
arbitrary long user-profile paths. No Windows policy/registry change or
production manifest change was made. Keep trial profiles at a short path
until the product-level limitation is resolved.

## Remaining trial gates

Passing these focused checks does not establish Word/Excel/PowerPoint parity.
Round-10 layout limitations, full-workspace coverage, native IME composition,
accessibility, other displays/platforms, encrypted native recovery, installer
upgrade/uninstall and a multi-day personal trial remain separate gates.
Keep backups and use copies of important documents while trialing this build.
