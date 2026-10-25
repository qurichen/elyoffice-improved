# Windows build: compatibility branch

This is a work-in-progress recipe, not an installer release. Keep the
existing installed app unchanged until the publication gates pass.
The actual engine revision is pinned in `scripts/fetch-references.sh`;
Rust and Cargo dependencies use `rust-toolchain.toml` and `Cargo.lock`.

The latest local engine build applies 64 patches. Patch 0064 fixes duplicated
screen-DPI scaling in Windows LOK non-printer graphics; it does not change the
Windows display settings. See [round 14 validation](windows-validation-round14.md)
for the rebuilt DLL identity, image checks and rerun application suites.

## Tools and paths

Use a short, local, task-owned build directory on a drive with sufficient
free space. The local verification uses `E:\ElyOfficeBuild`; this is an
example, not a required path. Do not upload this directory as source.

- Visual Studio 2022 Build Tools, C++ x64 toolset and Windows SDK.
- Cygwin with autoconf, automake, Perl, Python 3, git, patch, bison, flex,
  gperf, zip, unzip, rsync, cpio, gettext, libxml2, NASM and Perl modules
  Archive::Zip, Digest::MD5, IO::String, XML::Parser and Font::TTF.
- LibreOffice's native Windows GNU make and pkgconf in `/opt/lo/bin`.
- The pinned Rust MSVC toolchain; no global PATH change is necessary.

Obtain tools from their official sources and verify provided signatures
or hashes. Cygwin setup signature checking and Windows protection remain
enabled. Do not add broad antivirus exclusions or reuse another person's
signing certificates, account tokens or private package caches.

Official tool instructions: [LibreOffice Windows build](https://wiki.documentfoundation.org/Development/BuildingOnWindows),
[Cygwin](https://cygwin.com/install.html), [Rust](https://rustup.rs/).

## Build the engine

Run from the repository root in Cygwin, not Git Bash:

```sh
export CARGO_INCREMENTAL=0
export ELY_BUILD_JOBS=4
scripts/build-libreoffice.sh --disable-antivirus-check
```

The optional flag skips only the upstream build-directory diagnostic,
which otherwise generates an antivirus test file. It does not disable
Windows security. Omit the flag to retain the upstream default diagnostic.
`ELY_BUILD_JOBS` caps both make and external dependency parallelism;
use fewer jobs if RAM is scarce. Do not run another make in the same tree.

All current patches must be applied as a coherent sequence. The helper rejects
unrelated edits rather than overwriting them. Never modify `reference/`.
On Windows the completed engine is under `engines/lo-src/instdir/program`.
A configure success, active process or some object files is insufficient:
the build command must exit zero and the required libraries must exist.

## Build the worker

Run in a Visual Studio x64 developer environment with the pinned Rust
MSVC toolchain. The target directory may also be placed on D or E:

```sh
export CARGO_INCREMENTAL=0
export CARGO_TARGET_DIR='E:/ElyOfficeBuild/rust-target'
cargo build --locked -p ely-lok --bin ely-lok-worker --no-default-features -j2
```

This builds the worker, not the complete GUI. It does not prove that the
engine loads or that document compatibility is correct.

For GUI/test builds, resource auto-discovery failed on this host even though
the active SDK compiler worked in a direct probe. In the active x64 developer
shell, explicitly select its verified resource compiler before building:

```powershell
$env:RC = Join-Path $env:WindowsSdkVerBinPath 'x64/rc.exe'
if (-not (Test-Path -LiteralPath $env:RC)) { throw 'Active SDK rc.exe is missing' }
```

This is process-local tool selection, not an SDK or registry modification.

## Checks

From Cygwin, run the existing source-level build checks:

```sh
python3 tests/scripts/build-lo-jobs-test.py
python3 tests/scripts/lo-patch-prefix-test.py
python3 tests/scripts/antivirus-probe-test.py
```

For Meson activation, use native Windows Python with `ELY_LO_SRC` pointing
to the engine tree containing its pinned Meson tarball. Set
`ELY_TEST_REAL_VSENV=1` to opt into actual VS activation and compiler tests:

```powershell
$env:ELY_LO_SRC = 'E:/ElyOfficeBuild/elyoffice/engines/lo-src'
$env:ELY_TEST_REAL_VSENV = '1'
python tests/scripts/meson-env-test.py
```

In a native developer environment, also run
`python -X utf8 tests/scripts/xmlsec-encoding-test.py`. The XMLSec tests
check the Windows recipe and a real code-page-936 failure versus UTF-8
compile-link-run. Use `-X utf8` for native Python source-inspection scripts
on Windows installations whose default text encoding is not UTF-8.

Missing prerequisites produce explicit skips; a skipped check is not a pass.
The Meson tests cover Unicode environment values, Unicode and special-character
paths, compiler activation errors, temporary-file cleanup and real compilation.
The approach avoids console decoding, similar to
[newer Meson environment transfer](https://github.com/mesonbuild/meson/blob/master/mesonbuild/utils/vsenv.py).

Only after the full engine builds should the new worker run the fidelity
regressions and native layout checks. Keep these results separate from
source-reader tests and tests against an older installed engine.

The [2026-10-20 validation record](windows-validation-20261020.md) reports the
completed 61-patch native engine, worker, targeted runtime tests and three GUI
builds. Compilation is not GUI runtime validation. The current app profile is
shared with the installed version; isolate settings, recovery and instance
discovery before launching a candidate alongside it. Never redirect global
Windows profile settings just to run a smoke test.

## Explicit trial profile

`ELY_DATA_DIR` selects the suite's state directory, including shared settings,
per-app settings, recovery, worker profiles and instance locks/discovery. Unset
means the original platform location. Empty, relative, filesystem-root and
parent-traversal paths are errors, never a fallback to the installed profile.

Set it only in the candidate process environment, to a fresh absolute folder:

```powershell
$env:ELY_DATA_DIR = 'E:/ElyOfficeBuild/trials/example-profile'
$env:ELY_LOK_DIR = 'E:/ElyOfficeBuild/elyoffice/engines/lo-src/instdir/program'
$env:ELY_LOK_WORKER = 'E:/ElyOfficeBuild/rust-target/debug/ely-lok-worker.exe'
& 'E:/ElyOfficeBuild/rust-target/debug/ely-doc.exe'
```

Do not reuse an existing profile or a symlink/junction pointing at one. This
isolates suite state, not source documents, registry identity, file associations
or installed update channels. Open only disposable copies in a GUI trial.
The existing installer does not create or remove arbitrary trial profiles.
