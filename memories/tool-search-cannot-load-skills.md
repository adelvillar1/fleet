---
name: tool-search-cannot-load-skills
description: "tool_search ranks tools and cannot load skills; a skill named in the session's skill list but absent from the callable function set is loaded by reading its body at ~/.agents/skills/<name>/SKILL.md and executing it by hand"
type: structure
date: 2026-11-09
basis: inferred
tier: durable
shape: [fail-open]
phrases: [tool_search cannot load skills, use_skill not callable this session, ~/.agents/skills write-session-recap SKILL.md, docs/recaps YYYY-MM-DD-slug house naming, read the skill body and execute by hand]
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source:
---

When the owner asked "write a session recap" (2026-11-09), the session's skill list carried `write-session-recap` but the callable function set carried no use_skill function; `tool_search` — the only discovery surface — is a TOOL ranker (returns MCP tools with junk scores and "no tool clears the floor") and cannot fetch a skill BODY. The correct instrument, verified after the fact: the skill's full procedure lives at ~/.agents/skills/<skill-name>/SKILL.md (+ templates/), readable straight from the owner's cross-harness library, and the procedure is executed by hand from that file. Two further house notes: fleet's recap convention on disk is docs/recaps/YYYY-MM-DD-<slug>.md (three files at 2026-11-07/08), NOT the template's SESSION-RECAP-YYYY-MM-DD.md default the description carries; and the nomenclature write-guard refused the first recap draft for quoting a capitalized case-variant of an established term — paraphrase, don't misquote. The earlier claim "the recap skill isn't loadable here" was stated without attempting the named instrument or reading the house library — the load path broken is the runtime's function exposure, never the library itself.
