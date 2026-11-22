/* The mini console's worker — the shell worker's clocks, restated for the
 * third face's two assets (docs/plans/2026-11-21-mini-console.md, phase 2).
 *
 * It exists for two things and refuses a third: an installed console loads
 * its two assets without re-fetching them, a navigation that cannot reach
 * the face gets a readable card, and nothing that carries a session is
 * ever cached — `/api/`, `/events`, `/pair` are network-only by PATH,
 * because the remote face's credential is a cookie the browser attaches
 * after this handler sees the request.
 *
 * The cache name is the build's, not the registration URL's: the server
 * substitutes __FLEET_ASSET_HASH__ with the hash build.rs computed over
 * every served asset, so two builds can never share a cache and an old
 * one can never be served.
 *
 * The clocks are KI-587's, all four: `fetchWithin` measures SILENCE (the
 * timer is cleared when headers arrive, so a slow body is never aborted),
 * a versioned asset that goes quiet is answered from the last generation's
 * cache (`anyVersion`), the prefetch carries the same deadline so install
 * cannot hang, and activation sweeps one generation LATE so the fallback
 * has something to answer with. The page's own watchdog is console.js's.
 */
var VERSION = '__FLEET_ASSET_HASH__';
var CACHE = 'fleet-console-' + VERSION;
var ASSETS = ['/console/console.css?v=' + VERSION, '/console/console.js?v=' + VERSION];
// One line beside the two assets: when this generation was born. A cache
// name is a build hash — a coordinate, not a clock — and the server sends
// no `Date`, so nothing else in the cache can order two generations.
var BORN = '/__fleet-console-born';

var ASSET_DEADLINE_MS = 6000;
var NAV_DEADLINE_MS = 8000;

// A deadline that measures silence, not throughput. The timer is cleared
// the moment headers arrive: `AbortSignal.timeout` stays armed for the
// life of the response and would cancel a transfer that was working.
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
  // The two assets, fetched once at install, under the asset deadline: a
  // stall here would hang install, the new worker would never activate,
  // and the page's new `?v=` would fall through the OLD worker to a path
  // that is not answering.
  event.waitUntil(
    caches
      .open(CACHE)
      .then(function (cache) {
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
      // Claim first, sweep second: once this worker controls a live client,
      // that client's asset requests are compared against THIS worker's
      // VERSION, so it falls through to the network instead of reading a
      // cache we are about to drop.
      await self.clients.claim();
      // Then sweep one generation LATE. Deleting the previous generation in
      // the activation that replaces it empties the only answer a stalled
      // update has (KI-587): the new cache can be empty and the old one
      // gone in the same breath. One generation costs two assets for one
      // release; by the next activation every live client has reloaded.
      var keys = await caches.keys();
      var older = [];
      for (var i = 0; i < keys.length; i += 1) {
        if (keys[i].indexOf('fleet-console-') !== 0 || keys[i] === CACHE) continue;
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
  if (url.pathname !== '/console/console.css' && url.pathname !== '/console/console.js') return false;
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
    // The new coordinate is a cache miss AND the path went quiet: answer
    // with the build the device already has instead of nothing.
    var previous = await anyVersion(request);
    if (previous) return previous;
    throw new Error('fleet console asset deadline expired and no generation is cached');
  }
  var response = attempt.response;
  if (cache && response.status === 200 && response.type === 'basic') {
    cache.put(request, response.clone()).catch(function () {});
  }
  return response;
}

// The same asset at any generation: the cached entry is `?v=<old>` and the
// request is `?v=<new>`, and the two are one file one build apart.
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
    if (keys[i].indexOf('fleet-console-') !== 0) continue;
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
    '<title>fleet console</title><style>' +
    ':root{color-scheme:dark light}' +
    'body{margin:0;min-height:100dvh;display:grid;place-items:center;background:rgb(11,12,14);' +
    'color:rgb(206,211,220);padding:28px;box-sizing:border-box;' +
    'font:13px/1.6 ui-monospace,SFMono-Regular,Menlo,monospace}' +
    'main{max-width:44ch}' +
    'h1{font:inherit;font-weight:600;margin:0 0 10px;color:rgb(240,242,246)}' +
    'p{margin:0 0 14px;color:rgb(158,166,178)}' +
    'a{color:rgb(126,168,255);text-decoration:none;border-bottom:1px solid currentColor}' +
    '</style></head><body><main>' +
    '<h1>fleet console</h1>' +
    '<p>the path is not answering. the run continues on the machine, and nothing was ' +
    'lost: only the console\u2019s two assets are cached, and no session data is.</p>' +
    '<a href="">try again</a></main></body></html>';
  return new Response(html, {
    status: 503,
    statusText: 'Service Unavailable',
    headers: { 'Content-Type': 'text/html; charset=utf-8', 'Cache-Control': 'no-store' },
  });
}

self.addEventListener('fetch', function (event) {
  var request = event.request;
  if (request.method !== 'GET') return;

  var url = new URL(request.url);
  if (url.origin !== self.location.origin) return;

  // The session paths are network-only by name.
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
          // Silence gets the same named page as a refusal: a tab still
          // waiting at the deadline has already failed.
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
