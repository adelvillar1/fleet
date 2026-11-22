# The bot PWA probe

The rig KI-588 was closed on. It serves **both faces on one origin** from
the repo's own assets, so the console's worker and the bot's can be
registered side by side and the browser can be asked what actually
controls what — then it measures the four acceptance cells in a real
Chrome at 390x844.

The Rust server's own routing and headers are pinned by
`src/remote/listener.rs`'s tests over a real listener; this rig covers
what those cannot: service-worker scope, claim, cache contents, the
offline page, and the layout at phone size.

## Running it

```sh
python3 scripts/bot-pwa-probe/serve.py --port 3196 --hash h1 &
python3 scripts/bot-pwa-probe/probe.py after     # the installed app (/bot/)
python3 scripts/bot-pwa-probe/probe.py baseline  # the pre-fix page (/bot.html)
python3 scripts/bot-pwa-probe/update.py          # the hash bump
```

Requires `playwright` (`pip install playwright`) and a Chrome to drive
(`--channel chrome`, the default; pass `--channel chromium` if the
Playwright bundle is installed). `probe.py` also accepts `--base`.

`serve.py` reads `src/gui/assets/` live, so a CSS or worker edit is
measured without a build, and substitutes `__FLEET_ASSET_HASH__` exactly
as the Rust server does at serve time.

## What each cell reads

| cell | what is measured |
|---|---|
| (i) registration | `navigator.serviceWorker.controller`, every registration's scope and script, `caches.keys()` and each cache's entry count — at install, not after the first fetch |
| (ii) offline | with the context offline: the page's title, `h1`, text and the controlling worker — for the bot **and** for the console on the same origin, so the two retry pages can be told apart |
| (iii) update | `/bump?to=h2`, reload, then the same state plus `document.readyState`, the inbox rows, and a real tap on the composer |
| (iv) layout | `.conv`/`.scroll`/`.dock`/composer rects, `.scroll` client vs content height, `elementFromPoint` on the composer, the focus target after a tap — and the same numbers with `.conv` forced back to `display:block` (the red-proof) |

## The numbers it measured (2026-11-21, Chrome 158)

| | result |
|---|---|
| (i) | bot page controlled by `/bot/bot-sw.js?v=h1`; `regs` = `[/` → `/sw.js`, `/bot/` → `/bot/bot-sw.js?v=h1`]`; `fleet-bot-h1` holds 2 entries at install (the console's `fleet-shell-h1` also present) |
| (ii) | bot offline → "Can't reach fleet / the run keeps going without you", controlled by `/bot/bot-sw.js`; console offline → "fleet is not reachable", controlled by `/sw.js` |
| (iii) | `fleet-bot-h1` → `fleet-bot-h2` (old deleted), `fleet-shell-h1` intact, `readyState complete`, 2 rows, composer focused on tap, dock at 844 |
| (iv) | `.conv` 0..844, `.scroll` client 654 / content 4528, `.dock` 757..844, composer 768..830, `elementFromPoint` = `steer`; with the rule removed: `.dock` 4631..4718, composer 4642..4704, `elementFromPoint` = a transcript turn |
