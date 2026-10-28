// The bot worker's cache boundary — the console worker's rule, restated
// for the bot's assets (plan 2026-10-20-bot-pwa, C16): only the two
// versioned assets are cached, and only by exact URL. The page asks for
// them with ?v=<stamp>; a stamp change is a new URL, so an old cache
// entry can never be served and there is nothing to invalidate. Every
// other request — HTML, /api/, /events, /pair — goes to the network; a
// failed *navigation* falls back to the worker's own retry page, which
// says what is true: the run continues on the machine, and no session
// data lives on the phone.
const CACHE = `fleet-bot-${self.location.search}`;

self.addEventListener('install', (event) => {
  event.waitUntil(self.skipWaiting());
});

self.addEventListener('activate', (event) => {
  event.waitUntil(
    (async () => {
      const names = await caches.keys();
      await Promise.all(
        names.filter((name) => name.startsWith('fleet-bot-') && name !== CACHE)
          .map((name) => caches.delete(name)),
      );
      await self.clients.claim();
    })(),
  );
});

// The retry page, served only when a navigation fails and the shell is
// not already cached. It carries no session data because it carries no
// data at all.
const RETRY = `<!doctype html>
<html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">
<title>fleet bot</title>
<style>
  :root { color-scheme: light dark; }
  body {
    margin: 0; min-height: 100dvh; display: grid; place-items: center;
    background: #f6f4ef; color: #181a1d;
    font: 16px/1.5 -apple-system, BlinkMacSystemFont, "SF Pro Text", sans-serif;
  }
  @media (prefers-color-scheme: dark) { body { background: #0e0f12; color: #ebe7e4; } }
  main { text-align: center; padding: 24px; max-width: 34ch; }
  .mark {
    width: 56px; height: 56px; margin: 0 auto 16px; display: grid; place-items: center;
    border-radius: 16px; color: #fff; font: 600 32px/1 Georgia, "New York", serif;
    background: linear-gradient(135deg, rgb(63,108,218), rgb(37,78,176));
  }
  h1 { font: 500 24px/1.2 Georgia, "New York", serif; margin: 0 0 8px; }
  p { margin: 0 0 18px; color: inherit; opacity: .72; }
  button {
    min-height: 46px; padding: 0 26px; border: 0; border-radius: 12px;
    background: rgb(47,95,208); color: #fff; font: 650 15px/1 inherit;
  }
  small { display: block; margin-top: 14px; font-size: 11px; opacity: .5; }
</style></head><body><main>
  <div class="mark">f</div>
  <h1>Can't reach fleet</h1>
  <p>The machine is offline, or the tailnet dropped. Nothing was lost — the run keeps going without you.</p>
  <button onclick="location.reload()">Try again</button>
  <small>This page came from the app itself, not the network. Only the two versioned assets are cached; no session data is.</small>
</main></body></html>`;

self.addEventListener('fetch', (event) => {
  const url = new URL(event.request.url);
  // Streaming and API responses never touch the cache: a cached SSE
  // buffer is a lie about a live run.
  if (url.pathname.startsWith('/api/') || url.pathname === '/events' || url.pathname.startsWith('/pair')) {
    return;
  }
  // The two versioned assets, by exact URL: cache-first is safe because
  // the URL changes with the build.
  const versioned =
    url.pathname === '/bot.css' || url.pathname === '/bot.js';
  if (versioned && url.searchParams.get('v')) {
    event.respondWith(
      (async () => {
        const cache = await caches.open(CACHE);
        const hit = await cache.match(event.request);
        if (hit) return hit;
        const response = await fetch(event.request);
        if (response.ok) {
          await cache.put(event.request, response.clone());
        }
        return response;
      })(),
    );
    return;
  }
  // A navigation that cannot reach the machine gets the retry page;
  // everything else is network-only.
  if (event.request.mode === 'navigate') {
    event.respondWith(
      (async () => {
        try {
          return await fetch(event.request);
        } catch (error) {
          return new Response(RETRY, {
            status: 200,
            headers: { 'Content-Type': 'text/html; charset=utf-8' },
          });
        }
      })(),
    );
  }
});
