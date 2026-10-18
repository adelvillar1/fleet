# Fleet routes through its own router — the kit is the precursor, not a dependency

Status: complete — landed 2026-10-16. The owner's five corrections during
the diagnosis reshaped this plan twice; the shape below is the one that
landed, and the superseded design is recorded at the end so the record is
honest about what changed and why.
Date: 2026-10-16

## Why

The owner reported the router board failing to read the kit at
`http://127.0.0.1:8301`, and named the cause. What the diagnosis found was
not one defect but a design that had Fleet handing its routing to another
app.

**The board read a dead config, and then a live one it should never have
read.** `boards.rs::router_config_path` preferred
`~/.agnostic-router-kit/router/config.json` — port 8301, nothing listening
on it — over `~/.zcode/router/config.json` — port 8300, where a live node
server answers. So the board dialed 8301 and reported it could not find
the router. Fixing *that* defect meant pointing Fleet at the zcode
harness's own router, which is where the owner's corrections began.

**Fleet declared another app its head.** `HEAD_ID` was `router-kit`, the
roster's first candidate. Every judged route already went direct — no
workload preference named the kit — but the declared HEAD was the kit, so
the empty-table fallback walked the declared chain *from the kit* and
served `auto`, letting the kit re-route the request with its own judge and
its own keys.

The governing requirement, in the owner's words: "fleet needs to have a
router that's owned by fleet. the precursor is not the solution." Fleet is
a self-contained app. Its router is the in-process `route::select` — sys1
asks for a workload verdict, the workload preference table picks the
provider, and failover walks the declared chain. It is not a process, not
a port, not a service. The `agnostic-router-kit` is the **precursor** to
Fleet, the app Fleet superseded. Port 8300 is the **zcode harness's own
router**, another harness's, and Fleet never routes through it and never
reads its config.

Three constraints the owner set explicitly, all still in force:

- **No external router is a dependency of Fleet.** Nothing in Fleet needs
  a router process to be up — not to select a route, not to show the
  router board.
- **Do not remove the zcode-router-kit.** The kit deployment on the
  machine stays running. That is simply not Fleet's business to touch.
  The lineage, for the record (KI-013): `agnostic-router-kit` is the
  neutral core extracted from `zcode-router-kit` on 2026-10-04 — the
  kit's own `CLAUDE.md` says so — and the ZCode edition is one of its
  consumers, not its predecessor. The two names are not
  interchangeable, and the extraction is why this repo's roster entry
  for the kit is gone entirely rather than renamed.
- **Fleet's router never reads a config file from a different harness.**
  The same ruling as `2026-10-16-own-home-credentials.md`, applied to the
  board's token-resolution leg.

## Acceptance criteria

- **C1** The router-kit entry is gone from the compiled-in roster
  entirely. `HEAD_ID` names Fleet's own head (`zai-coding-plan`) and the
  invariant `default_roster()[0].id == HEAD_ID` still holds.
- **C2** `route::select` is Fleet's whole router, and no automatic path
  hands a request to another app for it to re-route. With the kit gone,
  the empty-table fallback walks Fleet's own providers, period.
- **C3** The router board dials nothing: no HTTP client, no port, no
  operator API, no token, no `AGNOSTIC_ROUTER_*` name anywhere in
  `src/boards.rs`. It reads the same three things the router reads — the
  declared chain with each candidate's resolved key, the workload
  preference table, and the day's route rows under the calibration root.
- **C4** The board renders Fleet's own routing state: the tier chains, the
  gates and the pin, what Fleet just served, the day's counts, the judge
  split, the workload mix, the per-model bars, and Fleet's own demotion
  guidance — at 42 columns without clipping, with a test pinning the
  80-column facts.
- **C5** The pay-as-you-go guard reports the policy honestly: deepseek is
  payg, is declared in no workload's chain, and serves only when the owner
  pins it. Two tests pin the refused case and the pinned case.
- **C6** The board's ledger line says the day's route rows carry no token
  counts rather than estimating one, because the route row genuinely has
  no token field.
- **C7** The docs that called the kit Fleet's head are corrected:
  `docs/features/routing.md`, `docs/features/boards.md`,
  `docs/architecture/ecosystem.md`, `CLAUDE.md`, `CLAUDE.local.md`
  (gitignored), and the KI-116/KI-126 rows in the known-issues table.
- **C8** The gate runs whole and green, and a live `fleet providers` shows
  the seven-candidate chain with the kit absent and `head
  zai-coding-plan`.

## Files to be touched

`src/boards.rs` (the board rewritten onto Fleet's own sources, the HTTP
leg and the token-resolution leg deleted, the payg guard fixed),
`src/tui/ui.rs` (`router_lines` and the board's render tests),
`src/route.rs` (the roster, its doc comments, `fallback`'s doc), `src/eval.rs`
(the calibration roster's stand-in), `src/calib.rs`, `src/judge/tests.rs`,
`src/route_cache.rs`, `src/memory.rs` (two seeded facts), `src/settings.rs`
(a fixture), `src/home.rs` (a test-isolation race), `docs/features/routing.md`,
`docs/features/boards.md`, `docs/architecture/ecosystem.md`, `CLAUDE.md`,
`CLAUDE.local.md` (gitignored), `docs/known-issues.md`. New files: none.

## Verification

The four-leg gate with every exit code read directly; `fleet providers`
live (head is zai-coding-plan, the kit absent); the router board live
(it reads Fleet's own routing state and shows the tier chains); a judged
`fleet run` still walking stepfun.

## Out of scope

- The kit's own converters (codebuddy, opencodex) — the owner can add them
  as ordinary candidates through `fleet settings provider.add:` if wanted.
- The second deep dive's remaining findings — a later plan.
- Any change to the kit's deployment on this machine. It stays running;
  Fleet neither touches it nor depends on it.

## The superseded design, recorded

This plan was first written to *keep* the kit in the roster and move
`HEAD_ID` past it, with the board resolving the live router at 8300
(criteria C5–C6 of that draft: prefer, among the configs that resolve, the
one whose router answers on the port its config names). The owner
corrected that five times — the port 8300 is the zcode router, not the
kit's; Fleet must be self-contained; the kit is a precursor, not a
dependency; Fleet needs a router it owns. The superseded criteria are
recorded here rather than deleted, because the correction is the ruling
and a plan that quietly rewrites itself loses it.
