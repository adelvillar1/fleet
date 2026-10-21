---
status: completed
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
chat's working folder is meant to be any folder on the disk — but the
picker can only browse inward from the launch directory. The picker's
confinement is borrowed from the inspector's pane tree, where launch-dir
containment is deliberate (an SSRF-shaped guard on a content-serving
route). The picker serves folder *names* to a chooser, not content: it
needs the hierarchy, not the cage.

**Correction found at build time (2026-10-21).** The plan as approved
assumed `POST /api/folder` already accepted any absolute folder. It does
not: `resolve_folder` (`src/gui/bridge.rs:2600`) refuses an absolute path
that canonicalizes outside the launch directory, and the same function
resolves a stored folder when a run is accepted. Built as approved, the
picker would show the whole hierarchy and then refuse every folder
outside the launch dir — a worse defect than the one reported. The
approach and C1 below are amended to lift that containment for the
*choice*: the tools' own cage is untouched (a run in a chosen folder
still cannot escape it, `Tool::execute_in`), and choosing a folder
anywhere is the owner's grant, the same grant an unrestricted run has
from any launch directory.

## Approach

The pane's tree stays exactly as it is. The picker gets an absolute
listing (`/api/tree` grows an `abs=1` mode: directories only, any
readable absolute path, refusals named), the popover browses with it
(opening at the chat's current folder, ↑ enabled everywhere but `/`), and
a text box takes a typed path with suggestions drawn from the same
listing — type a prefix, see its existing subdirectories, pick one or
press Use. Committing a folder goes through `POST /api/folder`, whose
`resolve_folder` now accepts an absolute path that canonicalizes to an
existing directory anywhere (the pane's rooted *relative* form keeps its
rule); a typed path that does not exist is refused by name, in the
transcript, as today.

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

- [x] **C0 — an absolute listing exists and the pane's cage is untouched.** `GET /api/tree?abs=1&path=<dir>` answers `{type:"tree", abs:true, path, entries:[{name,kind:"dir"}]}` for any readable absolute directory — dot-directories included, `.`/`..` never listed, files filtered out; a non-directory and an unreadable one are named refusals, and an empty `path` with `abs=1` is the filesystem root. New tests in `src/gui/server.rs` pin it: an `abs=1` listing of a scratch directory lists its subdirectories (dot-directory included) and no files, a file path is refused as not a directory, and the relative mode's `outside the working directory` refusal is unchanged — the existing pane-tree tests stay green untouched.
- [x] **C1 — the popover browses the whole hierarchy, and the choice may land anywhere in it.** It opens at the chat's current folder when one is set, else the launch folder; the path line always shows the absolute path; ↑ is enabled at every level and disabled only at `/`; descending works at any depth through `abs=1`; "Use this folder" submits the browsed absolute path and "Reset" clears, through `POST /api/folder`. *(Amended at build time:)* `resolve_folder` accepts an absolute path that canonicalizes to an existing directory anywhere on the disk, for the choice and for the stored folder a run resolves; a relative path keeps the pane's rooted rule; a missing path is still refused by name. Pinned by a server test that sets a folder outside the launch directory and reads it back canonical, beside the existing refusal tests for `..` and missing paths.
- [x] **C2 — the text box completes from the filesystem.** An input in the popover, prefilled with the browsed path; typing refetches suggestions debounced at 200 ms (the KI-244 lesson: the debounce arms on every keystroke, not the first word); the suggestions are the typed path's existing subdirectories, or — when the typed path does not exist — the nearest existing ancestor's subdirectories matching the typed last segment; picking one fills the input and descends; Use submits the typed path, and a nonexistent one meets the existing named refusal as a transcript line.

## Files to be touched

`src/gui/bridge.rs` (the `abs` mode beside `tree_read`),
`src/gui/server.rs` (the query, tests), `src/gui/assets/app.js`
(popover: absolute browsing, the input, the debounced completion),
`src/gui/assets/index.html` (the input), `src/gui/assets/app.css`
(styling at both widths).

## Out of scope

Changing `pane_path` or the pane tree's containment (that cage is
deliberate for a content route). Changing the tools' own cage
(`Tool::execute_in`: a run in a chosen folder cannot escape it). Showing
files in the picker. A folder bookmark/recents list. Any change to the
run bar's other controls.

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
