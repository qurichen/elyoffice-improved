# Round 10: targeted fidelity and safer recovery

Experimental continuation of this fork, not an upstream release or a rewritten
office engine. The pinned LibreOffice source remains
`8221e31b3ac356a1623c672912a3d2b492f7e3d1`. Patches 0062 and 0063 extend the
previous 61-patch sequence. Keep original authorship and licences.

## Editable legacy equations

Word's legacy EQ fields and OMML can give different inline equation sizes.
Converting a supported EQ fraction to OMML changed the following paragraph
positions even when the equation's values survived. Changing only its font
or adding a fixed paragraph offset did not solve the native Word comparison.

Patch 0063 records the source kind as a Boolean in the current math model.
It regenerates supported EQ field instructions and their cache from the
current StarMath tree. Editing a fraction from 1/2 to 7/9 changes the output;
no original instruction, source XML or fixed layout coordinates are replayed.

The intentionally narrow accepted tree contains ASCII literal text, nested
stacked fractions, one-content line/expression containers and uniform size
wrappers. The leaves must agree on font, size, weight and posture. Leading
or entirely blank literal expressions, literal quotes, multiple spaced
content nodes, mixed styles, radicals, scripts and other unsupported trees
continue through the existing OMML exporter. This is not general EQ support.

Generated field runs explicitly set current font, size, bold, italic and
black/automatic color, including false bold/italic values. A surrounding red,
bold or italic paragraph cannot implicitly supply those omitted properties.
EQ cache handling prevents duplicate cached display text on import.

MathML annotations carry only the source-kind Boolean and the current text
font and base size. ODT/FODT round trips can therefore retain this information
without carrying source XML. The auxiliary properties are applied only to a
confirmed WordEQ model. A missing/false marker must not change ordinary math
styles. The current exporter emits the marker before these properties; this
does not establish support for arbitrarily reordered private annotations.

The synthetic regression covers direct DOCX, two repeated DOCX saves,
ODT/FODT intermediate saves, edited values, safe rejection of unsupported
trees, ordinary OMML, stale field caches and private-metadata isolation.
The native comparison is narrower than the semantic test coverage: the
complex sample's eight following paragraphs match the original positions
after saving and reopening in Word. Other layout differences remain.

## Default chart point colors

For the tested modern classic DrawingML charts, Excel treats an omitted
`c:varyColors` as enabled. The previous importer initialized it to false,
turning single-series bar/pie points into one color when saving.

Patch 0062 follows the existing modern/Office-2007 Boolean distinction when
initializing classic-chart models. Explicit true/false still overrides it;
chart-type and multiple-series converter restrictions remain. The change
does not apply classic-chart defaults to ChartEx models. It does not change
the theme palette, automatic chart styles or explicit shape fills.

The fixture has absent/false/true controls, a style control, a purple explicit
fill, bar and pie charts and a multiple-series negative control. Readers
check source cells, category and series-name references, caches, effective
point colors, series count and the imported/reopened model. They do not
declare success from the presence of a color token alone.

## Windows save and backup access policy

Windows file permissions exposed by Rust's portable permissions type do not
represent a file's DACL. Replacing a protected file with a newly inherited
temporary file previously widened access. Backups had the same problem.

Existing-file staging and backups now snapshot the source DACL and pass a
protected descriptor when exclusively creating the empty file. This avoids
a create-then-restrict interval in which a broader read handle could be
opened. The original inheritance/protection state is restored using the same
descriptor and file handle before document bytes are written. A new document
with no prior file continues to inherit its destination directory's policy.

Metadata errors other than NotFound and native security errors are exposed.
Windows read-only targets are refused before staging. Native paths are
validated and normalized for long names; CREATE_NEW does not clobber a
pre-existing backup name. The existing atomic replacement mechanism remains.

Tests cover protected and inherited/explicit DACLs, private backups, new-file
inheritance, sharing violations, read-only refusal, null DACL variants, empty
staging policy, invalid paths and existing long-name regressions. They do not
establish owner/SACL/alternate-stream preservation, an adversarial filesystem
sandbox, network-share behavior or native FAT/exFAT behavior. An empty
deny-all DACL is not the same as the tested null-DACL cases.

Relevant API contracts: [object creation security](https://learn.microsoft.com/en-us/windows/win32/secauthz/security-descriptors-for-new-objects),
[handle rights](https://learn.microsoft.com/en-us/windows/win32/secauthz/requesting-access-rights-to-an-object),
[reading descriptors](https://learn.microsoft.com/en-us/windows/win32/api/aclapi/nf-aclapi-getnamedsecurityinfow)
and [setting a handle's security](https://learn.microsoft.com/en-us/windows/win32/api/aclapi/nf-aclapi-setsecurityinfo).

## Recovery-key profiles and lifetime

Round 9 isolated ordinary document recovery, but encrypted recovery still
selected the shared formal credential. Round 10 closes that separate path.
With ELY_DATA_DIR unset, the previous service/account and account-home-derived
lock remain unchanged. A valid explicit profile uses a separate credential
service and a bounded SHA-256 account identity derived from its canonical
path in the platform's native encoding. Its initialization lock is local to
that canonical directory. Invalid profiles cannot select the legacy backend.

Canonical aliases share the same profile. Recreating the same canonical path
keeps its identity; moving or renaming that directory changes it. This is not
portable encrypted recovery across machines or arbitrary directory moves.
The selected profile is fixed for the process lifetime. Native credential
storage remains the backend; no plaintext key file is introduced.

If persistence fails, existing session-only key behavior remains. Successful
encrypted recovery now produces a one-time, unlogged user warning that it
cannot survive app exit/restart and that edits should be saved normally.
Plain documents never load a recovery key for this operation; persistent
keys do not consume the warning gate. Failed snapshots retain their original
error rather than falsely reporting successful recovery. Normal save is
unchanged. Tests inject the key loader and do not access real credentials.

## Remaining fidelity work

Default spreadsheet widths still differ. Nine explicit-font workbooks with
45 sheets were compared in native Excel and the old engine. Excel used
168-DPI pixel quantization on this machine; the headless engine used 96 DPI.
A fixed +0.54 or unconditional eight-pixel rounding would worsen other
font/size cases. No such compensation patch was added.

Chart title/style and overlay behavior, some default paragraph spacing,
unsupported-EQ OMML formatting and the complex sample's footnote separator
length remain different. The native results do not justify a claim of
pixel-perfect Office compatibility. Interactive dialogs, system IME,
encrypted GUI crash recovery, installers and other platforms still need
their own validation. See the round-10 Windows validation record for exactly
which current-source executions passed.
