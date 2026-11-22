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
// The one line the activation sweep keeps beside the two assets: when this
// generation was born. A cache name is a build hash — a coordinate, not a
// clock — and the server sends no `Date`, so nothing else in the cache can
// order two generations.
var BORN = '/__fleet-born';

// The worker's two clocks. Both exist for the same failure, and it is not
// the one `unreachable()` was written for: a dropped tailnet or mobile path
// does not reset the connection, it goes quiet. `fetch` then neither
// resolves nor rejects, so a rejection handler never runs and a request
// held by `respondWith` holds the page with it. KI-587 measured the shape
// of that: the shell painted its chrome, `mounted` stayed false, and at
// twenty seconds there was still no card — an update on a dead path froze
// the face permanently, while a *refused* route was answered in 28 ms.
//
// ASSET_DEADLINE_MS is short because there is something better than waiting:
// a versioned asset that goes quiet past it is answered from the last
// generation's cache (`anyVersion`), so a device that updated on a dead path
// boots the build it already had. NAV_DEADLINE_MS is longer because its only
// answer is the retry page, and a slow-but-alive face earns the extra second.
// Both sit under the page's 15 s watchdog (index.html), so the cached answer
// lands first and the card is the net under the net.
var ASSET_DEADLINE_MS = 6000;
var NAV_DEADLINE_MS = 8000;

// A deadline that measures SILENCE, not throughput. The timer is cleared the
// moment headers arrive, so a 1.4 MB script on a slow link is never aborted
// mid-body — `AbortSignal.timeout` stays armed for the life of the response
// and would cancel the stream of a transfer that was working.
async function fetchWithin(request, ms) {
  var controller = new AbortController();
  var expired = false;
  var timer = setTimeout(function () {
    expired = true;
    controller.abort();
  }, ms);
  try {
    var response = await fetch(request, { signal: controller.signal });
    clearTimeout(timer);
    return { response: response, expired: false };
  } catch (error) {
    clearTimeout(timer);
    if (!expired) throw error;
    return { response: null, expired: true };
  }
}

self.addEventListener('install', function (event) {
  // Two small assets, fetched once at install: the shell is usable on the
  // next visit even before the first controlled navigation. The page is
  // not cached — an installed face still reads its session live.
  //
  // The prefetch carries the same deadline, because a stall here is the
  // other half of the freeze: `cache.add` never rejects on silence, so
  // install would hang, the new worker would never activate, and the
  // page's new `?v=` would fall through the OLD worker to a network that
  // is not answering. Missing a prefetch only costs the next load one
  // fetch; hanging costs the face.
  event.waitUntil(
    caches
      .open(CACHE)
      .then(function (cache) {
        // One line beside the two assets: when this generation was born.
        // The activation below keeps one previous generation, and the cache
        // name is a build hash — a coordinate, not a clock — so the sweep
        // needs something it can order. (The server sends no `Date`, so the
        // stored responses cannot supply it.)
        var born = cache
          .put(BORN, new Response(String(Date.now()), { headers: { 'Content-Type': 'text/plain' } }))
          .catch(function () {});
        return born.then(function () {
          return Promise.all(
            ASSETS.map(function (url) {
              return fetchWithin(new Request(url), ASSET_DEADLINE_MS).then(function (attempt) {
                if (attempt.expired || !attempt.response.ok) return null;
                return cache.put(url, attempt.response).catch(function () {});
              }).catch(function () {});
            }),
          );
        });
      })
      .then(function () {
        return self.skipWaiting();
      }),
  );
});

self.addEventListener('activate', function (event) {
  event.waitUntil(
    (async function () {
      // Claim first, sweep second. Once this worker controls a live client,
      // that client's own asset requests are compared against THIS worker's
      // VERSION (`isVersionedAsset`), so it falls through to the network
      // instead of reading a cache we are about to drop. That is the
      // property the diagnosis measured as safe at every viewport: the live
      // client survived the old cache being deleted.
      await self.clients.claim();

      // Then sweep one generation LATE. Deleting the previous generation in
      // the activation that replaces it empties the only answer a stalled
      // update has: KI-587 measured the new cache created empty (its
      // prefetch could not finish) and the old one deleted in the same
      // breath, so `anyVersion` would have had nothing to serve and the
      // shell would still have frozen — the deadline would only have turned
      // an endless wait into a named failure. Keeping one generation costs
      // two assets for one release; by the next activation every live
      // client has had a full generation to reload.
      var keys = await caches.keys();
      var older = [];
      for (var i = 0; i < keys.length; i += 1) {
        if (keys[i].indexOf('fleet-shell-') !== 0 || keys[i] === CACHE) continue;
        older.push({ name: keys[i], born: await bornAt(keys[i]) });
      }
      older.sort(function (a, b) {
        return b.born - a.born;
      });
      for (var j = 0; j < older.length; j += 1) {
        // Keep the newest previous generation, and keep anything we cannot
        // date: never delete a cache we cannot prove is stale.
        if (j === 0 || older[j].born === 0) continue;
        await caches.delete(older[j].name);
      }
    })(),
  );
});

// A generation's birth stamp, or 0 when it has none (a cache written by an
// older worker that predates the stamp).
async function bornAt(name) {
  try {
    var cache = await caches.open(name);
    var stamp = await cache.match(BORN);
    if (!stamp) return 0;
    var value = parseInt(await stamp.text(), 10);
    return isNaN(value) ? 0 : value;
  } catch (error) {
    return 0;
  }
}

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
  var attempt = await fetchWithin(request, ASSET_DEADLINE_MS);
  if (attempt.expired) {
    // The new coordinate is a cache miss AND the path went quiet: the
    // update is what broke this load, so answer with the build the device
    // already has instead of nothing. Failing to the network's error path
    // is the last resort, not the first.
    var previous = await anyVersion(request);
    if (previous) return previous;
    throw new Error('fleet asset deadline expired and no generation is cached');
  }
  var response = attempt.response;
  if (cache && response.status === 200 && response.type === 'basic') {
    cache.put(request, response.clone()).catch(function () {});
  }
  return response;
}

// The same asset at any generation. `ignoreSearch` is the whole trick: the
// cached entry is `/app.js?v=<old>` and the request is `/app.js?v=<new>`,
// and the two are the same file one build apart — the query is exactly the
// coordinate this load no longer trusts. With the activation policy above
// keeping one previous generation there is at most one such cache; the
// loop is written for the case where an older one outlived a sweep.
async function anyVersion(request) {
  var url = new URL(request.url);
  var bare = url.origin + url.pathname;
  var keys;
  try {
    keys = await caches.keys();
  } catch (error) {
    return null;
  }
  for (var i = keys.length - 1; i >= 0; i -= 1) {
    if (keys[i].indexOf('fleet-shell-') !== 0) continue;
    try {
      var cache = await caches.open(keys[i]);
      var hit = await cache.match(bare, { ignoreSearch: true });
      if (hit) return hit;
    } catch (error) {
      // An unreadable cache is not a reason to stop looking.
    }
  }
  return null;
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
    'svg{display:block;margin:0 0 14px}' +
    'h1{font-size:20px;line-height:1.25;margin:0 0 8px;letter-spacing:-.01em}' +
    'p{margin:0 0 16px;opacity:.8}' +
    'code{font:12.5px ui-monospace,SFMono-Regular,Menlo,monospace;background:rgb(24 26 29 / 6%);' +
    'padding:1px 5px;border-radius:4px}' +
    'a{display:inline-block;font-weight:600;color:#1e3e8c;text-decoration:none}' +
    'a:hover{text-decoration:underline}' +
    '</style></head><body><main class="card">' +
    '<svg width="28" height="28" viewBox="0 0 32 32" aria-hidden="true">' +
    '<rect width="32" height="32" rx="7.4" fill="#12151b"></rect>' +
    '<text x="16" y="23.1" text-anchor="middle" font-family="New York, ui-serif, Georgia, serif" ' +
    'font-size="23.5" font-weight="600" fill="#f2efe7">f</text>' +
    '<rect x="5.9" y="25.5" width="20.2" height="1" rx="0.5" fill="#5b8def"></rect></svg>' +
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
      fetchWithin(request, NAV_DEADLINE_MS).then(
        function (attempt) {
          // Silence gets the same named page as a refusal: a tab that is
          // still waiting at the deadline has already failed, and the owner
          // should be told so rather than watching a blank frame.
          if (attempt.expired) return unreachable();
          return attempt.response;
        },
        function () {
          return unreachable();
        },
      ),
    );
  }
});
