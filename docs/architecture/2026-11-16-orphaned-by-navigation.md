# Orphaned by navigation — the deep dive

The owner's report (2026-11-16): *"A running session cannot be orphaned or
inaccessible."* A run kept going at the bridge while they looked elsewhere,
and no click brought its chat stream back (KI-557, fixed). This is the
sweep for the rest of the class.

## What the class is

An arrival (`showRoute`) changes the lens; a feature can fail the owner in
three ways around it:

1. **A skipped restore.** A guard decides "nothing to do" from state that
   navigation made stale — the KI-557 shape (`same || driven` skipped the
   resume while the view was detached).
2. **A surviving transient.** Something the last lens opened (a door, a
   popover, the palette, the @ picker) rides over the lens the owner
   landed on.
3. **A lost context.** Per-lens state (an open deliverable, a lesson, a
   draft, a queue) is not re-derived or re-attached on return.

## What was checked, and the verdict

| Area | Navigation-sensitive state | Verdict |
|---|---|---|
| Conversation reopen | `openWork` skipped `/api/resume` + `load`/`replay` when `same \|\| driven` | **fixed (KI-557)** — the skip now requires the view to already show the transcript (`attached`), and the threads-lens detach clears `running`/`driving`/`seat`/`stop` |
| Transient overlays | Doors (persona, invite, huddle), the goal/folder/doc popovers, the @ picker, the palette | **fixed (KI-558)** — every arrival closes them; the harness case "NAV an arrival closes the transient overlays" fails on the pre-fix code |
| Drafts | Per-session `fleet-draft-<session>` keys; `swapDraft` on every bind and detach | **safe** — the harness "each conversation keeps its own draft" pins the swap and the reload; a live probe typed on the landing, opened a conversation, and returned: the draft came back |
| Queued sends | `face.queued` items carry their own session; the drain posts to the item's session | **safe** — "REG a queued flush names its own conversation" |
| The pane | Its tabs and width; the reload reattaches it | **safe** — "RE the pane returns with the reload"; the pane is an overlay by design and arrival keeps it |
| Rooms | `openRoom` skips only `same` (the mounted view); a detached session resumes | **safe by code** — the room transcript lives in the chat panel, which only the `#/chat` detach clears, and that detach nulls the session, so the return takes the resume path |
| Desk / learning open object | `face.deskOpenDesk` / `face.learnPick`; `paintDesk`/`paintLearning` repaint them on arrival; the address form reopens by slug | **safe by code** — a fixture-less probe's refusal came from a refreshed desk document that omitted the deliverable, which a real home's read cannot do |
| Seats / approvals | Inbox rows answer by the seat's own session, not the open one | **safe** — the IBC cases |
| The archive | Lists itself; only a *needs* row bounces (the 2026-11-14 ruling) | **safe** — "C1 the list groups by attention…" |
| The tissue focus | Derived from `face.focusObject` when an address names no id | **by design** (P3/U47): a pivot re-projects the focus; it is not navigation state to clear |

## The rules this leaves

- A skipped restore must **prove the view already shows the thing**
  (`attached`: same object, visible surface, content present) — never infer
  it from a name, a flag, or a "driven" pointer.
- An arrival **clears every transient** it did not open: doors, pops, the @
  picker, the palette, the drawer.
- Per-lens open state is restored by a **paint from the state**, and the
  address form is the second way back; a lens with no address carries its
  last open object.

## Evidence

- KI-557's fix and its harness case, which fails on the pre-fix code with
  the owner's symptom (no resume; `log hidden=true`).
- KI-558's arrival close and its harness case, which fails on the pre-fix
  code (`hidden=false`).
- 1,303 checks across 188 face cases; the four-leg gate green.
