# Ely Office

This working branch improves Windows build reliability and OOXML fidelity.
It is based on [ZacharyZhang-NY/ElyOffice](https://git.zacharyzhang.com/ZacharyZhang-NY/ElyOffice)
at `0d991e1c715859abd1282981fd46989e7e2319c7`, not a new implementation
or an official upstream release. Original authorship and licences are retained.

The native Windows engine now builds with all 63 patches. Ninety-three selected
client/worker regressions pass against it, without skips; this is not the full
workspace suite. Supported legacy EQ fractions retain editable field semantics,
and the complex native Word sample's eight following paragraphs now match the
original positions after saving and reopening. Modern chart point-color defaults,
Windows save/backup DACLs and encrypted recovery profile scoping are repaired.
Session-only encrypted recovery has a one-time user warning. Doc/Table/Show
GUI binaries also rebuild successfully. Complete interactive/recovery and
installer checks remain open; native Office still shows other layout differences.
See [round 8 validation](docs/windows-validation-20261020.md) and
[round 9 GUI/profile validation](docs/windows-validation-round9.md).
The current scope and limits are in [round 10 validation](docs/windows-validation-round10.md)
and [round 10 implementation notes](docs/ooxml-fidelity-round10.md).
See [repair evidence](docs/ooxml-fidelity-fixes.md),
[Windows build notes](docs/windows-build.md) and
[publication gates](docs/github-publishing.md).

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
