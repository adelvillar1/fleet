# Session Recap — Fleet leaves the fork and becomes its own repository

**Date:** 2026-10-13 **Branch:** `master` (default, all work) + `fleet` (working branch), both level with each other **Plan:** `docs/plans/2026-10-12-fleet-native-app.md` — "a fully native Rust agent app", still `active` **Repo:** the fork at `git@github.com:adelvillar1/deepseek-harness.git` (SSH, private); `upstream` stays pull-only

## Summary

Fleet is now its own repository: [adelvillar1/fleet](https://github.com/adelvillar1/fleet), private, MIT-licensed, with its own CI. The Rust crate was extracted from this fork's `fleet/` directory with both of its commits intact, given the three things the fork had been supplying on its behalf, and removed from the fork's tree. The local checkout is `~/Projects/fleet`.

The trigger was a question about why no `~/Projects/fleet` existed. The answer was that Fleet had never been a separate project at all — and that the word "self-contained" had been read as a statement about runtime dependencies rather than about repository boundaries, when the owner's intent was the latter.

## Commits

| Part | Message |
|------|---------|
| extraction | Fleet: the native Rust agent, slice 1 (own client, tool loop, sessions, doctor) |
| extraction | Fleet: a native Rust launcher distribution around the intact harness |
| standalone repo | Standalone repository: own MIT license, CI, and a self-contained README |
| this fork | Fleet: the crate leaves the fork for its own repository |

The first two rows carry into the new repository with their original messages, authors and dates, rewritten only by `git filter-repo`'s subdirectory filter. The last row is this fork's removal.

---

## What the investigation found

Fleet's crate sat at `fleet/` in this fork with no nested `.git` — a plain subdirectory, not a project of its own. It arrived in two commits: one added it as a std-only Rust launcher that booted the intact harness, an approach later abandoned and deleted, and the other rewrote it as the standalone native agent. Both landed in the fork's history, and no repository named `fleet` existed on the account.

The reason it was placed there is that every recorded direction defined self-containment in terms of runtime dependencies: "one binary per OS carrying its own Node runtime and harness payload, extracted on first run — no checkout, no pnpm install", and later "fully written in rust and not a wrapper to the existing code." Neither said anything about a repository boundary, so the crate was created in place and never extracted. The reading was wrong, and the owner corrected it: decoupling from the fork was the point, not a consequence.

## What made the extraction safe

The crate was already decoupled in substance, which is why the move was mechanical rather than a refactor. `fleet/` is absent from `pnpm-workspace.yaml` — correctly, since it is a Cargo crate and not a Node package — and no CI workflow referenced it, so nothing in this fork's tooling built or tested it. Every `fleet/` string in the tsconfigs and `pnpm-lock.yaml` resolves to `packages/fleet/`, the plugin group, which is a different thing entirely. No root `Cargo.toml` ties the crate to a workspace. Its README already claimed that `cargo build` from that directory was the whole install, and acceptance criterion C5 had verified that no Node invocation, checkout reference or `dsh` launch path remained in its source.

## What was added to the new repository

Three things the fork had been supplying on Fleet's behalf. An MIT `LICENSE` under the owner's copyright, following the convention of the owner's other standalone Rust tool. A CI workflow running `cargo fmt --check`, `cargo clippy -D warnings`, `cargo build --release` and `cargo test`, in the shape of the owner's other single-job workflows, with a comment recording that the suite is keyless by design because every integration test spawns the crate's own mock server on loopback and the one real-API test is `#[ignore]`d. And a README whose attribution paragraph credits DeepSeek Harness as the architectural base without pointing at a sibling checkout.

## Verification results

The extracted crate builds and tests green in its new home: 44 unit and 11 integration tests pass with zero warnings, matching slice 1's C1 exactly. The same 55 pass again in the fresh `~/Projects/fleet` checkout. Every file in the fork's `fleet/` is present in the new repository byte-identically — verified by comparing the fork's committed tree against the new repository's archive — with one deliberate exception: `README.md`, whose license link was corrected. No file was lost.

The new repository's CI is **red, and not because of the code**. The job is created but never dispatched: zero steps, no runner, and a two-to-five second wall time, on both the initial push and a re-run. The workflow file is valid and pushed with all five steps present, so this is an account-level Actions condition rather than a defect in Fleet. It needs the owner's attention and is not something this session could diagnose from the API — the billing endpoint is not readable with the available token. The owner's `agnostic-router-kit` repository shows the same red-CI condition, so it is not specific to Fleet.

## Files changed

This fork lost the crate's 16 tracked files and 283 MB of untracked `fleet/target` build residue, which was deleted along with the directory. `docs/plans/2026-10-12-fleet-native-app.md` gained a section recording the move, a corrected status line, and an `updated` date of 2026-10-13; two path references to `fleet/` in C5 and its result paragraph were reworded, since that path no longer names anything in this repository.

---

## Open questions / next steps

- **The new repository's CI does not run.** The job never reaches a runner. This needs an account-level fix before the workflow can act as a gate.
- **The live-API acceptance criterion C3 is still blocked by account balance**, unchanged by the move. The stored key authenticates and reaches api.deepseek.com, which answers 402 Insufficient Balance.
- **Fleet's slices 2 and onward now belong to the new repository**, not this fork. The plan here stays as the design record for slice 1; anything further should be planned and committed in `adelvillar1/fleet`.

## Notes

The lesson is the one already recorded in the previous recap, arriving from a new direction. There, "the package is unmodified in my tree" was a causal fact dressed as a scoping judgment. Here, "self-contained" was a runtime fact dressed as an architecture decision. In both cases the words on the page were narrower than the intent behind them, and the gap only closed when the owner said so directly. The general shape: when a direction can be satisfied at more than one layer, the cheapest reading is usually the wrong one, and it is worth asking which layer was meant before building.

The other thing worth keeping is that the extraction was cheap precisely because the earlier work had been done honestly. A crate with no inbound dependency on the harness moves in one command; one that still reached back into the fork's packages would have turned this session into a refactor.
