---
title: add a bot to the conversation — the door where the work happens
date: 2026-11-15
status: completed
---

# Add a bot to the conversation

Owner report (2026-11-15): *"I'm having a hard time finding where I can
add bots/personas to a conversation. That's functionality that was
available in huddles."*

## What the tree says

- **Creating a room with bots exists** — `#/huddles` → `New` → goal +
  role picks → `POST /api/huddles`. The huddles surface is **folded out of
  the rail** (U44: `FOLDED.huddles = ['workspace','threads']`), so the only
  doors to it are ⌘K (`huddles`) and the address itself.
- **Joining a role to a session exists** — `POST /api/session/join`
  (server.rs:1423, `bridge.join`), and the face presses it in exactly two
  places: a capability-gap card in the transcript (`convene <role>`,
  app.js:2339) and the persona door's catalog rows (app.js:13906).
- **Nothing offers it on an open conversation.** `#room-head` carries
  break-out / distil / archive and non-interactive member chips;
  `#conv-head` carries rename / archive / new / text size. There is no
  "add a bot" control on either.
- The **bots lens** (Settings → Advanced → Assistants) is channels and
  paired peers, not conversation membership — a likely second confusion.

So the capability never left; its front door is a gap card that only
appears when a member raises a gap. The owner's ask — add a bot/persona to
the conversation I am in — has no button.

## The shape (the owner's pick, 2026-11-15)

An **add a bot** control on the conversation:

- the conversation head gets one labelled press (`add a bot`), visible
  whenever a session is open;
- a room's members row gets the same door as a dashed chip
  (`+ add a bot`) after the members;
- both open one small door: this home's roles as pick-chips, a press that
  joins every picked role through the existing `POST /api/session/join`,
  and `create a new persona` as the escape (it opens the existing persona
  door, which joins after the write);
- joining the first bot to a plain conversation turns it into a room
  (the state's `members` land, and the room head paints itself) — no new
  verb, no second transport, no rail change (U44 stands).

Written in plain words: the door says what it does, the hint names what a
room is, and a home with no roles says so and points at the persona door
rather than offering an empty list.

## Acceptance criteria

- [x] C1: `#conv-add-bot` in the conversation head opens `#invite-door`;
  visible only with a session open; hidden on the landing and in the
  huddles surface (the room's own chip covers rooms).

  *Amended 2026-11-16 (the owner's point):* the door moved to the composer
  (`#composer-add-bot`) — the ask is where a conversation starts — and the
  landing is no longer excluded: a press there starts the conversation with
  the picked bots (the huddle creation path), where an open conversation
  receives them through the join verb. The conv head no longer carries it;
  a room's members chip still does.
- [x] C2: `#room-members` carries a trailing `+ add a bot` chip (not a
  `.member`, so the member count stays honest) that opens the same door.
- [x] C3: the door lists this home's roles (`face.settings.roles.entries`,
  unbroken ones) as toggles, joins every picked role with one press
  through `POST /api/session/join`, closes, and reloads; a home with no
  roles says so and still offers `create a new persona`.
- [x] C4: a failed join names the failure in a notice and leaves the door
  open; nothing is silently dropped.
- [x] C5: the face harness pins the door opening, the role list, the exact
  join body, and the room row's chip; the whole suite stays green.
- [x] C6: gate green (`cargo fmt --all --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo build --release`,
  `cargo test`), deployed locally, and verified live on the running face.
