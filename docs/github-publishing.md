# GitHub publication checklist

This branch is a derivative of
[ZacharyZhang-NY/ElyOffice](https://git.zacharyzhang.com/ZacharyZhang-NY/ElyOffice),
based on commit `0d991e1c715859abd1282981fd46989e7e2319c7`.
It adds Windows build fixes, document-fidelity patches and regression
fixtures. Preserve the original copyright, attribution, licence texts
and patch history. Do not describe it as the upstream author's release.

## Source preview versus software release

A source-only preview may document incomplete work. Label it experimental
and state exactly which tests ran. Do not attach an old upstream installer
as if it contained these fixes. A locally exported index snapshot is not
a committed, versioned or audited production release.

Before publishing:

- Choose the GitHub repository name and fork maintainer identity.
- Review every staged file and retain `LICENSE` and third-party notices.
  README and `Cargo.toml` now agree on AGPL-3.0-or-later. See
  [licence collection](licenses.md); final distribution review remains open.
- Keep `.git`, `.work`, `engines`, `reference`, tool downloads, caches,
  personal documents, diagnostics, credentials and local build logs out
  of source snapshots. Existing generated test fixtures are intentional.
- Review test signing material as test-only, never production credentials.
- Check filenames and content for secrets and personal data; a simple
  pattern scan is not a comprehensive security audit. Git history also
  needs review if publishing it rather than importing a source snapshot.
- Review and commit the intended changes. Choose the version only after
  establishing the release scope; do not overwrite an existing tag.
- Set a new publishing remote after confirming its owner and visibility.
  Do not push to the original author's server or replace `origin` blindly.

## Upstream infrastructure still present

The inherited release automation is `.gitea/workflows/release.yml`, not
a tested GitHub Actions workflow. `scripts/ci.sh` uploads to upstream
Gitea; do not run it to publish this fork. No GitHub publishing automation
or credentials have been configured by this work.

The actual in-app update entry point, `crates/ely-ui/src/updates.rs`, points
to this fork's GitHub release feed, `qurichen/elyoffice-improved`. Source updates
create no release assets and do not offer an installable update through it.

The inherited website/release tooling, installer publisher fields, application
identifiers and signing settings require separate review before binary releases.
Original attribution remains. Do not run upstream upload jobs. Source publishing
uses the repository owner's authorized GitHub connection, not those jobs.

## Binary release gates

- Full pinned C++ engine build with all current patches, exit zero.
- Rebuilt worker loading that engine; no use of an old installed library.
- DOCX/XLSX/PPTX round trips and independent output assertions pass.
- Complex-document native layout comparisons, including remaining equation
  fallbacks, paragraph/footnote formatting and spreadsheet column widths.
- Complete GUI build, opening/editing/saving tests and crash/recovery checks.
- Installer/uninstaller in an isolated location, preserving existing files.
- Dependency licences, required corresponding source, notices, signing and
  checksums checked against the actual distributed binaries.
- Separate trial installation and rollback plan before replacing the old app.

No repository, issue, pull request, tag or release is created automatically
by these notes. Upload only after the owner approves the intended contents.
