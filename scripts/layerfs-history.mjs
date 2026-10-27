// layerfs-history.mjs — C0 of docs/plans/2026-10-26-layerfs-history-instrument.md.
//
// The face's ONE read: `backend.store.project.get_history` passed through
// whole. One call carries the branches (name, base, forkedFrom, head),
// their newest commits, main's Layers, the Store's nine figures
// (store.stats, with the Init record) and a `version` — so the face's
// refresh is a single 8 ms backend call instead of the per-branch CLI
// fan-out this script's first draft served. A read whose `version` is
// still current answers `{ kind: "unchanged", version }` in 1 ms.
//
// Prints one JSON line — the backend's own answer, unpicked:
//   { kind: "ok", version, store: { stats }, project, branches,
//     moreBranches, commits, more, layers, moreLayers, workspaces }
//   — or { kind: "unchanged", version } when nothing moved.
//
// Usage (the driver spawns it; the environment carries the composed
// LAYERFS_* facts and the worker's endpoint):
//   node layerfs-history.mjs <project> <bundle> [<since-version>]
import { join } from "node:path";

const [project, bundle, since] = process.argv.slice(2);
if (!project || !bundle) {
  console.log(JSON.stringify({ error: "usage: node layerfs-history.mjs <project> <bundle> [<since-version>]" }));
  process.exit(2);
}
// The face's reads are not the delegation's activity: a trace file left
// set by the last child's spawn would record this script's backend calls
// into that child's feed, so the read path strips it before the client
// is made (the trace file is read when the client opens).
delete process.env.LAYERFS_TRACE;
// Both packages are imported by ABSOLUTE path from the sealed bundle —
// their internal imports resolve inside the bundle's own node_modules,
// and nothing resolves from this file's location.
const { LayerFS } = await import(join(bundle, "api/node_modules/@layerfs/agent/index.js"));
const { LayerFSBackend } = await import(
  join(bundle, "api/node_modules/@layerfs/backend/index.js")
);

const client = await LayerFS.open(project);
try {
  const backend = LayerFSBackend.of(client);
  const history = await backend.store.project.get_history(since ? since : 0);
  console.log(JSON.stringify(history));
} finally {
  await client.close();
}
