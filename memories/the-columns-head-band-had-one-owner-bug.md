---
name: the-columns-head-band-had-one-owner-bug
description: "The band over the chat column was one line in paintRoomHead un-hiding the huddles invitation whenever the session was not a room, on every paint and every surface; the fix is one-owner-per-element painters that guard their own surface, and a room is decided by membership, not by the panel."
type: project
date: 2026-11-19
basis: inferred
tier: durable
shape: [rendering, resolved]
phrases: [a band that sometimes displays text from other pages, room-empty hidden = !!room, paintRoomHead raises the invitation, one owner per element, the invitation was visible at boot]
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source:
---

The owner's report: "on the top of the chat area there is a band that sometimes displays text from other pages, for instance if you go to the huddle page the text from that page shows even if you are in the conversations page."

The band is the conversation column's head strip. The cause was ONE line in `paintRoomHead` (src/gui/assets/app.js):

    if (el.roomEmpty) el.roomEmpty.hidden = !!room;

That UN-hides the huddles invitation whenever the loaded session is not a room — on every paint, on every surface. The state read runs on every event, so any conversation could put "Start a huddle. A working session you share with one or more bots." at the top of its column. It measured visible at BOOT, before any navigation, which is what disproved the first hypothesis (a stale head from the previous surface).

The fix is an ownership rule, not a patch: each element at the top of the column has exactly one painter, and the painter guards its own surface.
- `paintRoomHead` may only ever STAND THE INVITATION DOWN (it used to raise it). A room is decided by membership — `face.session && members.length` — which is enough, because a plain conversation has none. Do NOT gate the room head on `face.panel === 'huddles'`: `#/sessions/<room>` is the conversations panel, and the room head legitimately renders there (three face cases caught that over-restriction).
- `paintRoomEmpty` owns the invitation and returns early when the surface is not huddles. A deferred continuation inside `openRoom` (its `load().then(...)` rejection path) resolves after the owner may have moved on; it can no longer paint huddles copy over another surface.
- Arriving at a lens clears the previous lens's head, so no surface has to remember to clean up after the one before it.

Pinned by the face case `theColumnssHeadsNeverBorrowAnotherSurfacesText` (registered as 'BAND the column's heads never borrow another surface's text'), which walks the report's exact sequence plus a repaint; restoring the old line turns three of its checks red.

Two measurement traps worth keeping: the face stamps `app.js?v=<hash>` computed at server startup and does NOT recompute on reload, so an edited asset can be served as a 304 and a live probe silently measures the OLD code — restart the face, or check the hash, before believing a measurement. And `obscura`'s page context has no `DataTransfer`/`DragEvent`, for anything that needs drag synthesis.
