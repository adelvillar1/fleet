# The plans built: navigation, the probe stamp, the preview, the pane, the onboarding

**2026-10-29 (second session)** · five plans implemented and closed in
one sitting — the conversation/sessions home (N0–N3), the probe stamp,
the preview formats, the right pane, and the guided onboarding — plus
the turn-feedback icons and the queue's verbs. Fifteen commits, all
gates green with exit codes read directly, the face harness from 351 to
**375 checks across 41 cases**, three Chromium probes measuring what a
DOM stub cannot, and one audit that found two criteria checked early
(both fixed the same day, the plan's record corrected rather than left
green). Ledger checker green: **366 rows, 31 open**.

The earlier session's record is
[2026-10-29-face-icons-resume-closeout.md](2026-10-29-face-icons-resume-closeout.md) —
the icons, the resume close-out, and the navigation *audit*. This file
is what happened when the owner said `go`.

## The conversation home: the rail is the owner's (N0–N3)

The owner's four frictions, designed in the draft plan and built whole:

- **N0 — the rail renders the owner's work only.** Every group filters
  through one predicate (`railRow`: class-yours and not a shell); the
  "background · 169 runs · hidden" line, its toggle and the family code
  died; the bridge's counts became owner-scoped (a head cannot count a
  row the list will not render — KI-300's law), with the all-class
  background count kept as the document's own fact; the welcome's
  "pick up where you left off" filtered at both sources.
- **N1 — the home is always reachable.** Arriving at `#/chat` detaches
  (draft swapped, pointer cleared, welcome shown, nothing server-side);
  the stream's guard skips named events for a detached face while the
  `session` adoption event still lands, so a send from the home follows
  its own conversation.
- **N2 — New mints nothing until words exist.** New and `/clear` walk
  home with a pending intent; the session is created at the first send;
  the 34 abandoned shells stopped rendering and stop being created.
- **N3 — the rail's badge came alive.** The dead count spans carry needs
  + working from every surface, hidden when quiet.

Two defects the harness caught mid-build, both real: my predicate had
shadowed the message renderer's `ownerRow` (owner messages vanished
until renamed), and a detached face would have ignored its own run's
events. Also closed a harness fidelity gap: the fake window now fires
`hashchange` on assignment like a real browser, so programmatic
navigation walks the router.

## The probe stamp (KI-366)

The archive's verification traffic — eval-scenario replays, face
checks — carried no kind and rendered as the owner's conversations (a
third of the rail's newest-200 slice). One word at every layer:
**probe**. `fleet run --probe` births `kind: probe`; `work_class`
classifies it; `fleet sessions --probe <id>` marks legacy logs in a
`sessions-probes.json` sidecar (append-only holds — no log re-stamped).
**74 legacy logs marked**; the borderline remainder named, not marked
("session warmup" and three genuine owner questions stay visible).

## The preview renders the common documents

Research first (Claude artifacts, Canvas, VS Code/Cursor, the library
landscape), then the fence and seven formats:

- **The fence**: `/api/preview` serves allowlisted bytes through the
  text read's own containment (credential refusal, 32 MB cap),
  server-derived content types, `nosniff`, `Content-Disposition:
  attachment` on everything — a navigation to the route can never
  execute a document. The sandboxed document frame exists; the deck
  renders inside it.
- **The formats**: markdown (tables + DOMPurify), CSV/TSV (quote-aware
  tables), JSON/NDJSON (trees), images and SVG as `<img>` (scripts
  inert), PDF through vendored PDF.js with scripting disabled, XLSX
  hand-rolled (a zip walk over `DecompressionStream` + `DOMParser`,
  sheet tabs, values only), DOCX through docx-preview inside the frame,
  PPTX as a per-slide outline (the honest partial, the limitation
  named), drawio as XML with its `.svg` companion.
- **Provenance**: five libraries vendored with `VENDOR.json` (versions,
  upstreams, SHA-256s, licenses, reasons) and `THIRD-PARTY-NOTICES`;
  ~1.9 MB lazy-loaded, the 28 kB sanitizer always on.
- **The verifier's finding closed**: D4 and D5 had only fake-DOM
  evidence, so an e2e probe now drives a scratch gui in real Chromium —
  the hand-rolled reader inflates a real two-sheet xlsx (shared strings
  resolved, every value), and docx-preview reports **rendered** from
  inside the sandboxed frame via a render report the frame now posts.

## The right pane: one surface, one entry, the owner's width, a browser

The two topbar icons opened two versions of the right thing because
both surfaces shared grid column 3. Now: one pane, four tabs (**run** —
the dock's process, session and trust cards relocated; **files**;
**preview**; **web**), the dock retired with all its machinery, and:

- **The owner's width**: a grip on the left edge, 300 px floor to the
  stage's cap, persisted, double-click resets, arrow keys step it,
  `role="separator"`. Measured in Chromium: 440/720/300 honored.
- **The browser**: `/api/web/frame` reads the site's own
  `x-frame-options` and CSP (headers only), framable sites render in a
  sandboxed frame that cannot navigate the pane away, refusals name the
  site's words with the prose reader and the system browser as doors.

The research (Claude, Cursor, Copilot, Cline, Canvas, Devin, Cascade)
confirmed the canon — process visibility, files, live preview, resize,
steering — and named fleet's two real gaps, recorded as follow-ups: a
**Changes** section and a **plan card** (P3/P4 in the plan).

## The onboarding: the door, then the work

The owner's plan, examined against the code (six findings: the install
dependency, the GUI never specified, the completion probe needing a
first fact, the chain wording, and — from the deeper look — that a
fresh home had **no first-run at all**, plus that the design had no
house language), then built:

- **The door**: when no candidate resolves, the welcome becomes its
  first-run state — `fleet · first run`, "One account, and fleet can
  think.", the benefit sentence, one decision. It looks before it asks
  (an account already on the machine is one press), and the proof is
  the product: one bounded call whose answer lands in the conversation.
  A send with no key raises the door inline (`UiEvent::NeedsSetup`)
  instead of writing `routing config:` to the transcript. The critical
  path is one key and no lanes — the vault is an offered upgrade,
  install waits for a lane step.
- **The cards**: the capabilities still outstanding, each with its
  benefit, a **visible proof** (a derived reading: "6 of 6 specialists
  are installed", "30 repositories under …", "8 choices in force ·
  quick … · hard …"), one action and a real "not now". A settled fleet
  shows no setup.
- **The strategy preview**: the roster step ends in a screen — the chain
  as built with each candidate's state and billing, the last resort
  named, quick vs hard, the ceilings in force, the window fallback, then
  "this looks right" beside "not now" and "change it".
- **The sequence**: `fleet onboard` runs the same steps, resuming at the
  first incomplete one because the steps' completion *is* the machine's
  state; decisions ride the calibration stream's existing `lifecycle`
  rows (a narrow typed reader beside the filtered `OPS` set). Every
  narrated fact is derived from the registries.
- **Design language**: no new chrome — `.empty`, `.offer`, `.card`,
  `.method`, `.foot-btn`, the composer's own field; the harness asserts
  no new token and no literal colour. The screenshot was the check that
  mattered: the door rendered but the power-user furniture still showed
  beneath it, so a state attribute on the welcome's frame now stands it
  down (and the same for the preview).

### Proven by hand

`POST /api/onboard {"op":"prove"}` against the live face: `proved:
true`, the model's own words back — one bounded call through the
router. The criterion the plan deferred to a hand proof.

### The audit

The completion audit found two criteria checked before the behaviour
existed (no proof element on the cards; the strategy action still
jumped to settings). Both landed the same day with harness checks of
their own, and the plan's record now carries the correction rather than
a quiet green. It also surfaced a real bug: the discovered-key press
was posting a *key write* with a placeholder, now a `prove` op that
touches nothing.

## Commits

| Commit | What |
|---|---|
| `364c09c` | the resume record + KI-365 |
| `a563b7e` | turn controls as icons, the composer's two verbs |
| `9a11b7d` | docs: the resumed word, the navigation draft, the recap |
| `806abdd` | the rail is the owner's — N0–N3 |
| `5b55768` | the probe stamp — KI-366, 74 logs marked |
| `1cda5a3` | composer tooltips, the queue's three icon verbs |
| `f8ce79b` | the queued text never outgrows the composer |
| `d20c257` | the right pane — one surface, the width, the browser |
| `2c59652` | the preview's fence and seven formats, five libraries vendored |
| `551dcaf` | the docx frame's render report, the e2e probe |
| `e838241` | the guided onboarding — the door, the cards, the sequence |
| `8419da3` | the strategy preview and a visible proof on every card |

(Two docs commits from another thread — `8aa9ea8`, `3a8915a` — landed
in the same window and are not this session's work.)

## Files changed

- **Rust**: `src/onboard.rs` + tests (new), `src/session.rs` (the resume
  record), `src/cli.rs` / `src/main.rs` (`onboard`), `src/gui/bridge.rs`
  (work-state filter, counts, the onboarding document and ops, the
  preview read, the framing verdict, the door event), `src/gui/server.rs`
  (the new routes and the vendored assets), `src/web.rs` (`framable`),
  `src/calib.rs` (the lifecycle reader, `ReadRow`'s kind/name),
  `src/settings.rs` + `src/mcp_registry.rs` + `src/lanes.rs` (the three
  accessors the narration reads), `src/agent.rs` (`NeedsSetup`),
  `src/tui/app.rs` (the event's terminal answer).
- **Face**: `index.html`, `app.js`, `app.css` (the icons, the queue
  verbs, the tooltips, the rail filter, the home/door states, the pane's
  tabs and grip, the preview dispatch and renderers, the onboarding
  cards and preview), `assets/vendor/` (five libraries, the manifest,
  the notices).
- **Tests**: `tests/face/harness.mjs` (351 → 375 checks across 41
  cases), `tests/integration.rs` (three onboarding tests over a scratch
  home).
- **Docs**: `docs/features/onboarding.md` (new), `docs/features/gui.md`
  (the pane, the preview formats, the icons, the rail), the functional
  spec (the `resumed` word, `--probe`, the rail's rule), five plans
  (four completed, one corrected), KI-365 and KI-366, the README, the
  harness orientation.
- **Probes** (kept, not committed): `tmp/pdf-probe.mjs`,
  `tmp/preview-probe.mjs`, `tmp/queue-overflow-probe.mjs`,
  `tmp/pane-width-probe.mjs`, and two screenshot scripts.

## Doc updates — applied

`docs/features/gui.md` (pane, preview, icons, rail, queue, tooltips),
`FUNCTIONAL-SPECIFICATIONS.md` (the `resumed` state word, `--probe`,
`--probe <id>`, the rail's owner-only rule, the pane's tabs),
`docs/features/onboarding.md` (new), `README.md` (the onboarding in
"What ships", the quick start), `docs/harness-orientation.md` (the first
run). `TECHNICAL-DOCUMENTATION.md` untouched — its routing and
verification summaries still hold.

## Open questions / next steps

- **The right pane's two named gaps**: **P3 the Changes section** (the
  review surface's hunks and accepts, pane-resident) and **P4 the plan
  card** (plan mode's document visible while a run works under it).
- **The ~59 verification runs still in the archive** that were not
  unambiguous enough to mark — named in KI-366, the owner's call.
- **`probe` is not yet the harness's own habit**: the face's own checks
  drive through a scratch home, but a manual verification run still
  mints a plain session unless someone passes `--probe`.
- **The `sys1`/`sem1`/`blend1` repointed estate** (from the morning's
  prior sessions) is unchanged by this one.
- **The onboarding's live verify for the LLM step** is one bounded call
  on demand; the ledger's calibration rows name it when it runs.

## Notes

- **Gate, every time**: fmt 0, clippy 0, build 0, test 0, exit codes
  read directly on every increment. The final suite: 1 + 7 + 16 + 152
  passed, 3 ignored.
- **Measured, not asserted**: the queue's overflow (chip 506 → list 488
  → row 488 → text 375, ellipsized), the pane's width (440/720/300
  honored, the grip takes the pointer), PDF.js rendering real bytes
  (2,345 inked pixels), the xlsx and docx end to end, the door and the
  strategy preview rendered in Chromium against live documents.
- **Cache sweep at close**: `debug 13.2 GiB → 13.2 GiB; pruned 0 files
  older than 7d; free space 76.3 GiB (release untouched)` — nothing
  stale qualified.
- **The harness's own fidelity grew three times** this session (the
  hash setter fires `hashchange`; static `hidden` attributes are real
  initial state; attribute selectors replaced by class scans) — each one
  found because a check failed for a reason that was not the product.
- **Two defects the checks caught that no one had reported**: the
  `ownerRow` shadowing (owner messages vanished), and the `finished`
  case falling into the door handler (every receipt broke).
- Process registry: none on this machine — the only processes this
  session started were the scratch guis (swept) and the owner's own
  face, relaunched each time.
