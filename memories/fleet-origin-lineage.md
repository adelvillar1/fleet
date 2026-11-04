---
name: fleet-origin-lineage
description: "The owner's stated lineage behind fleet (2026-10-23, corrected the same evening) — eight harnesses tried first; the CODE base is the DeepSeek Harness (well regarded, open, low baggage), forked as spec/integration lab then natively ported to Rust (criterion C5: never reintroduce Node, a checkout dependency, or a dsh launch path); the INTELLIGENCE layers are lanes → blend1 → dev-decisions → agnostic-router-kit → zcode-router-kit (the guest edition whose strain made fleet's founding decision evidence-based); Z…"
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

Owner statement 2026-10-23, the origin story behind fleet — and its
prologue, stated the same day: **"I've been using Claude Code, Kimi Code,
Codex, OpenCode, Qwen Code, Hermes Agent, Zcode, and each one had good
things and things that were not for me. So I decided to do the crazy
thing, spend a few billion tokens to get the tool that I need."** The
eight-harness trial was the research phase; fleet's opinionated rules are
that complaint list made structural (e.g. ZCode's all-or-nothing config
validation → fleet's validate-before-rename writer; foreign configs'
plaintext credentials → env-names-only; the hated turn cap → the owned
budget; hang-without-control → the control file). The token spend was
deliberate capex — judgment as the scarce input, tokens as labor.

"blend1 is just a
layer of coordinated use of all the lanes, then there is dev-decisions, and
the agnostic-router-kit and zcode-router-kit — that's where the fleet idea
came from. Why do a franken-harness when I can build my own to use it
properly." Each layer stayed a separate repo with its own contract rather
than being collapsed:

- **The lanes** — eventually configurable BY USERS; that is the standing intent.
- **blend1** — the coordination layer over the lanes (see
  [[blend1-lane-assessment-2026-10-23]] for its portable three-transport shape).
- **dev-decisions** — the judgment/gate layer (plan-gate, docs-gate,
  calibration, disposition), harness-neutral.
- **agnostic-router-kit** — the ENGINE edition: owns `lib/workflow/` (the
  zero-runtime-dep workflow plane), "depends on no harness anywhere, no
  harness is read, nothing about one is assumed"; the `auto` model id carries
  judged workload, execution style, failover, metering.
- **zcode-router-kit** — the consumer/guest edition, and the visible strain
  that justified fleet: `check:port` exists because "the installed runtime
  beside the router can go stale silently, and that is what the live service
  executes"; `kit apply` renders into someone else's config schema; drift
  guards everywhere. Guest-in-a-harness costs are intrinsic, not sloppiness.
- **fleet** — the native edition: the harness designed around the plane
  instead of a plane adapted into a harness. The lineage lives natively — the
  kit's judge became the router's sys1_classify (judging workload AND the
  execution head), the lanes became model-callable tools, dev-decisions'
  gates became the calibration stream and eval corpus, and the workflow
  plane's journal became hard rule 5 (model-visible ⟺ logged, now pinned
  byte-equal).

**CORRECTION, same evening** — when I attributed the base to ZCode's
ecosystem, the owner fixed it: **"actually the base for fleet is DeepSeek
Harness, a well regarded and open harness with not too much baggage."**
The repo record confirms the shape: first commit "a native Rust launcher
distribution around the intact harness"; the fork was the fleet profile /
spec-integration lab ("never a shipping competitor"); the plans were
copied byte-identical from the fork; and founding criterion C5 is
"Native port, not a fork — never reintroduce Node, a checkout dependency,
or a dsh launch path." So the division of credit is: the DeepSeek Harness
gave the SKELETON and the methodology (plans/recaps/ledger, inherited
intact); the lanes and kits gave the INTELLIGENCE; ZCode gave the felt
sense of what to keep and what to fix. Named after the DeepSeek API, the
harness outgrew it — DeepSeek is now the funded failover tail of every
roster walk. Choosing that base was itself the enterprise move: evaluate
the foundation, fork-as-lab, port natively on one criterion.

**The ZCode nuance, same conversation**: "Zcode is close to my needs and
that's why I built the zcode-router-kit, but then the franken-harness
started to take shape. I enjoy zcode, but it's just not quite there" —
enjoyment → kit → franken-harness onset is the causal chain, and "not
quite there" is structural, not a version complaint: you cannot own the
loop from outside it. And the negative space of the whole lineage is
named: **the anti-goal is Hermes Agent** — see
[[fleet-anti-hermes-constitution]].

**Progressive elaboration is deliberate, not unfinished**: the owner will
package a seamless install "eventually," and deferring is correct because
packaging freezes contracts while the contracts are still sharpening (the
execution head was rescued from `parse_workload` barely a day before the
swarm plan). Because nothing collapsed prematurely — blend1 pinned by
VENDOR.json, the plane a `file:` dependency that cannot drift from the
engine — fleet could consume instead of reimplement, which is why it took
days instead of months.

**Why:** this lineage explains design decisions that look odd from inside
fleet alone (why the router IS a judgment, why lanes are vendored children,
why the kits are kept as separate repos, why the methodology worked from
week one — it was inherited intact from the DeepSeek Harness base) and it
supersedes nothing — it enriches [[fleet-routes-through-its-own-router]],
which already recorded that the kit is the precursor fleet superseded.

**How to apply:** when the owner speaks of "the lanes," "the plane," "the
kits," or "the house pattern," these are the referents; treat cross-repo
contracts (VENDOR pins, file: resolution, single-writer renders) as
load-bearing and do not propose collapsing them into fleet. When packaging
or install work is proposed, check with the owner first — the deferral is a
ruling, and the roster/`kit apply` single-writer pattern is the shape the
future seamless install extends. Related: [[owner-is-a-team-of-one]],
[[fleet-judgment-beats-spawn-per-row]], [[blend1-is-the-latency-triad]].
