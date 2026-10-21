# The folder picker browses the hierarchy

Status: completed
Date: 2026-10-21

The owner's first dogfooding finding: the composer's folder selector
"is not letting me navigate to other levels of the folder hierarchy",
and it should take a typed path with completion. Built against
`docs/plans/2026-10-21-folder-picker.md`, approved before the build.

## What changed

- **The root cause was a borrowed cage, and a second one the plan had
  missed.** The picker reused the inspector pane's tree, which is
  contained to the launch directory by design (it serves content). An
  absolute path was pasted onto the launch dir and refused, `..` was
  refused as "outside the working directory", and ↑ was disabled at the
  launch root — so from where the picker opened, no parent level
  existed. At build time a second cage surfaced: `resolve_folder`
  refused an absolute *choice* outside the launch directory too, so the
  plan as approved would have shown the whole hierarchy and then refused
  every folder outside the launch dir. The plan was amended first, per
  the plans README, and the correction is recorded in it.
- **The picker's own listing.** `GET /api/tree?abs=1&path=<dir>`
  (`tree_abs_read`): directories under any readable absolute path —
  dot-directories included, files filtered, sorted without regard to
  case, capped like the pane's tree; a relative path, a non-directory
  and an unreadable directory are each refused by name; an empty path is
  `/`. The pane's tree is byte-for-byte unchanged, its cage included.
- **The choice may land anywhere.** `resolve_folder` accepts an absolute
  path that canonicalizes to an existing directory anywhere on the disk,
  for the choice and for the stored folder a run resolves; a relative
  path keeps the pane's rooted rule; a missing path is still refused by
  name. The tools' own cage — a run in a chosen folder cannot escape it —
  is untouched; choosing a folder anywhere is the owner's grant, the
  same grant an unrestricted run has from any launch directory.
- **The popover.** Opens at the chat's folder (else the launch folder),
  shows the absolute path, ↑ at every level and off only at `/`,
  descends at any depth. A text box, prefilled with the browsed path:
  typing refetches suggestions debounced at 200 ms, re-armed on every
  keystroke (KI-244's lesson) and dropping a listing that lands after a
  newer keystroke; the suggestions are the typed path's existing
  subfolders, or the nearest existing ancestor's subfolders matching the
  first segment the disk does not have; picking one fills the input and
  descends; Enter or "use this folder" submits the typed path. Full-bleed
  on a phone with the actions stacked.

Tests: `the_picker_lists_directories_anywhere_and_the_pane_keeps_its_cage`
(the listing's shapes and refusals, the pane's unchanged refusal, a
choice outside the launch dir accepted canonical, a missing one refused)
and the rewritten bridge test
`a_relative_escape_is_refused_and_an_absolute_folder_anywhere_is_accepted`.
Gate: fmt, clippy -D warnings, release build, `cargo test` — green.
Post-build `plan-gate --tests src`: PASS, 3/3 criteria matched to tests.

Live close, where the report came from: Playwright driving the installed
Chrome against the running face at 1280×900 and 390×844 — the input takes
focus on open; typing `.gi` suggests `.git/` and `.github/`; picking
descends to `…/fleet/.git`; ↑ returns to `…/fleet`; walking up reaches
`/` with ↑ disabled there; no console errors from the picker (the one
desktop 404 is Chrome's automatic `/favicon.ico` request, KI-282). Six
screenshots (`picker-{desktop,phone}-{open,descended,root}.png`) are in
the session's visualization directory.

Two flakes met on the way, both named and fixed rather than rerun:
KI-280's row was **corrected** — the earlier "truncated body" diagnosis
was wrong; the rank route's reply never arrived inside the helper's
three-second budget because it embeds over HTTP to the shared backend
the live face keeps busy, so the request now has its own 30 s budget;
and KI-281, a bridge test that polled the state field but asserted the
stream event, now waits on the event itself.

## What is left

- Dot-directories sort first (`.git/`, `.github/`, … before `assets/`),
  which is correct and noisy for a working-folder picker; listing them
  after the plain names is a one-line follow-up if the owner wants it.
- The document picker (`doc-pop`) still browses the pane's contained
  tree — by design, it attaches files from the workspace — and is
  untouched.
- `/favicon.ico` is not served (KI-282, minor).

Cache sweep at recap: fleet: debug 2.6 GiB → 2.6 GiB; pruned 0 files older than 7d free space on the volume: 95.0 GiB → 95.0 GiB (release targets untouched) 
