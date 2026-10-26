// layerfs-register.mjs — C2 of docs/plans/2026-10-25-layerfs-seamless.md.
//
// Registers one folder as a LayerFS project on the machine's durable
// home: the runtime's `start_init` (the sanctioned macOS route — a
// container of its own with the source mounted read-only), polled to
// its end, answering one JSON line the driver records:
//   { "name", "volume", "store", "wallMs", "scanned" }
//
// Usage (the driver spawns it; the bundle's runtime is imported by
// absolute path, so nothing resolves from this file's location):
//   node layerfs-register.mjs <name> <source-folder> <home> <bundle> <image>
//
// One attempt, like every LayerFS operation: a failed Init removes
// what it made; an uncertain one is reported with the runtime's own
// record path and nothing is cleaned on a guess.
import { join } from "node:path";

const [name, source, home, bundle, image] = process.argv.slice(2);
if (!name || !source || !home || !bundle || !image) {
  console.log(
    JSON.stringify({ error: "usage: node layerfs-register.mjs <name> <source> <home> <bundle> <image>" })
  );
  process.exit(2);
}
// The runtime is imported by ABSOLUTE path from the sealed bundle —
// its own internal imports (@layerfs/agent) then resolve inside the
// bundle's node_modules, and nothing is resolved from this file's
// location (which has no node_modules of its own).
const { LayerFSRuntime } = await import(join(bundle, "api/runtime/src/index.js"));
const runtime = await LayerFSRuntime.open({ home, image, bundle });
try {
  const t0 = Date.now();
  const job = await runtime.project.start_init(name, source);
  const id = job?.id ?? job;
  let last = null;
  for (let i = 0; i < 2400; i++) {
    const st = await runtime.project.get_init(id);
    if (st === null || st?.ended) { last = st; break; }
    if (st?.error) {
      console.log(JSON.stringify({ error: st.error }));
      process.exit(1);
    }
    last = st;
    await new Promise((r) => setTimeout(r, 250));
  }
  const found = (await runtime.project.list()).find((p) => p.name === name);
  if (!found?.volume) {
    console.log(JSON.stringify({ error: "the runtime recorded no volume for the project" }));
    process.exit(1);
  }
  console.log(
    JSON.stringify({
      name,
      volume: found.volume,
      store: `/var/lib/docker/volumes/${found.volume}/_data/global/store.sqlite`,
      wallMs: Date.now() - t0,
      scanned: last?.scanned ?? null,
    })
  );
} finally {
  await runtime.close();
}
