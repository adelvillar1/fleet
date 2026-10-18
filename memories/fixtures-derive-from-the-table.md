---
name: fixtures-derive-from-the-table
description: "eval fixtures are dynamic because the preference table is configuration and will move: a fixture derives its expected provider from the same source the grader reads, and a mechanics test declares its own table rather than inheriting today's target."
type: project
date: 2026-10-15
basis: confirmed-by
tier: durable
shape: [change]
phrases: [hard_target derives from workload_candidate; mechanics tests declare their own tables; never hand-rebase a fixture to today's target]
missing: []
recall-count: 97
superseded-by:
valid-until:
---

The owner's correction, now the rule: a hand-rebased fixture passes today and fails the day the table moves, and it fails in the direction that hides the break - the fixture asserts the old target while the router correctly serves the new one. The dynamic form derives the expectation from workload_candidate through a shared hard_target helper, so the fixture and the grader can never disagree about what the table says. Mechanics tests that need a specific table declare their own - a routing.json in a test home, or a Vec<Preference> passed to select - which keeps the derivation honest where it matters and the declaration explicit where it does not.
