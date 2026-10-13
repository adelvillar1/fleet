---
name: fleet-cargo-is-not-on-path-on-this-machine
description: "cargo is not on PATH on this machine: prefix every cargo command with export PATH=\"$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH\"; the toolchain is rustup profile minimal, so rustfmt and clippy are absent (KI-007) and the style is matched by hand."
type: feedback
date: 2026-10-13
basis: confirmed-by
shape: [build]
recall-count: 0
superseded-by:
valid-until:
---

Every terminal session that touches this crate needs the PATH export first. There is no rustup shim to add the missing components with, and the owner's standing rule is to install nothing on this machine to work around it.

Source: CLAUDE.local.md "Local quirks" and CLAUDE.md's environmental blockers, 2026-10-13.
