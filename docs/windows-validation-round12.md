# Windows validation: round 12

Date: 2026-10-23. Bounded continuation at normal scope. This round closes
pending Windows test validation and refreshes the optimized application
build. It does not add features, introduce a C++ engine patch, replace the
installed applications, change user profiles, or publish a release.

## Changes and classification

- The Table test removes and renders away the previous context-menu offer
  before dispatching another real simulated right click. Its direct command
  helper had bypassed the menu component's dismissal path. Existing command
  effects and menu assertions remain; this is a harness correction, not a
  claimed production menu fix.
- Logical test cell reads normalize Windows CRLF. Export tests still compare
  the complete output, including exactly the expected platform line ending.
- Show output target expectations compare paths, not Unix display strings.
- Show PDF output tests use the existing workspace `lopdf` parser. Invalid
  PDF input fails the test rather than missing `pdftotext` causing a skip.
  The dependency is test-only; no production PDF exporter was changed.
- Local native harness clicks and screenshots use an explicit per-monitor
  DPI context, with the prior thread context restored. Owned-window geometry
  and additional menu captures make the interaction evidence inspectable.
  No global keyboard, clipboard, system input method or unrelated window
  was automated. This is local test code, not a product recovery rewrite.

The final Table menu correction had already been written at the end of
round 11 but was not built then. This round synchronized it to the E build
copy, rebuilt the application test executables and executed the corrections.
Later source comparison found all 68 candidate source/manifest/lock files
considered by the comparison matched their build-copy
counterparts. Previously staged changes were preserved.

## Actual complete application test runs

| Application | Passed | Failed / ignored / filtered | Reported skip | Peak owned private memory | Peak workers |
| --- | ---: | --- | --- | ---: | ---: |
| Doc | 64 | 0 / 0 / 0 | No | 1006 MiB | 1 |
| Table | 53 | 0 / 0 / 0 | No | 1154 MiB | 1 |
| Show | 92 | 0 / 0 / 0 | No | 1071 MiB | 1 |

Each process completed with exit 0, no guard abort and no forced cleanup.
Doc includes the one 24-cycle lifecycle soak, not 24 additional test cases.
Table's prior four failures and Show's prior path failure no longer occur.
Show's PDF page-count subcheck now executes without the old missing-tool skip.
The earlier targeted right-click pass and ten targeted Show output passes
are subsets, not extra tests to add to these counts.

These are application tests with actual workers and test application
contexts. They are not complete native-window interaction suites, a full
workspace run, release-mode unit tests, or a multi-day trial. Round-11
library/client results remain historical evidence and were not rerun here.

Application test executable SHA256 values:

| Application | SHA256 |
| --- | --- |
| Doc | `1B1F22FA5D80CEB6A5DF5C50A0C075D35B315094D7EEC941CF5003874BBBE48C` |
| Table | `CF73D3E938178619484EB8309713A5A95C1580221BCE220C796545E9606C6533` |
| Show | `4F86693B66A98ECC1DFBE7DAFB8299E695E0124087D95EF754CE17CEC3ED702D` |

## Native recovery investigation

Round-11's latest native recovery timeout was reproduced and retained.
Additional captures showed that the expected File menu had not opened;
they did not establish that recovery data was lost. The draft recovery
point and recovered tab were already observed before the timeout.

One diagnostic also exposed the owned test window and requested paint
frames; it still timed out. After removing the temporary exposure and
unifying click DPI handling, the same debug application and worker hashes
completed the entire plain-document recovery flow. Since the diagnostic
changed more than one harness setting, this is evidence for corrected
coordinate/DPI handling and a passing flow, not proof that DPI was the sole
possible cause of every timeout. No product recovery code was changed here.

The later debug pass checks draft contents in a recovery point, detection of
the intentional crash, saving the recovered draft outside scratch, original
file preservation, a normal close and no spurious restore on clean restart.
It exits 0 with no unexpected forced cleanup. Earlier failed experiments
remain failures and are not relabelled as passes.

## Optimized build and native release checks

The existing E build cache was used for an offline, locked incremental
optimized build of Doc, Table, Show and the LOK worker. It exited 0 in
1m03s. This is not a clean rebuild of all dependencies or the C++ engine.
The engine remains the existing 63-patch build.

Two isolated native Doc workflows then used the optimized application and
worker, both at the current window DPI of 168:

- Save: Unicode ordinary save, extensionless DOCX, cancel without output,
  real overwrite confirmation declined without changing the target,
  explicit ODT without a spurious DOCX, unsupported PDF refusal, and
  fresh-profile reopening without rewriting its file all pass.
- Recovery: intentional termination after a verified recovery point,
  same-profile reopening of the unsaved draft, content-checked save to a
  new file, original preservation, normal close and clean restart all pass.

Both workflows complete with exit 0 and no unexpected forced cleanup. The
intentional crash is the recovery stimulus, not a spontaneous app crash.
Screenshots of the opened menu, overwrite confirmation and reopened file
were visually checked. Unicode messages are not system IME composition.
Reopen captures wait for the first tile, not a completely settled viewport;
partially filled tiles can still be visible. File contents are checked by
parsing the saved document. These screenshots do not certify full visual
fidelity, complete tile presentation or complex-document layout.

Review also identified two non-blocking local-harness limitations: failed
DPI-context changes were not rejected, and one reopen branch recorded but
did not reject a nonzero exit before marking completion. Both checks were
made explicit. The two native release workflows were then repeated with
the hardened harness; both again passed with the same app/worker hashes,
exit 0 and no unexpected cleanup. Repeat runs are not extra application
test cases.

Shared release evidence hashes:

- Doc: `A9536AB6EC4DE7E0EEA92160C54FAABA223B422279239185AE7F6D079F2DC370`.
- Worker: `8A8AC83E492F020546EEC6799F56A9B169E5D0765D374FBABB905C896632708F`.

## Evidence index

Under the local private `E:/ElyOfficeBuild/qa-round12` root:

- `ely-trial-round12-doc-full-20261023-154228-b274f579`.
- `ely-trial-round12-table-full-20261023-154230-1414daec`.
- `ely-trial-round12-show-full-20261023-154228-41636e85`.
- `ely-trial-round12-table-menu-20261023-154101-cd26babd` (subset).
- `crash-debug-20261023-154024`, `154406` and `154802` (retained failures).
- `crash-debug-20261023-154934` (later debug pass).
- `win32-release-20261023-155108` (optimized native save pass).
- `crash-release-20261023-155108` (optimized native recovery pass).
- `win32-release-20261023-155559` (final hardened-harness save pass).
- `crash-release-20261023-155601` (final hardened-harness recovery pass).

The targeted Show output subset is under the previous QA root as
`ely-trial-round12-show-output-20261023-154012-fc588502`.
Test compilation log: `.work/round11/test-only-round12.log`.
Optimized build log: `.work/round12/release-build.log`.
Do not upload these raw logs/profiles without reviewing local paths and
synthetic/private content. No logs, credentials or real documents were
published by this round.

## Review and remaining gates

Scoped platform-test correction review R1 and bounded native evidence review
R1 passed. Final Save As review R3 passed and explicitly corrects the invalid
old GUI baseline claim; the independent COM probe is API evidence only.
The hardened local-harness checks and repeat evidence passed independent
R2 review. Review records are in `tasks/reviews/local-round12-*.md` and
`tasks/reviews/local-round11-save-r3.md`; no review result certifies a whole
workspace or long-term stability.

The deep-profile startup limitation from round 11 is not fixed; shallow
test profiles avoid it without changing Windows policy. Final shared
lifecycle/runner review, native Table/Show release interactions, encrypted
native recovery, system IME composition, multiple displays/DPI settings,
installation/upgrade/uninstall, multi-day trial and complex-document fidelity
remain separate gates. No installer, tag, commit or GitHub upload is implied.
