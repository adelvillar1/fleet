# The inbox

One queue of everything waiting on the owner. Five machineries feed it, one
read draws it, and the owner's decisions about a row are kept beside it.

The surface is the Inbox page: two segments (Waiting and Archived), class
filters with counts, one row per thing that needs a decision, and a context
menu on every row.

## The five classes

| Class | Read from | The row's own act |
|---|---|---|
| seat | the attached sessions' pending tool calls | once / always / deny |
| onboard | the onboarding ladder's incomplete steps | open that step's own surface |
| ask | the rooms whose last `@you` went unanswered | answer in the room |
| cron | jobs whose last run failed or was skipped | open the job |
| doctor | the doctor's failing checks | open doctor |

Every row is read live: a seat disappears when it is answered, a step when it
is done, an ask when the room is replied to, a job when its next run passes, a
check when it passes. The inbox stores none of the five.

## The owner's decisions

Two lists in `<FLEET_HOME>/inbox.json`:

- **dismissed** — hidden from the queue while the fact's **signature** holds.
  The signature is the fact's own fingerprint: a seat's call, an ask's
  timestamp, a job's last run, a step's narration, a check's message. When the
  fact changes, the signature changes and the row comes back. Dismissal hides
  a condition, never the machinery's next report of a new one.
- **archived** — kept with the day it went and what it said, restorable, and
  it outlives its own fact: an archived row whose condition has since resolved
  still reads, marked "the fact has since resolved".

The file is written atomically and pruned on every write: the archive keeps the
newest 200 rows and nothing older than 90 days.

**Seats are the exception.** A seat is a live gate holding a run: hiding it
would strand the run, so it cannot be dismissed or archived. Its three answers
are the only verbs — deny is its dismissal — and its menu carries only "open
the conversation".

## The controls

- **The primary button is the class's own act** (see the table above). The
  onboarding rows land on their own surfaces through one table
  (`vault` → `#/vault`, `github` → the settings account card, the rest →
  `#/settings`); every other row's primary opens what it names.
- **The `…` menu** carries `open`, `dismiss` and `archive` (seats: `open`
  only).
- **`not now`** on an onboarding row is a dismiss.
- **Keyboard:** `j` / `k` move the selection, `enter` runs the selected row's
  primary verb, `d` opens its menu.
- **A refusal paints in the pane that asked** — the reason, beside the row,
  never only in the transcript two panes away.

## The read and the ops

- `GET /api/owner-inbox` — one document: the five class arrays (each row with
  its `key`, `signature`, `label` and `class`), the `archived` list, and
  `total` (the waiting count, which is what the rail badge shows). Dismissed
  rows are filtered out of the arrays here, so every surface that reads this
  document agrees.
- `POST /api/inbox` — `{ "op": "dismiss" | "archive" | "restore" | "undismiss",
  "key": "<class>:<id>" }`. The answer is the fresh document. A malformed op, a
  key the read no longer carries, or a restore of nothing archived is a `400`
  with the reason named.

## What the keys mean

`<class>:<id>` — `seat:<session>`, `onboard:<step-id>`, `ask:<session>`,
`cron:<job-name>`, `doctor:<check>`. The key names a row across reads; the
signature says whether it is the same fact.
