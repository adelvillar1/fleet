---
name: a-bot-name-is-free-and-its-avatar-optional
description: "owner ruling 2026-11-03 — a bot's NAME is free (rename the designer bot to "bob") and its AVATAR is optional and owner-supplied; the owner's own framing is "that doesn't change what fleet is", reopening the 2026-10-20 "no portraits" ruling"
type: feedback
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

Owner ruling 2026-11-03, verbatim: *"we also need to revisit the avatar
proposal I had, because I might want to rename my designer bot to 'bob' and
give it an avatar, and that doesn't change what fleet is."*

**The ruling's own framing is the principle: a bot's name and avatar are
presentation, not contract.** Renaming a bot and giving it a picture changes
nothing about what fleet is — the role file, its tools, its skills, its
overlay, and the log records all stand. So identity is an **optional layer
over** the contract, never a change to it.

**This reopens a settled position, and the owner is the one reopening it.**
The 2026-10-20 bot-PWA mockups ruled portraits out in exactly these words —
*"That is deliberately not a portrait… A face would misrepresent what it
names… generated faces are the most reliable tell of a designed-by-template
product"* — and the plan's D10 lists *"Portraits or illustrated avatars"* as
out of scope ([[bot-pwa-is-assistant-centric-not-work-centric]]). The
mockup's structural argument still holds and must be **answered, not
ignored**: personas are user-authored (`<home>/roles/<name>.md`, any name),
so a derived mark works for one role or forty, while an image-per-persona
stops working the moment a new role file appears.

**The honest shape is optional, owner-supplied.** An avatar when the owner
supplies one; the derived serif-initial mark when they do not. The mockup's
own escape hatch — a `mark:` frontmatter key naming a glyph from a closed
set, which the roles editor already preserves as an unknown key — is the
cheaper version of the same idea, and the roles writer's
unknown-keys-survive rule means no schema change is needed for either.

**Where the mark appears is already ruled and should not be re-litigated:**
the row, the rail, the conversation header, and the persona page — and
**never beside a message** (all five major assistants keep the transcript
avatar-free; fleet's byline is a small serif initial with a relative time).
An avatar would extend that same list, not add a transcript position.

**The plan carries it as C11** (`docs/plans/2026-11-03-huddles.md`, approved
active — [[huddle-plan-implemented-12-of-14]]): a role may carry an
owner-supplied image, rendered in the rail row, the member stack and the
persona card, and never beside a message; absent an image the derived serif
initial stands, and renaming a bot is renaming its role file. The huddle
mockups draw both forms — the derived initial and the image — in the member
stack, and the mockup README records the placement rule as one of the
screens' decisions.

**C11 is BUILT and ticked.** The convention is the **filename**, not a
frontmatter key: `<roles_dir>/<name>.png|jpg|jpeg|webp|svg`, first match
wins — so the roles editor gains no field and an image costs nothing when
absent. `/api/avatar?name=` serves the bytes from the home with the sniffed
MIME type, and a role with no image is a named 404, never a placeholder. The
mark renders on the rail row, the room head, the new-room picker and the
settings persona card. **The trap this hit:** the settings roles board
renders `roleSections`, not `rolesCard` — so a mark added to the role block
was written into **dead code** and unreachable, and only a harness case
insisted on pinning the card caught it. When adding a field to a document,
confirm the renderer that consumes it is on the path.

**Why:** the owner wants warmth and recognisability in a surface they will
read for hours, and the earlier refusal was argued from the product's
integrity rather than from the owner's taste — so when the owner revisits it,
the answer is to supply the optional layer and keep the derived mark as the
fallback, not to re-argue the refusal. **How to apply:** when the owner
renames a bot, treat it as a role-file rename (the name is the file stem and
the `@name` address) and check every place a name is matched — the roster,
the join records, the byline, the member stack. When they ask for an avatar,
offer the optional field and the derived-mark fallback together, and name
the one structural cost (a new role file has no image, so the fallback is
what keeps forty roles coherent). Related:
[[bot-pwa-persona-roster-six]] (the six-name roster and the
distinct-first-letter constraint a rename must respect),
[[huddle-surface-ideation-2026-11-03]] (the surface this identity rides),
[[a-new-plan-must-answer-the-past-plans]] (the precedent this reopens).
