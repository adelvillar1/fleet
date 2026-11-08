---
name: tauri-updater-key-sealed-in-vault
description: "The Tauri updater's private key is sealed in the vault as FLEET_TAURI_UPDATE_KEY (2026-11-08), and the /tmp pair is the true half of the conf's pinned pubkey (minisign signer-id match proved)"
type: project
date: 2026-11-08
basis: confirmed-by
tier: durable
shape: []
phrases: [vault-seal-env, FLEET_TAURI_UPDATE_KEY, minisign signer id, TAURI_SIGNING_PRIVATE_KEY, untrusted comment, can you unseal it for me]
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source:
---

The owner's 2026-11-08 owe-item closed: FLEET_TAURI_UPDATE_KEY is sealed in the vault (owner typed the passphrase for an in-place unlock; the seal ran through the face's settings op vault-seal-env so the value never touched the agent transcript; the staged plaintext .env line was dropped after, grep-verified). Pair-proof: /tmp/fleet-updater-key signed a probe and the signature's embedded signer id (4223e59b2eb4069e) equals the conf's pinned pubkey id byte-for-byte — the earlier "mismatch" reading was a double-unwrap bug (Tauri's pubkey is base64 of the pubfile TEXT; minisign sigs/signatures decode directly to the untrusted-comment text then one more base64 on the payload line; secret keyfiles base64 directly to text). Key-id extraction: unwrap to untrusted-comment text, payload line base64 -> blob; alg 2B + id blob[2:10]. No rotation needed. The old cut's .sig (2026-11-08 11:41) is therefore valid against the pinned key. /tmp/fleet-updater-key{,.pub} still exist on the machine post-seal for the first cut's convenience (release.sh reads /tmp/fleet-updater-key or vault extraction at cut time).
