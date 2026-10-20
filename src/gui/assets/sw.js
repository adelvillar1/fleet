/* The fleet shell worker.
 *
 * It exists for two things and refuses a third: an installed face loads
 * its two assets without re-fetching them, a navigation that cannot reach
 * the face gets a readable retry page, and nothing that carries a session
 * is ever cached. The remote face's credential is a cookie the browser
 * attaches by itself, so the boundary is by PATH — `/api/`, `/events`,
 * `/pair` — not by an Authorization header (the Qwen worker's test would
 * miss fleet's cookie entirely).
 *
 * Only the two assets, and only at the current build's `?v=` address, are
 * intercepted. Everything else falls through to the network untouched.
 *
 * __FLEET_ASSET_HASH__ is replaced by the server at serve time with the
 * hash build.rs computed over the served assets.
 */

var VERSION = '__FLEET_ASSET_HASH__';
var CACHE = 'fleet-shell-' + VERSION;
var ASSETS = ['/app.css?v=' + VERSION, '/app.js?v=' + VERSION];

self.addEventListener('install', function (event) {
  // Two small assets, fetched once at install: the shell is usable on the
  // next visit even before the first controlled navigation. The page is
  // not cached — an installed face still reads its session live.
  event.waitUntil(
    caches
      .open(CACHE)
      .then(function (cache) {
        return Promise.all(
          ASSETS.map(function (url) {
            return cache.add(url).catch(function () {});
          }),
        );
      })
      .then(function () {
        return self.skipWaiting();
      }),
  );
});

self.addEventListener('activate', function (event) {
  event.waitUntil(
    caches
      .keys()
      .then(function (keys) {
        return Promise.all(
          keys
            .filter(function (key) {
              return key.indexOf('fleet-shell-') === 0 && key !== CACHE;
            })
            .map(function (key) {
              return caches.delete(key);
            }),
        );
      })
      .then(function () {
        return self.clients.claim();
      }),
  );
});

function isVersionedAsset(url) {
  if (url.origin !== self.location.origin) return false;
  if (url.pathname !== '/app.css' && url.pathname !== '/app.js') return false;
  return url.searchParams.get('v') === VERSION;
}

async function fromCacheOrNetwork(request) {
  var cache;
  try {
    cache = await caches.open(CACHE);
    var hit = await cache.match(request);
    if (hit) return hit;
  } catch (error) {
    // A disabled or full browser cache must not stop the face loading.
  }
  var response = await fetch(request);
  if (cache && response.status === 200 && response.type === 'basic') {
    cache.put(request, response.clone()).catch(function () {});
  }
  return response;
}

function unreachable() {
  var html =
    '<!doctype html><html lang="en"><head><meta charset="utf-8">' +
    '<meta name="viewport" content="width=device-width,initial-scale=1">' +
    '<title>fleet is not reachable</title><style>' +
    ':root{color-scheme:light dark}' +
    'html,body{margin:0;min-height:100%}' +
    'body{min-height:100dvh;display:grid;place-items:center;background:#f6f4ef;color:#181a1d;' +
    'font:15px/1.55 ui-sans-serif,-apple-system,system-ui,sans-serif;padding:32px;box-sizing:border-box}' +
    '@media (prefers-color-scheme:dark){body{background:#0a0c10;color:#e8eef4}' +
    '.card{border-color:rgb(255 255 255 / 15%)}a{color:#8fb2ff}}' +
    '.card{max-width:520px;border:1px solid rgb(24 26 29 / 12%);border-radius:12px;padding:22px 24px;' +
    'background:inherit}' +
    'svg{display:block;margin:0 0 14px;opacity:.85}' +
    'h1{font-size:20px;line-height:1.25;margin:0 0 8px;letter-spacing:-.01em}' +
    'p{margin:0 0 16px;opacity:.8}' +
    'code{font:12.5px ui-monospace,SFMono-Regular,Menlo,monospace;background:rgb(24 26 29 / 6%);' +
    'padding:1px 5px;border-radius:4px}' +
    'a{display:inline-block;font-weight:600;color:#1e3e8c;text-decoration:none}' +
    'a:hover{text-decoration:underline}' +
    '</style></head><body><main class="card">' +
    '<svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" ' +
    'stroke-width="1.6" stroke-linecap="round" aria-hidden="true">' +
    '<path d="M4 6.5h11"></path><path d="M4 12h16"></path><path d="M4 17.5h8"></path></svg>' +
    '<h1>fleet is not reachable</h1>' +
    '<p>The face is not answering on this address. Start it with <code>fleet gui</code>, ' +
    'then reload this page. Nothing you had open was saved in this shell — the conversation ' +
    'lives in the session log, and the face reads it again when it is back.</p>' +
    '<a href="">Try again</a></main></body></html>';
  return new Response(html, {
    status: 503,
    statusText: 'Service Unavailable',
    headers: {
      'Content-Type': 'text/html; charset=utf-8',
      'Cache-Control': 'no-store',
    },
  });
}

self.addEventListener('fetch', function (event) {
  var request = event.request;
  if (request.method !== 'GET') return;

  var url = new URL(request.url);
  if (url.origin !== self.location.origin) return;

  // The session paths are network-only by name. `url.pathname` is used
  // rather than a header because the remote face's device token rides a
  // cookie, which the browser adds after this handler sees the request.
  if (
    url.pathname === '/events' ||
    url.pathname === '/pair' ||
    url.pathname.indexOf('/api/') === 0
  ) {
    return;
  }
  if ((request.headers.get('Accept') || '').indexOf('text/event-stream') !== -1) {
    return;
  }

  if (isVersionedAsset(url)) {
    event.respondWith(fromCacheOrNetwork(request));
    return;
  }

  if (request.mode === 'navigate') {
    event.respondWith(
      fetch(request).catch(function () {
        return unreachable();
      }),
    );
  }
});
