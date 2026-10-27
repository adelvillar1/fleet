# The surfaces: what each is for, and why

A read of the browser face as a user meets it, taken live on
2026-10-21 against every destination in the rail. This is the what and
the why of each surface — the terms the README and the functional
specification should speak. The how stays in the code.

## The shape every surface shares

Eleven destinations in four groups — **work** (conversation, sessions),
**intelligence** (router, delegation), **knowledge** (memory, skills,
second brain), **system** (bots, vault, settings, calibration) — one
flat level, reachable from the rail or the ⌘K palette by name. Every
destination is built from the same grammar:

- a **head**: the group, the surface's name, and one sentence saying what
  it is for;
- an **identity card**: the object this surface is about (the archive,
  the corpus, the library, the stream, the home, the vault, the service,
  the tool) with its one-line status and one or two **ask** buttons;
- a **strip** of counts — the surface's facts at a glance;
- a **column** of cards that is the surface's substance, and a **rail**
  of secondary reads beside it.

The ask buttons are the tell. "Ask about the archive", "ask what fleet
knows", "ask the curator", "ask sys1 for a judgment", "ask for the next
step", "run a gate review" — every surface is an entry point into the
conversation about the thing it shows, not a dashboard to stare at. The
feature doc says it plainly: *the conversation is the app*; the router's
decisions, the recalls, the skill loads and the delegations render as
cards inside the transcript as they happen.

## The principles the surfaces embody

These are the repo's own rules, and each surface is one of them made
visible:

- **Local first, and every recipient named.** The archive stays on the
  machine; a turn goes to the provider that serves it; a document
  escalated to Perception goes to Nace. The home page prints that
  sentence under the composer.
- **Everything the model sees is in the log.** A session's replay is the
  truth of what happened — which is what lets Sessions read a run beside
  its list and Delegation rebuild a tree from the logs alone.
- **Nothing is hardcoded; the router decides; the owner overrides.** The
  composer's provider control reads *Auto*; the router board reads *no
  pin*; a pin is the owner's standing choice, never a default baked in.
- **No form asks for a value fleet already has.** Settings picks every
  value from what the home knows — models, lanes, roles, skills — and
  every write rides the one writer the CLI rides.
- **A proposal never applies silently.** Skills drafted from session
  evidence wait in a decisions queue; self-maintenance changes apply only
  through the local gate, with the verdict recorded.
- **Graded, not just recorded.** Calibration is the audit spine: every
  route, every tool screening, every run ending lands as a row, and a
  gate decides whether the failover may learn from the day.
- **A value is never rendered.** The vault lists names and seals values;
  the passphrase lives in the process and is never written.
- **Plan, delegate, verify.** The home names the method — the run writes
  nothing in plan mode, the graph carries the order, sys1 grades the
  evidence.

## The surfaces

### Conversation — *work*

**What it is.** The home and the workbench: "What are we building?"
over three counts (facts the corpus knows, skills in reach, runs under
this home), the method in three words, the lanes as chips, four offers
that are prompts in disguise (walk the router, read the plan, review
today's changes, ask what fleet knows), and the recent sessions to pick
up where you left off.

**What you do.** Type a task. Choose how much to ask first — *Ask
before*, *Autonomous*, *Full access* — and, if you want, which provider
serves it and which folder it works in. Attach a document from the
workspace or a file from the machine. Plan instead of build. Send, and
watch the run happen as cards: the route it took, what it recalled, the
skill it loaded, the child it spawned, the approval it is waiting for.
The head names the conversation — its title (the switcher), the pencil
that renames, its state in words, the way back to the work list, and
New — so the conversation is a place you can leave and another you can
choose. Replies read as
markdown, at the block boundary where the whole answer is in hand. The
conversation itself carries its text size — `A−`/`A+` in its head, or
`Alt+↓`/`Alt+↑` from the composer — because it is the one surface the
owner reads at length: the reply, its headings and its code scale
together, and so do the run's own rows (the thought, the command card,
the receipt), while the chrome around them keeps the measure it has.

**Why.** Because the valuable things a harness does were, everywhere
else, hidden in panes and logs. Here they are the transcript.

**State.** The core surface; complete. The working-folder picker now
browses the whole disk and takes a typed path.

### Work — *work*

**What it is.** Every conversation and every run under this home, as one
list grouped by what needs the owner rather than by where it was born:
*needs you* (an approval waiting, a run the owner paused), *working*,
the *background* — one line, closed by default, that says how many of
the app's own runs (scheduled jobs, delegated children, peer
conversations) it holds and opens to the family lines on request — and
*earlier*, grouped by project.

**What you do.** Read the state in words — *completed*, *capped*,
*stopped by you*, *interrupted*, *active*, *new* — with each row's age,
project, and, for background work, its job. Open a row: it loads in
place with its composer. Search the title, id, project, model, job and
state. Or press New. The column folds to a strip — its way back and the
count of what needs you — and its left edge follows the navigation, so
a folded rail leaves no gutter behind the list and a folded list leaves
the width to the conversation.

**Why.** A session log is the system of record — if it is not in the log,
the model never saw it — so the list is where accountability lives. But
accountability is not the owner's first question: "what needs me" is.
Background work exists to keep the home alive; it should not crowd out
the work the owner started, and an unfinished log should not pose as a
live run.

**State.** The archive's rows now carry their ending, their turns, and
the log's write time, and the bridge derives the state from them plus
the live face (the seat, the driving session, a pause's control file,
and a write-time window for a run on another process — a bound the
surface says rather than a heartbeat the log does not have). The
separate read-only replay retired: the conversation is the replay.

### Router — *intelligence*

**What it is.** Where this run's work went, what it tried first, and
what the day has served: the live decision, the declared chain per
workload (quick, standard code, hard, prose, deep context, vision,
composed), today's ledger of calls, the model and workload mix, and the
decision cache.

**What you do.** Read why a run landed where it did. Ask sys1 for a
judgment on a task to see how it would be classified. Open settings to
change the chains or pin a provider.

**Why.** Routing is the one decision a user cannot see in any other
harness: which model, chosen how, and what happened when it failed. The
board makes the decision auditable and the failover visible — the tail
that closes every walk, the demotion that reordered it, the pin that
narrowed it.

**State.** Complete; its numbers reflect the week's single-model pin and
the nightly jobs rather than real routing traffic, which only begins now
that the pin is off.

### Delegation — *intelligence*

**What it is.** The run's own tree of children, the runs before it, and
the dependency graph the plan declares. Depth cap, nodes in this run,
past runs, graph nodes, reads refused.

**What you do.** Watch a run spawn children and read what each was told
and what it returned. Ask for the next step.

**Why.** A child is a real session on a scoped tool set with a bounded
turn cap, routed by the router over its own brief, returning partial
results with a typed reason instead of silence. The tree is rebuilt from
the logs' own parent ids and the graph is a file the plan's work writes
— nothing is inferred from what a run did. Delegation is accountable
the same way everything else is: through the record.

**State.** The surface is complete; it is only as full as the work is.
Without a run delegating or a plan declaring a graph it reads empty, and
says so.

### Memory — *knowledge*

**What it is.** One fact per file: what this run recalled and what the
corpus already knows. Facts, live, protected, retired, today's hits,
recalls this run.

**What you do.** Search the corpus by name, sentence, type, shape or
body; open a fact with its whole body and edit it in place; ask fleet
what it knows; see what was recalled today and how strongly.

**Why.** Memory is a git-tracked folder of markdown facts, not a database
nobody can read — so a fact can be edited, protected, retired, diffed,
and recalled by meaning. Each recall is scored, and recall feeds back
into what rises next time.

**State.** Complete, with 24 facts in this repo's corpus and the
recall-count feedback live.

### Skills — *knowledge*

**What it is.** The library the model can reach, the names the loader
declined and why, and the proposals waiting for a decision. Loaded,
used, never selected, proposals, skipped, sessions scanned.

**What you do.** Search the library; open a skill, its path, or ask the
curator about it; approve or read a proposal the harness drafted from
session evidence; see which skills the loader skipped, in its own words.

**Why.** Skills are procedural memory shared across every harness the
owner runs, disclosed description-first and fetched on demand — and the
harness writes its own, as proposals that never apply silently.

**State.** The library loads 442 skills; the usage scan reports none
selected yet across 20 sessions, which is the honest number and the
curator's cue.

### Second brain — *knowledge*

**What it is.** The owner's control tower, read as an atlas: every
project's six-week pulse, the plans it keeps open, and the one line its
last session left behind, with the decisions that need a person kept
beside them. Projects, active, dirty, commits in 30 days, open plans,
conflicts, needing attention.

**What you do.** Scan the landscape; open a project's own page; read what
needs attention — ledger drift, stale plans, live handoffs, claims versus
evidence; refresh the tool's state.

**Why.** Second brain is a tool the owner already runs, with its own
repository and its own truth. Fleet surfaces what it publishes and
forwards its actions rather than reimplementing it — the plugin shape
for anything the owner already trusts.

**State.** Complete as a read; the tool's derived state is what it
shows, with its own freshness stamp.

### Workspace history — *knowledge*

**What it is.** The LayerFS substrate's tree, read as fleet's own
history, in the substrate's own vocabulary: a **band** for every Layer
on main's spine, a **lane** for every branch with work sprouting from
its actual fork, a **leaf** for every commit, a **dashed stem** where
a publish grafted work back, and one idle chip for the branches nobody
worked. Beside it: the work those branches carried (`C<ordinal> ·
↓ L<k>` with state, activity and the child session), the substrate's
**live call feed** (every call the trace recorded, with its actor and
cost), a **file's own instruments** (text, blame, revisions, diff), and
the Store's **accounting** — on disk, unique, compressed, the metadata
subset, the SQLite overhead named separately, the Init record, and what
each commit cost. One backend read answers all of it in under 250 ms,
and a read whose version still holds answers "unchanged" in 1 ms.

**What you do.** Read the tree as it stands — zoom, pan, follow the
minimap, hover a node for its record; click a node and the file
instruments read at that ref (branch, commit or layer); watch what the
substrate is doing in the feed; see which children merged back and
which are still open. The Delegation panel's rows carry each child's
branch and click straight through.

**Why.** Isolation is fleet's property, not a substrate's: a delegated
child's writes land on its own branch and nothing reaches the working
tree until the merge is settled — and then the settle brings the merged
work home through the parent's own gated write path, holding any file
the owner touched since the fork. The import carries what a git repo
declares (its tracked files — the `.gitignore` is the owner's own
decision), so a build tree never blocks a registration, and each spawn
syncs the host's newer tracked files onto the fresh fork — the child
works from the folder as it stands, never a frozen import. This surface is
where the owner reads what the isolation kept and what came home.

**State.** Complete as a read. The tree is the substrate's own; a
machine that has not provisioned, or a folder no delegation has
registered, says so where the tree would be rather than drawing an
empty one — and the same rule holds in the small: a cut branch is named
in words, a binary file is named rather than rendered, and traces that
are off say so instead of reading as "no activity".

### Bots — *system*

**What it is.** The channels that carry work in from outside: who is
paired, what they may do, whether the service is listening, and what
they have asked for.

**What you do.** Turn the channel service on; seal a Telegram token;
let a webhook peer or a Telegram chat in, choosing at pairing whether it
may act or only listen; see who is paired, who is waiting at the door,
and what each has done; revoke a device.

**Why.** Work arrives from a phone, a CI box, a chat — and every peer is
a grant: paired by name, capability fixed at pairing, token stored as a
digest. Nothing from outside reaches the agent without a person letting
it in. All of it is off by default.

**State.** The pairing and device model are complete; the channel service
is idle in this home until turned on, and the bot-first phone shell is
planned, not built.

### Vault — *system*

**What it is.** The credentials Fleet seals and the passphrase that keys
them. Ten names sealed in this home.

**What you do.** Unlock the vault for this process, or start a new one
if the passphrase is lost; migrate plaintext `.env` entries into it on
first run; lock it again.

**Why.** Keys are the one thing a local-first agent must hold and must
never show. The vault lists what it holds and renders no value, ever;
unlocking holds the passphrase in memory until fleet exits.

**State.** Complete; a working surface with its own rail destination.

### Settings — *system*

**What it is.** The router, the providers, the lanes, the roles, the
vault, the spend caps, the rules, the cron jobs, and the identity slot
— every knob the home has, in one place.

**What you do.** Reorder a workload's chain or add a candidate from the
catalog; pin a provider; override a provider's endpoint or models; set a
lane's chain; create a role with its tools and skills; set spend caps;
remember or forget an approval rule; enable, disable or schedule a job;
write the identity slot.

**Why.** Every value is picked from what this home already knows, so no
form asks you to retype a model id or a path, and every write rides the
same writer the CLI rides — the browser never has a second truth.

**State.** Complete across all eleven sections.

### Calibration — *system*

**What it is.** The day's stream, the judged quality of what served, and
the gate that decides whether the failover may learn from it. Rows,
judged, fit cells, informing the failover; the day's routes, turns, eval
runs, recall hits, tool screens.

**What you do.** Read what the day recorded; run a gate review; see
which (provider × workload) cells are fit enough to move the failover.

**Why.** Recording is not enough — a routing decision has to be graded
against an independent judgment before it may change behavior. The gate
is explicit, visible, and can be turned off, so autonomy is earned on
evidence rather than assumed.

**State.** Complete; the stream is live and the judging beats exist,
with no judged cells for the day until a review runs.

## What a first-time user meets today

The surfaces are built; what they show is a home mid-development. The
numbers on Router, Sessions and Calibration today are dominated by the
nightly corpus-refresh jobs (fourteen, most ending in error — KI-277)
and by a week under a single-model pin, so the boards read busier and
redder than real use would make them. That is a state of the data, not
of the surfaces, and it clears as the pin stays off and the jobs are
repaired.
