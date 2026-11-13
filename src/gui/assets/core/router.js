// core/router.js — the one address grammar (P2 / P3 / U47).
//
// Every surface reads the same `#/<lens>/<id>` shape through this single
// parser: the lens is the first segment, the focused object the rest
// (decoded, inner slashes kept).
export function parseAddress() {
  var raw = (window.location.hash || '').replace(/^#\/?/, '');
  var parts = raw.split('/');
  return {
    lens: parts[0] || '',
    id: parts.length > 1 ? decodeURIComponent(parts.slice(1).join('/')) : '',
    parts: parts
  };
}
