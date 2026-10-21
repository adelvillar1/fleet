---
status: active
created: 2026-10-21
updated: 2026-10-21
slug: folder-picker-browses-the-hierarchy
---

# The folder picker browses the hierarchy

## Context

Dogfooding finding, 2026-10-21: the composer's folder selector "is not
letting me navigate to other levels of the folder hierarchy", and it
should take typed input with completion. Root cause, with evidence:

- Descending *inside* the launch folder works at every depth
  (`/api/tree?path=src/gui` and `docs/plans` both answer on the live
  face). The break is everything else:
- an absolute path is treated as a *relative* one —
  `/Users/alejandrodelvillar/Projects` is refused as
  `<launch-dir>/Users/alejandrodelvillar/Projects: No such file or
  directory`;
- a `..` walk is refused by `pane_path`'s lexical check ("outside the
  working directory", `src/gui/bridge.rs:2558`);
- and the popover's ↑ button is disabled at the launch root
  (`up.disabled = !folderBrowse.rel`, `src/gui/assets/app.js:5075`), so
  from where the picker opens, no parent level is reachable.

The feature and the picker disagree about the universe of folders: a
chat's working folder may be any folder on the disk — `/api/folder`
validates absolute paths and the run works there (that is the point of
the working-folder slice, `f554a59`) — but the picker can only browse
inward from the launch directory. The picker's confinement is borrowed
from the inspector's pane tree, where launch-dir containment is
deliberate (an SSRF-shaped guard on a content-serving route). The picker
serves folder *names* to a chooser, not content: it needs the hierarchy,
not the cage.

## Approach

The pane's tree stays exactly as it is. The picker gets an absolute
listing (`/api/tree` grows an `abs=1` mode: directories only, any
readable absolute path, refusals named), the popover browses with it
(opening at the chat's current folder, ↑ enabled everywhere but `/`), and
a text box takes a typed path with suggestions drawn from the same
listing — type a prefix, see its existing subdirectories, pick one or
press Use. Committing a folder goes through the existing
`POST /api/folder` validation unchanged, so a typed path that does not
exist is refused by name, in the transcript, as today.

Populated mockup (the popover as it will read, desktop width):

```
 run bar:  [ Ask ▾ ]  [ ⌂ fleet ]  [ folder ▾ ]  [ Ask ▾ ] [ run ]
                                  │
                                  ▼
      ┌ working folder ──────────────────────────────────────┐
      │ /Users/alejandrodelvillar/Projects/fleet             │ ← where you are
      │ ┌──────────────────────────────────────────────────┐ │
      │ │ /Users/alejandrodelvillar/Projects/fleet/▏       │ │ ← type a path
      │ └──────────────────────────────────────────────────┘ │
      │   blend1/                      ← its existing        │
      │   cruiserintelligence/           subfolders, pick    │
      │   second-brain/                  to descend          │
      │ ──────────────────────────────────────────────────── │
      │ ↑ up        (enabled at every level, off only at /)  │
      │ [ Use this folder ]      [ Reset to launch folder ]  │
      └──────────────────────────────────────────────────────┘
```

Same popover at phone width: full-bleed, the input and the two buttons
stack; the suggestion rows stay one line each.

## Acceptance criteria

- [ ] **C0 — an absolute listing exists and the pane's cage is untouched.** `GET /api/tree?abs=1&path=<dir>` answers `{type:"tree", abs:true, path, entries:[{name,kind:"dir"}]}` for any readable absolute directory — dot-directories included, `.`/`..` never listed, files filtered out; a non-directory and an unreadable one are named refusals, and an empty `path` with `abs=1` is the filesystem root. New tests in `src/gui/server.rs` pin it: an `abs=1` listing of a scratch directory lists its subdirectories (dot-directory included) and no files, a file path is refused as not a directory, and the relative mode's `outside the working directory` refusal is unchanged — the existing pane-tree tests stay green untouched.
- [ ] **C1 — the popover browses the whole hierarchy.** It opens at the chat's current folder when one is set, else the launch folder; the path line always shows the absolute path; ↑ is enabled at every level and disabled only at `/`; descending works at any depth through `abs=1`; "Use this folder" submits the browsed absolute path and "Reset" clears, both through the unchanged `POST /api/folder`.
- [ ] **C2 — the text box completes from the filesystem.** An input in the popover, prefilled with the browsed path; typing refetches suggestions debounced at 200 ms (the KI-244 lesson: the debounce arms on every keystroke, not the first word); the suggestions are the typed path's existing subdirectories, or — when the typed path does not exist — the nearest existing ancestor's subdirectories matching the typed last segment; picking one fills the input and descends; Use submits the typed path, and a nonexistent one meets the existing named refusal as a transcript line.

## Files to be touched

`src/gui/bridge.rs` (the `abs` mode beside `tree_read`),
`src/gui/server.rs` (the query, tests), `src/gui/assets/app.js`
(popover: absolute browsing, the input, the debounced completion),
`src/gui/assets/index.html` (the input), `src/gui/assets/app.css`
(styling at both widths).

## Out of scope

Changing `pane_path` or the pane tree's containment (that cage is
deliberate for a content route). Changing `POST /api/folder`'s
validation. Showing files in the picker. A folder bookmark/recents list.
Any change to the run bar's other controls.

## Verification

The gate — `cargo fmt --all --check`, `cargo clippy --all-targets -- -D
warnings`, `cargo build --release`, `cargo test` — exits 0 four times,
read by exit code, with C0's new server tests in the suite. Then the
close is at the live face, where the report came from — in a real
browser, at desktop width (≥1000 px) and at phone width (≤430 px): (a)
the input is present and receives focus when the popover opens, (b)
typing the launch folder's own first subfolder lists it as a suggestion
and picking it descends one level, (c) ↑ from a nested level returns to
its parent and shows the parent's absolute path. Four screenshots (two
widths × open and descended) go in the session's notes; the recap quotes
them.

## Linked artifacts

`f554a59` + `7e2e3c9` (the working-folder slice this picker belongs to).
`docs/recaps/2026-10-21-working-folder.md`. The owner's dogfooding
report, 2026-10-21. KI-244 (the debounce lesson C2 builds on).
