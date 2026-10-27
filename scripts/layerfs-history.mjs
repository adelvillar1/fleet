// layerfs-history.mjs — C0/C3 of docs/plans/2026-10-26-layerfs-history-instrument.md.
//
// The face's reads through the bundle's BACKEND package — the one route
// to what the agent CLI cannot answer:
//   (default)      the whole `store.project.get_history` answer: the
//                  branches, their newest commits, main's Layers, the
//                  Store's nine figures (store.stats, with the Init
//                  record) and a `version` — ONE call for the whole view,
//                  or `{ kind: "unchanged", version }` for a `since`
//                  version that still holds.
//   --revisions    `store.fs.list_revisions <ref> <path>`: a file's
//                  revisions, newest first, one page.
//   --comparable   `store.project.get_comparable <ref>`: the Commit with
//                  the same files as a saved Commit or Layer id, so a
//                  Layer can ride a diff that takes Commits.
//
// Prints one JSON line — the backend's own answer, unpicked.
//
// Usage (the driver spawns it; the environment carries the composed
// LAYERFS_* facts and the worker's endpoint):
//   node layerfs-history.mjs <project> <bundle> [<since-version>]
//   node layerfs-history.mjs <project> <bundle> --revisions <ref> <path> [<pageSize>] [<pagePos>]
//   node layerfs-history.mjs <project> <bundle> --comparable <ref>
import { join } from "node:path";

const [project, bundle, ...rest] = process.argv.slice(2);
if (!project || !bundle) {
  console.log(JSON.stringify({ error: "usage: node layerfs-history.mjs <project> <bundle> [<since-version> | --revisions <ref> <path> [<pageSize>] [<pagePos>] | --comparable <ref>]" }));
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
  let mode = rest[0];
  if (mode === "--revisions") {
    const [, ref, path, pageSize, pagePos] = rest;
    if (!ref || !path) {
      console.log(JSON.stringify({ error: "usage: --revisions <ref> <path> [<pageSize>] [<pagePos>]" }));
      process.exit(2);
    }
    console.log(JSON.stringify(
      await backend.store.fs.list_revisions(ref, path, pageSize ? Number(pageSize) : 50, pagePos ? Number(pagePos) : 0)));
  } else if (mode === "--comparable") {
    const ref = rest[1];
    if (!ref) {
      console.log(JSON.stringify({ error: "usage: --comparable <ref>" }));
      process.exit(2);
    }
    try {
      console.log(JSON.stringify(await backend.store.project.get_comparable(ref)));
    } catch (error) {
      console.log(JSON.stringify({ code: error.code || "Refused", message: error.message || String(error) }));
    }
  } else {
    const since = mode && !mode.startsWith("--") ? mode : undefined;
    const history = await backend.store.project.get_history(since ? since : 0);
    console.log(JSON.stringify(history));
  }
} finally {
  await client.close();
}
