---
name: a-contrast-measurement-needs-the-alpha-channel
description: "feedback 2026-11-07 — a CDP contrast measurement that normalizes Chrome's 0-1 color components drops the alpha channel and reads every transparent background as black, reporting 132 phantom low-contrast failures; parse the alpha separately and skip elements whose background is transparent"
type: feedback
date: 2026-11-11
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source: zcode:fleet:content:285983102772d99aaed2c496d57acdf6f320aad7347d3891e9bb9e2378839d34
---

Measuring the component-catalog mockup's ink contrast over CDP, the script
parsed `getComputedStyle(el).backgroundColor` with a regex, normalized
0–1 components to 0–255, and took the first three numbers as RGB. Chrome
returns `rgba(0, 0, 0, 0)` for a transparent background — the alpha is the
FOURTH number, and the normalization sliced it off, so every transparent
ancestor read as pure black and the walk reported 132 "low contrast" failures
on text that was plainly legible. The fix: read the alpha separately
(`match(/[\d.]+/g)` length ≥ 4) and only accept a background when
`alpha > 0.9`.

**Why:** a measurement that cannot see transparency manufactures findings.
The phantom failures are worse than no measurement — they send you chasing
defects that do not exist and teach you to distrust the tool. The same script
also flagged a real defect the first pass missed (the primary button's
white-on-accent failing AA in the dark themes), so the broken parser nearly
cost the one true finding.

**How to apply:** in any DOM contrast pass, resolve the background by walking
ancestors and stopping at the first OPAQUE one; treat `rgba(…, 0)` as
transparent, not black. And when a measurement reports a flood of failures,
suspect the measurement before the page — a small sample of the failures
(three or four) tells you which class of defect the tool is actually seeing.
See [[measuring-a-mockup-by-reading-the-png]] — the read and the measurement
catch different defects, and a broken measurement catches neither.
