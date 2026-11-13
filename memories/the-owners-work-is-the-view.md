---
name: the-owners-work-is-the-view
description: "owner ruling 2026-10-23 (KI-303) — the app's own background runs (corpus refresh, scheduled jobs, children) must not open a list the owner reads; they belong behind one quiet line that says how many there are and opens on request, and the counts the head shows are the owner's own work"
type: feedback
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 51
superseded-by:
valid-until:
---

The owner, after the work surface shipped: *"on sessions, can we not show the app
internal sessions like corpus refresh and other tasks that are running invisible
to the user."* The list had opened on 146 background runs — sixteen family lines
naming corpus-refresh jobs — above 90 of the owner's own conversations, so the
first thing it showed was work the owner had not started and could not act on.

**Why:** a surface that exists to answer "what needs me" cannot lead with work
that was never the owner's. The background runs keep the home alive and are real
records; they are not rows to read past. It is the same instinct as
[[the-dock-an-object-opens-needs-a-way-back]] — chrome that opens itself steals
the view.

**How to apply:** split *the owner's work* from *the app's work* by source
(the bridge already classifies `yours` vs `background`/`delegated`/`channel`),
show the owner's work as the list, and give the app's work one collapsed line
naming its count and that it is hidden; expanding shows family lines, then runs.
Do not delete or filter the records out of the API — the archive stays complete
and only the default view changes. And derive the head's count from the server's
own document (a `yours` count beside the group counts) rather than subtracting in
the page — a client that recomputes what the server classified is a second
answer (KI-300).

**Verified and pinned:** the real archive read `work 90 conversations` with one
line reading `background · 146 runs · hidden`, expanding to 16 families; the
harness's C1 case asserts the single line, no family until the toggle, the
expansion, and the line's position between *Working* and *Earlier*
([[gui-client-verified-by-a-vm-harness]]).

**ESCALATED 2026-10-29 — the one quiet line is no longer enough.** The owner:
*"system initiated sessions should be only available via a log for analysis,
never on the user screen."* The background toggle line is itself noise, and
the needs/working/archived groups filter by state not class, so system runs
in those states leak onto the screen anyway (85% of the newest-200 slice was
scheduled/subagent). System sessions' analysis home is the log: `fleet cron`,
`fleet sessions`, the calibration stream. **LANDED as 806abdd** — N0 renders
class-yours only and the KI-303 design (the one quiet line) is RETIRED: the
toggle, its families, and the state-only group filters are gone; the counts
are owner-scoped at the bridge. See [[conversation-sessions-home-plan-2026-10-29]].

**STILL LEAKING one surface over, 2026-10-30 (KI-372, `4753d13`).** The
rail learned the rule; the **bot face's `/api/inbox`** did not — it built
rows straight from `sessions_read` with no class predicate, and since
every earlier defect had hidden the inbox, nobody saw its contents until
the owner paired and asked why thirteen things "needed" him. They were
scheduled corpus-refresh runs (`kind: scheduled` → `background`), killed
mid-run by restarts → `interrupted` → the inbox's D5 mapping renders
`failed` → counted as needing the owner. The rule is now applied there by
name (the rail's own predicate), with `class` passed through on kept rows;
live 50 rows → 8. **The generalisation: the rule is a property of every
list of sessions, not of one surface — a new list inherits it by grep,
never by habit.** See [[ki-372-the-owners-screen-is-class-yours]].
