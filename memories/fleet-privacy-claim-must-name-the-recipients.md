---
name: fleet-privacy-claim-must-name-the-recipients
description: "feedback 2026-10-21 — the owner rejected \"nothing leaves this machine\" as a premise, because every turn already goes to a hosted model; state egress per recipient (the provider that serves the turn, each service enabled, each parsed document), never as a machine-wide promise"
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

**The correction (2026-10-21), verbatim:** "you cannot apply the nothing leaves
this machine rule, because when you are using an llm you are sending documents
to the llm, or am I wrong?" He was not wrong.

**Why it matters:** fleet's roster is hosted (z.ai, xiaomi, stepfun, aliyun,
deepseek, liquid, ollama.com), so every turn and every file the agent opens with
a tool reaches that provider. The chat empty state said "Nothing leaves this
machine", which the model path never honoured — a false claim shipped in the
product's own voice. Worse, it framed a new egress (Nace Perception parsing an
attached document) as if it were the first one, which would have made the
disclosure wrong about the larger, older flow.

**The owner's follow-up settled the "local provider" escape hatch (2026-10-21):
"my only local provider is embeddinggemma2 for embeddings."** Confirmed against
the documented roster: the only local model is EmbeddingGemma-2 on
`127.0.0.1:8901` (embeddings only), and the 8420 `blend1` entry is a LANE
surface (`/v1/ask`, its `/v1/models` 404s), not a chat model. So **every chat
turn on this machine is hosted** — do not offer "a local provider could keep it
local" as a nuance here; it does not apply. The line to hold: the archive,
settings, memory, and embeddings stay local; the turn — prompt plus any file a
tool read into it — goes to the serving provider, which the route bar names.

**The owner's second correction, same day (2026-10-21): "sys1, sdm1, sem1 and
gen1 are also lanes that are abstracted to deal with multiple providers."
"Provider" is plural per turn, not one.** The lanes are local processes that
front hosted services, so a turn can reach several recipients beyond the model
that answers it: sys1 fans out ten provider keys (Fastino, TypeSafe, Drex,
Supersonic, Liquid, CLM, OpenJev, Kev, Together, Laya), sdm1 fronts TabPFN,
gen1 fronts DashScope and StepFun; only sem1's local sentence-transformer
worker and Kokoro run on the machine. The copy shipped as `38b9838`: "turns go
to the providers that serve them". Do not reason about lanes as local just
because the lane process is local.

**How to apply:** never write a privacy claim about the machine as a whole.
Name the recipients and what each receives:

- the archive, recall corpus, and session logs — stay local until something
  ships them;
- the providers that serve each turn — the answering model plus any lane
  provider it calls (plural: a lane is an abstraction over several services,
  not a local endpoint);
- every service the owner enables (web lane, hosted parsers like Nace
  Perception) — each is its own recipient;
- local exceptions are narrow and named: embeddings (EmbeddingGemma-2), sem1's
  local worker, Kokoro, the browser lane. The route bar and the lanes' names
  are where the owner sees who served.

When a feature adds an egress, the copy change is part of the work and gets its
own criterion (a literal, greppable sentence), not a follow-up — see
[[plan-is-a-file-not-prose]] for why criteria must be literally checkable.

**Owner follow-up 3 (2026-10-21), which reframes egress a second time: "is that
reflected in the plan? … what happens to file types like word, excel,
powerpoint?"** The answer was no, and the correction is bigger than copy: a
plan that routes **every** binary document to a hosted parser treats the vendor
as the default when it should be the escalation. The practical rule that came
out of it — **local first, egress only for what local cannot do**:

- Clean Office/CSV/HTML/text-PDF extracts offline (Office formats are ZIP+XML;
  MarkItDown, openpyxl/python-docx/python-pptx, LibreOffice, pdftotext).
- A hosted parser (Nace Perception) takes only scans, layout-heavy PDFs,
  images, email and media, or a local extraction that came back empty.
- The attachment record names **which parser ran**, so "left the machine" is a
  per-document fact rather than a property of the feature.

So when adding an egress, answer three questions in the plan: what runs
locally, what escalates, and how the record says which one happened. Perception
does cover .doc/.docx/.docm, .xls/.xlsm/.xlsx/.csv and .ppt/.pptx across all
five of its tools — format coverage is not the reason to escalate; layout and
scans are.

**What shipped:** `543ff95` — the empty state now reads "the archive stays here;
turns go to the provider that serves them", verified live. The perception plan
was re-gated after the premise fix (`0defb2e`).

**Where the rule landed in code (2026-10-21).** The document attachment shipped
with the per-document record it demanded: the chip and the framed turn carry the
parser (`local:docx`, `local:pdftotext`, `nace`), pages, units, job id and the
escalation reason, so "left the machine" is a fact on one attachment. Escalation
is gated twice: the first `/api/attach` answers `{type:"attach_needs", reason}`
naming what would be sent and why, and the vendor is called only on a second
request carrying `consent:true`. A keyless home refuses by name. The copy test
(`documents::tests::the_copy_names_the_recipients`) greps both assets for the
old sentence and asserts the truthful one is present — the criterion is a
failing-if-regressed test, not a manual review.

Related: [[fleet-document-perception-plan]],
[[fleet-composer-needs-permissions-and-a-route-pin]],
[[fleet-routes-through-its-own-router]].
