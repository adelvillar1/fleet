---
name: tmp-driver-clobber-by-sibling-cron-run
description: "A shared /tmp driver filename let a concurrent corpus-refresh run overwrite this run's script, so the invocation executed the sibling project's refresh instead of its own."
type: feedback
date: 2026-10-26
basis: inferred
tier: durable
shape: [foreign-root, resolved]
phrases: [/tmp/blend1_refresh_driver.py, blend1_refresh_ci.py, action: no-source, cruiserintelligence, unique driver name, date +%Y%m%dT%H%M%S_$$]
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The corpus-refresh catch-ups hand-roll their stdio drivers into `/tmp` under near-identical names (`blend1_refresh*.py`; thirteen were present on 2026-10-25), so a generic name is shared state between concurrent scheduled runs. In this session `/tmp/blend1_refresh_driver.py` was written as 3981 bytes hardwired to project `sys1`; minutes later the same path held 1850 bytes hardwired to `{"project": "cruiserintelligence"}` with no sources — a sibling job's driver, shaped exactly like the existing `blend1_refresh_ci.py`. The next `python3 -u /tmp/blend1_refresh_driver.py refresh …` therefore initialized the lane and called `refresh` for `cruiserintelligence`, answering `{"project": "cruiserintelligence", "source_files": 0, "items": 0, "changed": false, "action": "no-source"}` for a command whose own argv named `sys1`. Nothing was written to any store (the foreign call was `no-source`), so the only damage was a report that looked like this run's own.

The tell: the printed output format was not the format of the script that was written — labels `INIT:`/`REFRESH:` instead of `INITIALIZED:`/`RESULT_RAW:`, and a `project` nobody asked for. Reading the file back before trusting its output is what caught it.

The fix, applied and verified: write the driver under a run-unique name (`/tmp/blend1_sys1_catchup_$(date +%Y%m%dT%H%M%S)_$$.py`) and run it in the SAME bash invocation that writes it, so no window exists in which a sibling can clobber it. Logged as KI-317 in docs/known-issues.md.
