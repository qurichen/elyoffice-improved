# Panics

Started 2026-10-06. A report counted the crate's panics: handlers left out, keys missing from their lists, outside failures, asserts, `expect`, `unreachable!`. Measured in library code, tests aside: 158 `panic!`, 477 `assert!`, 23 `assert_eq!`/`assert_ne!`, 343 `expect`, 14 `unreachable!`.

Rules, at the owner's word:

- A handler a component needs is an argument of `new`. Leaving it out fails to compile, not at render.
- A state live data or the outside world can reach (a key gone from a list that changed, a failed load) logs an error naming the owner and draws a visible state: nothing chosen, or an error line in the component's box. The app keeps running.
- An assert on a caller's argument, or on a fact true by construction, stays, as std's do. One that runtime data or input can trip is fixed. Reachable means what a component meets on its own: input inside it, its own later state, a call to the outside, or two arguments that agree at one time and not the next (a selected key and its list). One value checked against itself (a ratio above zero) is the caller's contract. A current index or key checked against its list is two arguments even in one call: an owner that rebuilds from live data reaches the mismatch. A list's own emptiness is one value.
- An unknown system time zone logs once and reads as UTC; every time shown in the system zone goes through `format::datetime`, which says "UTC" then. Each item routes its own times there.
- A paint call that fails inside gpui (glyphs, the sprite atlas) logs an error naming the component and skips that draw; drawing an error state would meet the same fault.

Each item reads every site in its folders, fixes what the rules name, updates callers in the gallery, docs examples and tests, passes `scripts/check.sh`, and passes a codex review (`gpt-6.1-sol`, effort `high`) in a herdr split, five rounds at most. Then it commits and pushes.

## Items

- [x] P00 Policy in AGENTS.md
- [x] P01 Primitives — `src/primitives`, `src/lib.rs`, `src/assets.rs`
- [x] P02 Typography — `src/typography`
- [x] P03 Layout — `src/layout`
- [x] P04 Window & Shell — `src/shell`
- [x] P05 Buttons & Actions — `src/buttons`
- [x] P06 Forms — `src/forms`
- [x] P07 Navigation — `src/navigation`
- [x] P08 Menus — `src/menus`
- [x] P09 Overlays — `src/overlays`
- [x] P10 Feedback — `src/feedback`
- [x] P11 Loading & Motion — `src/motion`
- [x] P12 Data Display — `src/data_display`
- [x] P13 Lists & Trees — `src/lists`
- [x] P14 Tables — `src/tables`
- [x] P15 Charts — `src/charts`
- [x] P16 Finance — `src/finance`
- [x] P17 Code Editor — `src/editor`
- [x] P18 Terminal — `src/terminal`
- [x] P19 Git — `src/git`
- [x] P20 Debug — `src/debug`
- [x] P21 Documents — `src/documents`
- [x] P22 Collaboration — `src/collab`
- [x] P23 AI Chat — `src/chat`
- [x] P24 Agent — `src/agent`
- [x] P25 Generative — `src/generative`
- [x] P26 Media — `src/media`
- [x] P27 Files — `src/files`
- [x] P28 Messaging — `src/messaging`
- [x] P29 Mail — `src/mail`
- [x] P30 Calendar — `src/calendar`
- [x] P31 Project — `src/project`
- [x] P32 Canvas & Design — `src/canvas`
- [x] P33 DB & Dev Tools — `src/devtools`
- [x] P34 Dashboard — `src/dashboard`
- [x] P35 Settings — `src/settings`
- [x] P36 Account — `src/account`
- [x] P37 Onboarding — `src/onboarding`
- [x] P38 Interaction — `src/interaction`
- [x] P39 Theme — `src/theme`
- [x] P40 i18n & a11y — `src/i18n`
- [x] P41 Maps — `src/maps`
- [x] P42 Misc — `src/misc`
- [x] P43 Library Tooling — `src/tooling`
- [x] P44 Rendering — `src/rendering`
- [x] P45 Recount, acceptance table

## Log

One line per item: what changed, what stayed and why, review rounds.
- P00: AGENTS.md holds the three rules; this list follows the PRD's chapters. Review: 2 rounds (round 1 found the runtime clause missing).
- P01: `init` returns `anyhow::Result`; a missing app icon logs once and shows `Icon::broken`. Kept: `layer.rs` expects (gpui's element order), `checked_ratio` (one value, the caller's). Review: 2 rounds.
- P02: `Link::new` takes its handler; `system_zone` logs once and reads as jiff's unknown zone, which `datetime` marks UTC; an oversized highlight query logs and marks nothing; a cut line's paint fault logs. Kept: caller contracts (speed, digits, step, keystroke, heading level, pattern) and the exponent parse. Review: 3 rounds.
- P03: `PaneGroup` leaves a gone or last pane and logs (`split` returns `Option`); a stale split path sets nothing; `SplitPane` splits evenly when sizes lag or fail, seeds by the owner's raw sizes, and drops a drag over another pane count; masonry sorts with `total_cmp`. Kept: dock ids (panels never leave), one-value contracts, pane-id overflow, workspace JSON. Review: 2 rounds.
- P04: no change. Kept: splash progress and column count (caller contracts), the objc class under `Once`, `Tray::pick` (items fixed at creation). Review: 1 round.
- P05: a `SegmentedControl` whose choice left its segments marks none; the cause, `motion::slide`, logs and returns no marker (P11 inherits it). Kept: one-value contracts (group and quick-action counts, toggle label or icon, share payload). Review: 1 round.
- P06: a root path shows whole; `TextInput::select` logs a range past the text or inside a character; the text element logs shaping and paint faults; a unit gone from its list logs; cron runs in the system zone go through `datetime`. Kept: empty choice lists (the owner disables, as AGENTS.md says), one-value contracts, facts by construction. Review: 2 rounds.
- P07: `Wizard::new` takes `on_step` and `on_finish`; tabs, crumbs, history, steps, wizards and pagination with a place their list no longer holds log and mark none; an empty or all-disabled tab strip ignores arrows; a gone recent command is skipped. Kept: non-empty lists, disabled-free switcher and search items, row index parse. Review: 3 rounds.
- P08: no change. Kept: a bar's menus and a pie's slices (one value each); the panel's level lookup, which `settle` fits to the menu before every draw. Review: 1 round.
- P09: a `Lightbox` or `Tour` past its last item logs and closes, handing focus back. Kept: dialog Enter without `focus_first` and popover face builders on an owner's opener (static builder misuse). Review: 1 round.
- P10: no change. Kept: a toast's stay above zero (one value). Review: 1 round.
- P11: a buffer at or behind the value draws none; `slide` was fixed in P05. Kept: values in 0..=1, segment and skeleton counts (one value each), builder misuse, reorder's permutation. Review: 1 round.
- P12: a featured column past the columns features none; a gauge past its scale pegs; usage parts past the total fill it and repeat hues past eight; stars above max log and fill. Kept: one-value contracts, a comparison row's shape, construction. Review: 3 rounds (round 2 met an edit that had not applied).
- P13: a directory listing's selected name gone from its entries picks none. Kept: one-value contracts; activation, folder split, single-select sections and tree rows hold by construction. Review: 1 round.
- P14: a pivot field missing from its data draws an error line; a gone group-by column leaves the table ungrouped; a merge past the sheet is skipped; each logs. Kept: row shapes of one snapshot, one-value contracts, builder misuse, construction. Review: 1 round.
- P15: chart values are finite and within ±`charts::LIMIT` (1e30), so sums and pixels stay in `f32`; scales work in halves and bound a share to ±1e4 ranges; chord and sankey size by shares; quartiles, weekday labels, flat tangents and a zero-width sankey's hover hold; notes, links, edges and waits past their lists are skipped. Review: 5 rounds, the cap; round 5's finding (points at one x) is fixed and tested, and P16's review checks it.
- P16: crossed and locked books, empty sizes, leverage and margin past their bounds, stale range and interval keys, short layouts and comparisons, growth from zero, long QR addresses and grids too fine to draw all log and show; ladders mark their middle rung and key cells by row; every system-zone time goes through `datetime`; `nice_step` stays positive. Kept: one-snapshot shapes and one-value contracts. Review: 5 rounds, passed on the fifth; it also passed P15's last fix.
- Noted for later items: `chat/params.rs` and `editor/settings.rs` pass `value.abs()` to `decimals`, which asserts a positive step.
- P17: the editor keeps every owner offset on whole characters of its text (checked on intake, carried through edits by `editor::anchors`, cleared when the whole text changes); stale selections and edit batches are dropped; a painted row whose line the text lost takes no click, drag or IME query; the sticky header reads this frame; a glob too big to compile matches nothing; a missing font advance logs. Kept: owner contracts and construction. Review: 5 rounds, passed on the fifth.
- P18: row paints and the font advance log gpui faults; a pty side caps at what ConPTY and alacritty's pixel sizes hold; a default shell past the list keeps the one before. Kept: constant regexes, captured matches, keyed rows, non-empty shells. Review: 2 rounds.
- P19: a blame that lags its code draws every line, bare where no owner fits; `git::resolve` returns `Option` for a conflict gone from the text. Kept: diff groups, merge regions, similar's pinned resolutions. Review: 2 rounds.
- P20: a hex selection off the bytes and a stack frame past the stack select none; a flame focus keeps what the profile holds; the timeline's wheel reports only finite, positive ranges from finite factors; a dump's last byte must have an address; flame paints log. Kept: disassembly patterns, `zoomed`'s contract. Review: 3 rounds.
- P21: block editor listeners painted for a block, field, cell or kind since gone log and change nothing, a dropped picture drag leaves undo alone; a visual block press or edit over text the owner replaced is dropped; nested images parse; a version or find hit past its list selects none. Review: 4 rounds.
- P22: annotations and suggestions outside their text log and stay out, picks and decisions keeping the owner's index; `accept` returns `Option`; a gone pick, active thread or followed peer logs; yours at no count comes off; a masked field clamps a stale offset, so remote cursors and selections draw. Kept: a thread's comments, presence max, one list's order, empty marks, constant role and mode tables. Review: 3 rounds.
- P23: a branch version past its total logs and shows a dash, its arrows off; a parameter outside its range logs and pegs; a passage's matches outside it log and stay out. Kept: one-value contracts (progress, scores, shapes, peaks, levels, limits, prices, capacity, format, hour), the composer's trigger rows and stream reveal by construction; `places` skips zero, so `decimals` meets no zero step. Review: 2 rounds.
- P24: a checkpoint or browser frame past its list logs and marks none; progress past its total and memory past its limit log and fill their bars, the text keeping the real counts; three `should_panic` tests became render tests. Kept: the artifact's preview-or-source builder check, one-value contracts (cost, cpu, pointer, totals and limits above zero), registry lookups over a constant table. Review: 1 round.
- P25: a verdict, ratio, style, shot, voice, variation or prompt version gone from its list logs and marks none (no picture, no Vary or Upscale, no diff); embedding points in a gone group stay out; an epoch and GPU memory past their whole log and peg. A seed field holds a seed: each field handed in is fitted, and text that is no seed clears; in `TextInput`, a fit starts a fresh undo history, undo keeps and `set_text` fits against the committed text, never an input method's unfinished one (`committed`), and a lifted mask stroke stays on its picture. Kept: one-value contracts, unique group names, one snapshot's loss lists, two to four sides, the legend's lookup. Review: 5 rounds, the cap; round 5's finding (`set_text` over a composition) is fixed and tested, and P26's review checks it.
- P26: a live position past its media logs and sits at the end (`media::played`); loaded time, a trim and chapters past the length peg or stay out; a gone device or track, or a refused trim, leaves none and clears the last; a dropped root path shows whole; subtitle times no clock holds read as none; a rail layout left no width gives no share (`scrubber::along`), so hover, press and drag skip it; waveform buckets count in `u64`, which a 32-bit web `usize` outgrows. Kept: non-zero lengths, one-value contracts, one list's uniqueness and order, lookups by construction. Review: 4 rounds; it also passed P25's last fix.
- P27: an owner's pick gone from a grid, an explorer folder or a directory listing logs and is kept as given, so it replaces a local pick and marks none (P13's listing now does the same); the columns mark a path name only while its parent holds it as a folder; moved bytes past a total log, the bar already capped; duplicate marks on no copy, or on every copy of a group, are dropped, so Remove never takes a group's last copy. Kept: one list's uniqueness, a snapshot's path and levels, items and downloads, lookups by construction, `datetime` with a constant pattern. Review: 3 rounds.
- P28: an open chat or selected member no longer listed logs and marks none. Kept: one list's uniqueness and non-emptiness, one message's receipt and replies, constant `datetime` patterns, picks from lists without disabled rows, keys from the same render. Review: 1 round.
- P29: an open mailbox, ticked label or selected mail no longer listed logs and marks none; a picked moment its zone cannot hold logs and reads as none, so the time dialog's action stays off. Kept: one list's uniqueness, day arithmetic from the owner's now, constant `datetime` patterns, the dialog's action behind its disabled button. Review: 1 round.
- P30: calendar views step only within jiff's range less a year at each end (`calendar::within`, `stepped`), so every grid around a day fits, and a step past logs and stays; recurrences end where jiff's range ends; the year view's marks stop at an event's last day; the event editor refuses moves, ends, all-day starts and switches to timed that leave jiff's range or a stale painted kind, each with a log; a backward timed draft's days are its first day. Kept: owner dates at jiff's far edges, one list's uniqueness, an event's own order, one organizer, working hours, slots, reminder offsets, constant patterns, picks that always name a value. Review: 2 rounds.
- P31: a gone assignee logs and shows nobody; a milestone past its count fills, the text real; a roadmap past jiff's range ends inclusively on its last day; the database timeline leaves out and counts tasks due before they start and counts months in `i32`, and its calendar keeps to days `calendar::within` holds; the board's drag and drop take only columns, cards and places this frame holds. Kept: one list's uniqueness, an initiative's and a milestone's own order, shares, month counts, pomodoro lengths, issue keys, lookups over the components' own tables. Review: 3 rounds.
- P32: the eight design panels take their handlers in `new` (AGENTS.md says so); edges, links, wires, links in progress, resizes, restacks, topics and whiteboard edits that name a node, port, shape, layer or topic gone from this frame log and change nothing; a wire's release fits its source as it now stands; `Topic::adding`, `removing` and `renaming` return `Option`; a mind map's selection, a palette's tool and a history position no longer held mark none. Kept: one-value contracts (alignment counts, frames, zoom, polygon sides, brush hues and type weights among the theme's tables), lookups over the panels' own option tables, button-gated expects. Review: 2 rounds.
- P33: six devtools take their handlers in `new` (contrast checker, environment selector, GraphQL explorer, WebSocket console, structure editor, index manager); `collection::moved` returns `Option` for a gone key or target or a drop into a request; a gone environment shows none chosen; an ER table without a place stays out. Kept: readings, costs, lookups over the components' own tables, button-gated expects, JSON that read writes back. Review: 1 round.
- P34: the refresh interval selector, filter bar and dashboard grid take their handlers in `new`; `tiles::arranged` and `stepped` return `Option` for a tile gone from the grid, so a landing or step after the owner removed it moves nothing; tiles without cards and cards without tiles stay out. Kept: columns, cells, intervals offered, services and uptimes, lookups from one render. Review: 1 round.
- P35: eighteen settings components take their handlers in `new`; a section or syntax theme not offered marks none; a font size or default outside its range pegs. Kept: options offered by construction, the proxy's Apply behind its read, drops that bring a file. Review: 1 round.
- P36: seventeen account components take their handlers in `new`, the email step's `on_send` with them; a busy provider, a current account or workspace, and a role no longer listed mark none and log; an invitation waits for its role to be offered; an expiry year that overflows reads as unread. Kept: quotas, providers, one current session, roles to invite as, the card behind Save. Review: 2 rounds.
- P37: seven onboarding components take their handlers in `new`, OnboardingWizard's `on_step` and `on_finish` among them; a help article no longer listed opens none; a wizard step past its last says so; a support topic no longer offered counts as none and holds Send. Kept: tasks, steps, releases and topics present, sentiments by construction, Send behind a pick. Review: 2 rounds.
- P38: Resizable, Rotatable, RovingFocus and SelectionArea take their handlers in `new`; a resize or band whose press state is gone moves nothing; Resizable keeps the owner's raw size as its seed and fits what it draws and reports to this frame's limits; ScrollSync's leader that left syncs nothing. Kept: finite angles, items and panes present. Review: 2 rounds.
- P39: no change. Kept: palette and syntax tokens named from constant tables, chart hues checked against the fixed eight (the owner's contract). Review: 1 round.
- P40: no change. Kept, as AGENTS.md says: catalogs checked against each other at construction, message keys and arguments from code, Ely's locale table, the owner's switch over its own catalogs, constant date patterns. Review: 1 round.
- P41: no change. Kept: finite and unique values, places and zooms in range (the map's own moves clamp and wrap), heat weights, routes of two places, builder misuse, tile sources and zooms, unique pins, ISO codes, the bundled world. Review: 1 round.
- P42: Captcha, CookieBanner, Poll, QrCodeScanner and Survey take their handlers in `new`; the macOS web view shows wry's build and load failures in its box and logs failed bounds, visibility and focus calls (checked by reading, as Windows cannot build that module; CI builds it); kept state fits lists that changed: cookie switches, export formats, licenses, poll picks and votes, survey sheets, import mappings, quiz and flashcard places; a picture with no frame reads no code. Kept: lists present, a question's right answer, counts per option, unit tables, places from their own choices, Vote behind a pick. Review: 2 rounds.
- P43: a playground knob painted before the owner's knobs changed sets nothing. Kept: knobs present, unique and starting in range, the preview asking by its own names (code), fixed tables in the inspector, checker and tokens. Review: 1 round.
- P44: the render surface logs an atlas that cannot free or paint a frame and skips that draw. Kept: one picture at the size asked (the host's contract). Review: 1 round.
- P45: recounted in library code, tests aside, by the same scan as the start.

## Result

| Kind | At the start | Now | What stays |
|---|---|---|---|
| `panic!` | 158 | 40 | Lookups over Ely's own tables (units, methods, statuses, colors), i18n keys from code, builder misuse, gpui's own invariants. No handler panics remain. |
| `assert!` | 477 | 404 | One value against itself or a constant, one list's own shape, an owner's static tables. |
| `assert_eq!`, `assert_ne!` | 23 | 25 | One snapshot's shape and catalogs; the two added check a host's frame. |
| `expect` | 343 | 296 | Facts by construction: lookups from one render, button-gated values, constant patterns. |
| `unreachable!` | 14 | 12 | Arms a match rules out by construction. |

Handlers a component needs are arguments of `new` in every chapter; live keys, places and indexes that a changed list leaves behind log and show none, start over or peg; outside failures (wry, gpui's atlas, an unknown time zone) log and show their state. Every item passed its review; P15 and P25 reached the five-round cap, and the next item's review passed their last fix.
