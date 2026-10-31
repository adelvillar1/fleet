---
name: bsd-mktemp-suffix-trap
description: "BSD mktemp leaves literal X's when the template has a suffix, so anti-collision driver names like /tmp/foo-XXXXXX.py silently never uniquify — put the X run at the end of the basename."
type: project
date: 2026-10-31
basis: inferred
tier: durable
shape: [fail-open, resolved]
phrases: [mktemp, XXXXXX, .py suffix, literal X's, shared /tmp filename, driver collision]
missing: []
recall-count: 0
superseded-by:
valid-until:
---

macOS BSD mktemp only substitutes a run of X's that ENDS the basename. A template with a suffix — mktemp /tmp/foo-XXXXXX.py — returns the literal path with X's intact (verified by probe 2026-10-31: created '/tmp/mktemp-probe-XXXXXX.py'), so the file lands at a SHARED name and concurrent scheduled runs can overwrite each other's scripts — the concrete mechanism behind the 2026-10-30 shared-/tmp driver lesson. Two prior runs' survivors prove recurrence: /tmp/fleet-blend1-refresh-second-brain.XXXXXX.py and /tmp/sys1-refresh-sys1.XXXXXX.py (literal-X names, mode 600). Rule for hand-run refresh drivers: put the X's at the very end of the basename (mktemp /tmp/foo-XXXXXX) and pass the file to python3 explicitly, or derive uniqueness from $$/$RANDOM. Also: BSD rm -v prints only the bare path on success, so a bare path in output is a successful removal, not a listing.
