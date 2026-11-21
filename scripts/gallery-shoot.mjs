#!/usr/bin/env node
// gallery-shoot.mjs — shoot the doc gallery's surfaces off a running face.
//
// The committed replacement for the /tmp rig the 2026-11-04 gallery was
// shot with (that script is gone; the memories record its recipe).
//
// Usage:
//   node scripts/gallery-shoot.mjs                     # all nine named shots
//   node scripts/gallery-shoot.mjs surface-board       # one shot
//   node scripts/gallery-shoot.mjs --base http://127.0.0.1:3099 surface-huddles
//
// What it expects: the face ALREADY SERVING at --base, seeded by
// scripts/gallery-seed.py and launched from the seed's scratch PROJECT
// (the board reads <cwd>/docs/plans), as
//   cd /tmp/fleet-gallery-project
//   FLEET_HOME=/tmp/fleet-gallery-home ./target/debug/fleet gui --port 3099 --no-open
// The script starts nothing and kills nothing it did not start; it opens
// Chrome, shoots, and closes only the browser it opened.
//
// THE TRAPS (the four the 2026-11-04 shoot recorded, plus one this rig
// found):
//   1. Never put `?bust=` before the hash — the face reads the hash only.
//      This script navigates about:blank -> '/' -> sets location.hash.
//   2. localStorage set after boot does nothing for chrome built once at
//      load (the rail, the fold). Per-shot `localStorage` values are set
//      as init scripts, then the page is RELOADED, and only then is the
//      hash addressed.
//   3. Readiness is polled on the panel the shot is about
//      (`#panel-<route>`), never a generic predicate:
//      `main.stage:not([hidden])` can be the previous surface, stable for
//      seconds. Work surfaces (sessions, huddles) ride the conversation
//      panel, so their readiness is polled on their own column or room
//      head instead — see SHOTS.
//   4. Fixture prose is one paragraph per LINE (the seed's job); this
//      script never rewraps anything.
//   5. A dev build serves a self-unregistering worker at /sw.js whose
//      activate navigates every open client once, right after the first
//      load — the navigation that eats the hash. One reload absorbs it;
//      the script reloads before it ever addresses a route.
//
// :3080 IS THE OWNER'S LIVE CONSOLE. A base whose port is 3080 refuses
// by name, before anything opens.

import { createRequire } from 'node:module';
import path from 'node:path';
import fs from 'node:fs';
import { fileURLToPath } from 'node:url';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, '..');
const require_ = createRequire(import.meta.url);

const args = process.argv.slice(2);
function argValue(name) {
  const at = args.indexOf(name);
  return at >= 0 && args[at + 1] && !args[at + 1].startsWith('--') ? args[at + 1] : undefined;
}
const baseArg = argValue('--base') || 'http://127.0.0.1:3099';
const homeArg = argValue('--home') || '/tmp/fleet-gallery-home';
const outArg = argValue('--out') || path.join(repoRoot, 'docs', 'images');
const qualityArg = Number(argValue('--quality') || 82);
const names = args.filter((one, at) => !one.startsWith('--') && args[at - 1] !== '--base'
  && args[at - 1] !== '--home' && args[at - 1] !== '--out' && args[at - 1] !== '--quality');

// --- the guard ---------------------------------------------------------------

const base = new URL(baseArg);
if (base.protocol !== 'http:' || !['127.0.0.1', 'localhost'].includes(base.hostname)) {
  console.error(`REFUSED: the face is shot over loopback only, not ${base.hostname}`);
  process.exit(2);
}
if (base.port === '3080') {
  console.error('REFUSED: :3080 is the owner\'s live console (the LaunchAgent\'s port). '
    + 'Shoot the scratch face on 3099.');
  process.exit(2);
}

// --- the shots ---------------------------------------------------------------

// Every shot: one fresh browser context (clean localStorage), one page.
// `route` may carry `{room}`, resolved from the seed's gallery-manifest.json.
// `ready` is the readiness predicate; `after` runs extra steps (clicks)
// between readiness and the shutter.
const SHOTS = {
  'surface-sessions': {
    route: 'sessions',
    ready: 'const list = document.getElementById("work-list");'
      + 'const chat = document.getElementById("panel-chat");'
      + 'return !!list && !chat.hidden && list.querySelectorAll("button, li, [data-session]").length > 0;',
  },
  'surface-board': {
    route: 'board',
    ready: 'const p = document.getElementById("panel-board");'
      + 'const board = document.getElementById("board-board");'
      + 'return !!p && !p.hidden && !!board && board.textContent.trim().length > 40;',
  },
  'surface-desk-drawer': {
    route: 'desk',
    ready: 'const p = document.getElementById("panel-desk");'
      + 'return !!p && !p.hidden && p.textContent.trim().length > 40;',
    // The inspector view: open the first deliverable card, then wait for
    // the artifact's own screen before the shutter.
    after: async (page) => {
      const card = page.locator('button.dk-card').first();
      await card.waitFor({ state: 'visible', timeout: 10000 });
      await card.click();
    },
    afterReady: 'const p = document.getElementById("panel-desk");'
      + 'const center = p && p.querySelector(".pw-center");'
      + 'return !!center && center.textContent.trim().length > 40;',
  },
  'surface-huddles': {
    route: 'huddles/{room}',
    // A room rides the conversation panel: readiness is the room head and
    // its member chips, never a generic stage predicate.
    ready: 'const head = document.getElementById("room-head");'
      + 'const members = document.getElementById("room-members");'
      + 'return !!head && !head.hidden && !!members && members.children.length > 0;',
  },
  'surface-bots': {
    route: 'bots',
    ready: 'const p = document.getElementById("panel-bots");'
      + 'return !!p && !p.hidden && p.textContent.trim().length > 40;',
  },
  'surface-router': {
    route: 'router',
    ready: 'const p = document.getElementById("panel-router");'
      + 'const board = document.getElementById("router-board");'
      + 'return !!p && !p.hidden && !!board && board.textContent.trim().length > 40;',
  },
  'surface-delegation': {
    route: 'delegation',
    ready: 'const p = document.getElementById("panel-delegation");'
      + 'const board = document.getElementById("delegation-board");'
      + 'return !!p && !p.hidden && !!board && board.textContent.trim().length > 40;',
  },
  'surface-memory': {
    route: 'memory',
    ready: 'const p = document.getElementById("panel-memory");'
      + 'return !!p && !p.hidden && p.textContent.trim().length > 40;',
  },
  // The phone precedent: 390x844 @2x (the pairing picker's 780x1688).
  // WIRED BUT NOT COMMITTED as of this rig's proof: the shell's mobile
  // tracks collapse the visible stage to 0px at 390 (the room's nodes
  // are all in the DOM, so readiness alone passes — the stage-width
  // guard below is what catches it). The shot fires the moment the
  // shell renders the room at phone width.
  'surface-huddles-phone': {
    route: 'huddles/{room}',
    viewport: { width: 390, height: 844 },
    ready: 'const head = document.getElementById("room-head");'
      + 'const members = document.getElementById("room-members");'
      + 'return !!head && !head.hidden && !!members && members.children.length > 0;',
  },
};

const ALL = Object.keys(SHOTS);
const wanted = names.length ? names : ALL;
for (const name of wanted) {
  if (!SHOTS[name]) {
    console.error(`REFUSED: no shot named \`${name}\`; the shots are:\n  ${ALL.join('\n  ')}`);
    process.exit(2);
  }
}

// --- the manifest ------------------------------------------------------------

const manifestPath = path.join(homeArg, 'gallery-manifest.json');
let room = '';
try {
  const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
  room = manifest.sessions.active_room || '';
} catch (error) {
  console.error(`REFUSED: ${manifestPath} cannot be read (${error.message}) — `
    + 'run scripts/gallery-seed.py first');
  process.exit(2);
}
if (!room) {
  console.error('REFUSED: the manifest names no active room — reseed the home');
  process.exit(2);
}

// --- playwright --------------------------------------------------------------

const pwCandidates = [
  process.env.PLAYWRIGHT_PATH,
  path.join(process.env.HOME || '', '.hermes/hermes-agent/node_modules/playwright-core'),
];
const pwPath = pwCandidates.find((one) => one && fs.existsSync(one));
if (!pwPath) {
  console.error('Playwright is unavailable: no playwright-core at '
    + pwCandidates.filter(Boolean).join(' or ')
    + '. STOP — do not invent a capture.');
  process.exit(2);
}
// playwright-core is a CommonJS package; the require hook loads it from an
// ESM module without a second install.
const { chromium } = require_(pwPath);

// --- the loop ----------------------------------------------------------------

fs.mkdirSync(outArg, { recursive: true });
const browser = await chromium.launch({ channel: 'chrome', headless: true });
let failures = [];
try {
  for (const name of wanted) {
    const shot = SHOTS[name];
    const route = shot.route.replace('{room}', encodeURIComponent(room));
    const viewport = shot.viewport || { width: 1440, height: 900 };
    const context = await browser.newContext({
      viewport,
      deviceScaleFactor: 2,
      ...(shot.locale ? { locale: shot.locale } : {}),
    });
    const page = await context.newPage();
    try {
      // Trap 1: about:blank -> '/' -> the hash, never a query before it.
      await page.goto('about:blank');
      await page.goto(base.origin + '/', { waitUntil: 'load', timeout: 30000 });
      // Trap 5 (found by this rig, not in the memories): a dev build serves
      // a self-unregistering worker at /sw.js whose activate navigates
      // every open client once — the navigation that eats the hash and
      // strands the router on the bare root. One reload absorbs it: after
      // the reload the worker is gone and the page stays put.
      await page.waitForTimeout(2000);
      await page.reload({ waitUntil: 'load', timeout: 30000 });
      await page.waitForTimeout(500);
      // Trap 2: anything the chrome reads at boot is set before the
      // reload, and the hash is addressed only after the reload.
      if (shot.localStorage) {
        for (const [key, value] of Object.entries(shot.localStorage)) {
          await page.evaluate(([k, v]) => window.localStorage.setItem(k, v), [key, value]);
        }
        await context.pages()[0].reload({ waitUntil: 'load' });
        await page.waitForTimeout(2000);
        await page.reload({ waitUntil: 'load' });
        await page.waitForTimeout(500);
      }
      await page.evaluate((address) => { window.location.hash = address; }, `#/${route}`);

      // Trap 3: readiness on the surface the shot is about.
      await page.waitForFunction(new Function('route', shot.ready), route, { timeout: 30000 });
      // Guard: the stage must actually OCCUPY the viewport. The shell's
      // mobile tracks can collapse it to zero width while the readiness
      // predicate still passes (every node is in the DOM) — a shot of a
      // 0-width stage is an empty frame, so fail by name instead.
      const stageWidth = await page.evaluate(() => {
        const stage = document.querySelector('main.stage:not([hidden])');
        return stage ? stage.getBoundingClientRect().width : -1;
      });
      const minStage = (shot.viewport || { width: 1440 }).width < 500 ? 120 : 300;
      if (stageWidth < minStage) {
        throw new Error(`the visible stage is only ${Math.round(stageWidth)}px wide `
          + `(needs >= ${minStage}) — the shell collapsed it at this viewport; not shooting an empty frame`);
      }
      if (shot.after) await shot.after(page);
      if (shot.afterReady) {
        await page.waitForFunction(new Function(shot.afterReady), undefined, { timeout: 30000 });
      } else if (shot.after) {
        await page.waitForTimeout(700);
      }

      const file = path.join(outArg, `${name}.jpg`);
      // `animations` pins CSS transitions so a looping one cannot starve
      // the shutter; `caret` keeps a focused field's caret out of frame.
      await page.screenshot({
        path: file, type: 'jpeg', quality: qualityArg,
        animations: 'disabled', caret: 'hide', timeout: 20000,
      });
      const stat = fs.statSync(file);
      console.log(`shot ${name} -> ${file} (${stat.size} bytes)`);
    } catch (error) {
      failures.push(name);
      console.error(`FAIL ${name}: ${error.message.split('\n')[0]}`);
    } finally {
      await context.close();
    }
  }
} finally {
  // The browser is the only thing this script started; the face on :3099
  // stays up for the next shot.
  await browser.close();
}

const shotNames = wanted.join(', ');
console.log(failures.length
  ? `done with ${failures.length} failure(s) [${failures.join(', ')}] of: ${shotNames}`
  : `done: ${shotNames}`);
process.exit(failures.length ? 1 : 0);
