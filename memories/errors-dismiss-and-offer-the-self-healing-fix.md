---
name: errors-dismiss-and-offer-the-self-healing-fix
description: "Every message can now be dismissed, and a failing check that a press can fix carries that press — mapped server-side from the check to a real self-healing endpoint, with a failed run matching its reason to the same repairs."
type: change
date: 2026-11-18
basis: confirmed-by
tier: durable
shape: [rendering, resolved]
phrases: [error messages need a way to be dismissed, give the user a fix action button, self-healing properties of fleet, repair_for, with_repairs, dismissPress]
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source:
---

The owner's 2026-11-18 ruling: "error messages need a way to be dismissed and if they are due to a misconfiguration of settings a way to use the self-healing properties of fleet to fix them, give the user a fix action button to do so."

DISMISSIBILITY. `notice(title, text, fix)` in app.js now always appends a dismiss press (`dismissPress`), and it takes a third `fix` argument. Removal uses `parentNode.removeChild`, NOT `Element.remove` — the face harness's stubbed DOM has no `Element.remove`, so the house pattern is the only portable one (found when the new harness case threw `el0.remove is not a function`). The services banner (`paintServices`) gained its own `svc-dismiss`; dismissing silences THAT warning until the services recover (`face.servicesDismissed` resets on a healthy read), so a failure after a good check speaks again rather than staying silent forever.

FIX ACTIONS. The mapping lives SERVER-side in `repair_for(check)` + `with_repairs(&mut document)`, called at the end of `doctor_read`, for the KI-300 reason: a surface that decided from the check's own words which button to draw would be a second opinion about what is fixable and would sooner or later offer a press that cannot work. Registry: credentials → `#/settings/models` (the person's key is theirs to supply, so the press opens the screen), corpus → `/api/corpus/sync`, services → `/api/repair/lanes`, layerfs → `/api/repair/layerfs`, runtime → `/api/repair/install-runtime`. A check with no entry is honestly unfixable and gets no button. A PASSING check carries no fix.

The document shape: a failing check gains `fix: {label, action, note}`. `fixAction(fix)` draws it — label in plain words, note saying what the press will do, `#route` actions navigate, `/api/...` actions POST and then `readDoctor(true)` so the person sees whether it worked. Result lands in `notice('fixed'|'the repair did not land')`.

A FAILED RUN offers a fix too, via `repairForFailure(reason)` — matched narrowly on the executor's own sentence (no key/401/credential → the key screen; service could not start/did not answer → services rebuild; layerfs/workspace → workspace repair), because offering the wrong fix costs more than offering none. The failure card is still a run's end (part of the record) and is not dismissible; only the notices are.

TESTS WITH TEETH: `a_failing_check_carries_the_press_that_fixes_it_and_a_passing_one_does_not`, `every_repair_names_a_route_the_bridge_serves` (asserts each action is a real route `include_str!("server.rs")` contains, and that no label/note says lane/face/seat/surface/rail), and the harness case `anErrorOffersItsFixAndCanBeDismissed`. The harness had NO `/api/doctor` stub — every unlisted route fell to `opts.reply || {}` — so the stub was added; without it the fix presses could not render in any case. The dismiss case was proved to have teeth by deleting the `dismissPress` call and watching both assertions fail, then restoring.

Live verification on a scratch home (no key): credentials check came back `ok=false` with `fix: {label: 'add a key', action: '#/settings/models'}`, and home/api (passing) carried no fix.
