# Ely Office local component fixes

This directory carries the tracked source of Ely GPUI Component 0.1.1 at
`de961eb164c1c88797ea35f0860aba9d18c95e4f`, from
https://git.zacharyzhang.com/ZacharyZhang-NY/Ely-GPUI-Components.
The original MIT license and asset licenses remain included. Component source
in the dependency cache and reference checkout is kept unchanged.

The root workspace keeps its original Git revision declaration and overrides
that source with this directory. The component is also a workspace member so
its own regression tests can run with the same GPUI patches as the applications.
Its upstream Web build profile is not the root workspace profile.

Round 22 changes the text input's layout record and geometry consumers:

- A layout records the text and masking state it actually shaped.
- Pointer and character hit testing require a matching text/layout pair.
  A stale layout returns no hit; it cannot assign an old offset to new text.
- Vertical movement preserves the current caret until matching layout exists.
- Existing pre-frame IME candidate bounds remain available; the layout is not
  indiscriminately erased after every text update.

The production changes are in `src/forms/text/{mod,element,geometry,actions,ime}.rs`
and the existing Markdown hit consumers `src/documents/{blocks/span,document/point}.rs`.
`src/forms/tests/mod.rs` registers the new `relayout.rs` regression module.
The portable source patch is recorded under the root
`patches/ely-gpui-component` directory. Apart from those fixes and the publication-only fixture values
described below, tracked upstream files retain the pinned source.

At the owner's request, the public source copy replaces credential-shaped
gallery and component-unit-test samples with clearly invalid placeholders.
Matching fixture expectations receive the same replacement. Test control flow,
the eight-character password boundary, and six-digit code cases are preserved. The sign-in handler rejects locally and the key
manager updates only in-memory demo state; neither connects to an account service.
Runtime library code and Office regression assertions are unchanged. See the
[publication notes](../../PUBLICATION.md).

From the Ely Office workspace root, the focused regressions can be run with:

```text
cargo test -p ely-gpui-component --lib --features test-support forms::tests::relayout:: -- --test-threads=1
```

Test results and the scope of native reproduction are recorded in
`docs/windows-validation-round22.md` in the root project. The historical
Round 21 failure is retained; matching a deterministic storage-free reproduction
does not identify who initiated the earlier Minimize event.
