# The health loop and the recovery mode

The failsafe's know-and-say half (the failsafe-restore plan, 0.6.22):
fleet knows a service is down and says so before something needs it,
and there is always a mode where the main session brings fleet back.

## The loop

The app's server probes on a beat (`src/health.rs`): layerfs's worker
port (only when the substrate is provisioned), each vendored lane's
presence, docker's daemon at the slow tier, and the API's key — cheap
checks only, never an MCP spawn. The verdicts ride `/api/state`'s
`services` block; the face paints a card naming every down service
beside its repair verb, and a transition announces itself on the
event stream.

## The repair note

A run over a provisioned-but-down substrate carries the repair on its
first request: the service, the cause, the verb, and the ruling —
never refuse the task over a down service; degrade and repair
instead. Logged as `SystemNote` (hard rule 5); the model repairs
through its own bash and the standing approval gate, bounded to one
attempt per down-transition.

## fleet recover

Every settings write stamps the prior file beside the home
(`<home>/recover/<name>.prev`). `fleet recover` restores every file
whose current form is broken from its stamp, then names the probes'
fresh verdicts — the main session's one command for "bring fleet back
to working order". The last-resort chain ends at routing.json's
`local_tail` when the owner configured one.
