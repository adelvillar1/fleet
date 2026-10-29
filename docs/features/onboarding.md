# Onboarding

How a first-time user gets from a fresh machine to a fleet that answers —
and what each step actually proves.

## The critical path is two steps

Everything else in this document is optional. A fleet answers with the
home's layout and one model key: the judgment fails open, so the lanes,
the projects folder, the evidence token, and the strategy table can all
wait. That is deliberate — the first experience of the product must not
be a checklist.

## The GUI: two states of the home

The browser face decides which home to show from one read
(`GET /api/onboard`), and nothing else.

**The door** — shown when no routing candidate resolves a key. It is the
welcome in its first-run state: the same `.empty` frame, its eyebrow and
title swapped, and one decision.

- What fleet is for, what it needs, what happens next: three sentences,
  in that order.
- **It looks before it asks.** The door scans the environment and
  `<FLEET_HOME>/.env` for any key the roster already knows and, when one
  resolves, offers it as a single press.
- **The proof is the product.** The press makes one bounded chat call
  and shows the answer in the conversation, with a line naming what
  happened in plain words. No roster, no tiers, no checkmarks.
- **Nothing else is asked.** No passphrase, no install, no folder. The
  key lands in `<FLEET_HOME>/.env` (the dotenv resolution step the
  credentials chain already reads); `fleet install` runs only when a lane
  step needs the venvs it builds; sealing keys into the vault is an
  offered upgrade.
- **A refused send raises the door.** Typing a task with no key does not
  write a routing error into the transcript: the door takes the log's
  place and the words go back to the composer.

**The work home** — everything else, unchanged. The capabilities the
machine can still gain appear as cards under the welcome while they are
outstanding, each with one sentence of benefit, one action, and a real
"not now". A settled fleet shows no setup at all.

## The CLI

`fleet onboard` walks the same sequence as cards, resuming at the first
incomplete step (the steps' completion *is* the machine's state — the
onboarding stores nothing of its own).

```
fleet onboard                  the sequence, from the first incomplete step
fleet onboard --step <id>      one step
fleet onboard --verify         the opt-in live check (one bounded call per lane)
fleet onboard --skip <id>      decline one step; the consequence is recorded
fleet onboard --key NAME=value set the first model key (never echoed)
fleet onboard --json           the document the GUI reads
```

The CLI's keyless refusal keeps its honest list of env var names for
scripts, and now names `fleet gui` as the guided path.

## The steps

| Step | What it gives you | What it proves |
|---|---|---|
| `home` | The layout on this machine: sessions, memories, skills, config | The directories and the config scaffold exist |
| `first-llm` | Fleet can answer | A candidate resolves a key — and the press proves it with one real answer |
| `lanes` | Small specialists for narrow jobs: judging, tables, meaning, images, composing | Each lane's home resolves; `--verify` asks them live |
| `projects` | Fleet can name your repositories and read their git facts | The folder resolves and holds at least one repository |
| `github` | Claims get checked against the record | The server card declares the token name and a value resolves under it |
| `roster` | Fleet chooses models your way, within your ceiling | A confirmation is recorded, or your own routing choices are on disk |

The lanes step's order is the lane registry's own order, and the
narration reads every name, role, root, and window from the structures
the runtime itself reads (`src/lanes.rs`, `src/route.rs`,
`src/boards.rs`, `src/compaction.rs`, the MCP registry). Change a
registry and the copy follows.

## The copy rules

These are criteria, not taste, and the harness asserts them:

1. **No mechanism on the default path.** `*_API_KEY` names, `endpoint`,
   `roster`, `tier`, `vault`, `port`, and model ids appear only inside a
   step's "how this works" disclosure.
2. **One decision per screen, one primary button.**
3. **Never ask what can be discovered.** Keys, the projects folder, an
   existing `.env`: probe first, offer the answer as a press.
4. **Every refusal names the benefit, not the name.**
5. **The setup surface shrinks to nothing** when every card is settled.
6. **The proof is the product** — a real answer, a real recall, a real
   project row; never a tick, never the word "configured".

## What completion means

`GET /api/onboard`'s `completion` carries four reads, and green means all
four: doctor's credential gate, one routed turn today, one recalled fact,
and one project the tower can name. The recalled fact exists because the
sequence put it there — the onboarding remembers itself through the
corpus's own writer (`onboard::remember_onboarding`), so a fresh home
ends with one true fact rather than an empty corpus pretending
otherwise.

## Where the code lives

`src/onboard.rs` (the sequence, the derived state, the narration, the
probes), `src/cli.rs` + `src/main.rs` (`fleet onboard`), and
`src/gui/bridge.rs` (`/api/onboard`, the key press and its bounded proof,
the `needs_setup` event that raises the door).
