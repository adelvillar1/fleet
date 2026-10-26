// layerfs-history.mjs — C4 of docs/plans/2026-10-25-layerfs-seamless.md.
//
// The face's trunk and figures, read through the bundle's BACKEND
// package — the one route to main's Layers (the agent CLI answers
// NoSuchBranch for main's commits) and the Store's storage accounting.
// Prints one JSON line:
//   { "layers": [...], "commits": [...], "workspaces": [...], "storage": {...} }
//
// Usage (the driver spawns it; the environment carries the composed
// LAYERFS_* facts and the worker's endpoint):
//   node layerfs-history.mjs <project> <bundle>
import { join } from "node:path";

const [project, bundle] = process.argv.slice(2);
if (!project || !bundle) {
  console.log(JSON.stringify({ error: "usage: node layerfs-history.mjs <project> <bundle>" }));
  process.exit(2);
}
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
  const history = await backend.store.project.get_history(0);
  const storage = await backend.store.project.get_storage();
  console.log(
    JSON.stringify({
      layers: history.layers ?? [],
      commits: history.commits ?? [],
      workspaces: history.workspaces ?? [],
      storage,
    })
  );
} finally {
  await client.close();
}
