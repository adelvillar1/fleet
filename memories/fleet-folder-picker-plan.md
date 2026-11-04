---
name: fleet-folder-picker-plan
description: "2026-10-21 first dogfooding finding — BUILT, LIVE-VERIFIED and PUSHED as c40319d — the picker browses the whole hierarchy with a typed path; build-time correction the owner was told about: the feature itself was caged too (resolve_folder), plan amended before building"
type: project
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The owner's first dogfooding finding (2026-10-21, unpinned pass): the composer's
folder selector "is not letting me navigate to other levels of the folder
hierarchy", and it should take typed input with completion. SHIPPED same day as
`c40319d` (pushed; plan `docs/plans/2026-10-21-folder-picker.md` status
completed; recap `docs/recaps/2026-10-21-folder-picker.md`), face restarted onto
it, live-verified through Playwright (`channel: "chrome"`) at 1280×900 and
390×844 — focus on open, `.gi` → `.git/` + `.github/`, descend, ↑ to the
parent, walk to `/` with ↑ disabled there; six screenshots in the session
visualization directory.

**The build-time correction the owner was told about plainly:** the approved
plan assumed `POST /api/folder` already accepted any absolute folder. It did
not — `resolve_folder` (`src/gui/bridge.rs:2600`) refused an absolute *choice*
outside the launch directory, so built as approved the picker would have shown
the whole disk and then refused everything outside the launch dir. Amended per
the plans README BEFORE building (C1 + out-of-scope, correction recorded in the
plan), then disclosed in the delivery. `resolve_folder` now accepts any
absolute path that canonicalizes to an existing directory anywhere; a relative
path keeps the pane's rooted rule; the tools' own cage (`Tool::execute_in`) is
untouched — the pane tree's containment is byte-for-byte unchanged.

What shipped: `GET /api/tree?abs=1&path=<dir>` (`tree_abs_read` — directories
only, dot-dirs included, files filtered, named refusals, empty path = `/`);
the popover browses absolute paths, ↑ enabled everywhere but `/`, opens at the
chat's folder else the launch folder; a text box (folder-pop-input) prefilled
with the browsed path, suggestions debounced 200 ms re-armed per keystroke
(KI-244) and dropping stale listings, climbing to the nearest existing
ancestor when the typed path does not exist; Enter/Use submits the typed path;
full-bleed on a phone with stacked actions.

**Why:** the next session must know the choice-may-land-anywhere ruling and
that the pane's cage is deliberate and untouched — and that a plan whose
premise was wrong about the code gets amended first, disclosed, not silently
built.

**How to apply:** gate-method note earned here — a criterion that only says
"the gate is green" fails the gate's coverage judge (fold the tests into the
criterion they pin, keep the gate in Verification); run plan-gate WITHOUT
`--tests` pre-build and WITH `--tests src` post-build. Follow-ups named:
dot-directories sort before plain names (noisy for a picker — one-line change
if the owner wants it); `doc-pop` still browses the contained tree by design;
`/favicon.ico` is not served (KI-282). Related:
[[fleet-chat-working-folder-plan]], [[plan-is-a-file-not-prose]],
[[fleet-recap-ends-with-cache-sweep]].
