---
name: no-form-asks-for-a-value-fleet-already-has
description: "owner rule 2026-10-20 for every configuration form — if fleet already knows a model name, key env, path, or provider, the field must offer it as a select, a combobox, or an ordered chip list; typing the value again is the failure"
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

While planning the router settings surface, the owner stopped the plan with:
"Wherever a user is asked to type something, we need to make sure that if we
already have, for example a model name, then the user shouldn't be forced to
type it again. I should be selectable from a dropdown or a multiple selection
for things like models assigned to the different workloads or profiles."

**Why:** a config form that asks an owner to retype `stepfun/step-5-preview`
is a form that can accept a typo the router will then fail over from at run
time. The catalog is also how the surface stays honest: a value that is *not*
in the catalog (a model the roster dropped, a hand-written path) is exactly
what should be visible and flagged, not what a free-text box hides.

**How to apply — the five shapes from the 2026-10-20 settings build
([[fleet-config-surface-is-the-settings-page]]):**
- closed set → `<select>`, seeded with the current value; an unset value gets
  its own `—` option so the browser cannot silently promote the first option;
- known-but-open set (model ids, key env names, base URLs, paths, voices) →
  combobox (`<input list>` + `<datalist>` from the catalog), free text only
  for a genuinely new value;
- ordered list (a workload's fallbacks, sys1's default chain) → chip editor
  with move-up/move-down and an "add custom" door;
- unordered set (a role's tools/skills/shapes) → multi-chip editor;
- free text only for what is new: description, overlay, cron expression, a
  new provider id, a secret.

One catalog read feeds every picker (`settings::catalog` → `/api/settings`):
roster models **plus** every value the current routing file names (a lost
model stays as a `known:false` custom entry rather than being dropped), env
names with presence/source and never a value, paths, tools, skills, shapes.
When a pick list would be empty, hide the picker and keep the free-text door —
an empty dropdown is a control that can never add anything. Related:
[[fleet-routes-through-its-own-router]], [[fleet-vault-credentials]].
