// layerfs-worker.mjs — C3 of docs/plans/2026-10-25-layerfs-seamless.md.
//
// Starts the home's worker through the bundle's RUNTIME package — the
// sanctioned macOS route (one long-lived Linux container per home, its
// port published to the Mac's loopback, the port and secret written to
// <home>/worker/endpoint.json). The runtime's start is idempotent: a
// running worker is left running, a stopped one is started, a
// container of another build is replaced. Prints one JSON line:
//   { "endpoint": "<home>/worker/endpoint.json", "state": "running" }
// or { "error": "..." } with exit 1, so the install's step can name a
// failed start instead of claiming a substrate that cannot answer.
//
// Usage (the install's step spawns it; the environment carries the
// composed LAYERFS_* facts):
//   node layerfs-worker.mjs <home> <bundle> <image>
import { join } from "node:path";

const [home, bundle, image] = process.argv.slice(2);
if (!home || !bundle || !image) {
  console.log(JSON.stringify({ error: "usage: node layerfs-worker.mjs <home> <bundle> <image>" }));
  process.exit(2);
}
// The runtime is imported by ABSOLUTE path from the sealed bundle —
// its own internal imports (@layerfs/agent) then resolve inside the
// bundle's node_modules, and nothing resolves from this file's
// location (which has no node_modules of its own).
const { LayerFSRuntime } = await import(join(bundle, "api/runtime/src/index.js"));

const runtime = await LayerFSRuntime.open({ home, image, bundle });
try {
  const status = await runtime.worker.start();
  const state = status?.worker?.state ?? "unknown";
  if (state !== "running") {
    console.log(JSON.stringify({ error: `the worker is ${state} after its start` }));
    process.exit(1);
  }
  console.log(JSON.stringify({ endpoint: join(home, "worker", "endpoint.json"), state }));
} catch (error) {
  console.log(JSON.stringify({ error: error?.message ?? String(error) }));
  process.exit(1);
} finally {
  await runtime.close();
}
