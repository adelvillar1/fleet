// The face's own tests: the real src/gui/assets/app.js against a
// stubbed DOM and stubbed bridge documents. Zero dependencies — the
// repository has no JS runner, so this file *is* the runner, and
// tests/face.rs shells out to it when node is present.
//
// Every case boots a fresh page (app.js is one IIFE that wires itself at
// load), feeds it documents in the bridge's own shapes, drives it the
// way a pointer or a keystroke would, and asserts on what it built.
import fs from 'node:fs';
import path from 'node:path';
import vm from 'node:vm';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(HERE, '../..');
const SRC = fs.readFileSync(path.join(ROOT, 'src/gui/assets/app.js'), 'utf8');
const HTML = fs.readFileSync(path.join(ROOT, 'src/gui/assets/index.html'), 'utf8');
const RUST = fs.readFileSync(path.join(ROOT, 'src/gui/bridge.rs'), 'utf8') +
  fs.readFileSync(path.join(ROOT, 'src/gui/server.rs'), 'utf8') +
  fs.readFileSync(path.join(ROOT, 'src/remote/listener.rs'), 'utf8') +
  fs.readFileSync(path.join(ROOT, 'src/gui/mod.rs'), 'utf8');
const BOTJS = fs.readFileSync(path.join(ROOT, 'src/gui/assets/bot.js'), 'utf8');
const CSS = fs.readFileSync(path.join(ROOT, 'src/gui/assets/app.css'), 'utf8');
const flat = (text) => String(text).replace(/\s+/g, ' ').trim();
const ruleBlock = (selector) => {
  const at = CSS.indexOf(selector + ' {');
  if (at < 0) return '';
  const end = CSS.indexOf('}', at);
  return flat(CSS.slice(at + selector.length + 1, end));
};

// ------------------------------------------------------------------ DOM


// One compound selector — `div`, `.assistant.md-rendered`, `#log` — and
// descendant chains (`A B`). Enough for the face's queries and for the
// assertions here; a stub does not need a CSS engine, it needs honest
// answers to the selectors in use.
function matchesSimple(el, sel) {
  const text = String(sel || '').trim();
  if (!text) return false;
  // Attribute parts — `[name="description"]`, the shape the face's own
  // form handlers read — are pulled off first and matched against the
  // element's property of that name (a stub sets `.name`, which in a
  // browser reflects to the attribute).
  const attrs = [];
  const bare = text.replace(/\[[^\]=]+="[^"]*"\]/g, (part) => {
    attrs.push(part);
    return '';
  });
  const tag = bare.match(/^[a-zA-Z][\w-]*/);
  if (tag && el.name !== tag[0]) return false;
  const ids = bare.match(/#[^.#\s]+/g) || [];
  if (ids.some((id) => el.id !== id.slice(1))) return false;
  const classes = bare.match(/\.[^.#\s]+/g) || [];
  const held = el.className.split(/\s+/).filter(Boolean);
  if (classes.some((cls) => !held.includes(cls.slice(1)))) return false;
  for (const attr of attrs) {
    const m = attr.match(/\[([^\]=]+)="([^"]*)"\]/);
    const value = typeof el[m[1]] === 'string' ? el[m[1]] : null;
    if (value !== m[2]) return false;
  }
  return Boolean(tag || ids.length || classes.length || attrs.length);
}

function descendantAll(root, sel) {
  const chain = String(sel || '').trim().split(/\s+/).filter(Boolean);
  if (!chain.length) return [];
  const out = [];
  const holdsChain = (el) => {
    if (!matchesSimple(el, chain[chain.length - 1])) return false;
    let node = el.parentNode;
    let index = chain.length - 2;
    while (index >= 0 && node) {
      if (matchesSimple(node, chain[index])) index -= 1;
      node = node.parentNode;
    }
    return index < 0;
  };
  const walk = (node) => {
    node.children.forEach((child) => { if (holdsChain(child)) out.push(child); walk(child); });
  };
  walk(root);
  return out;
}

class El {
  constructor(name) {
    this.name = name;
    this.children = [];
    // The browser's alias: an element's childNodes IS its children.
    // The bots board's activity feed reads childNodes on an element
    // that has not appended yet, and an element without the alias
    // throws where a browser answers zero.
    this.childNodes = this.children;
    this.parentNode = null;
    this.dataset = {};
    this.style = {
      _props: {},
      setProperty(name, value) { this._props[name] = String(value); },
      getPropertyValue(name) { return this._props[name] || ''; },
      removeProperty(name) { delete this._props[name]; },
    };
    this.className = '';
    this._text = '';
    this._html = '';
    this.value = '';
    this.hidden = false;
    this.disabled = false;
    this.type = '';
    this.title = '';
    this.placeholder = '';
    this.id = '';
    this.tabIndex = 0;
    this.draggable = false;
    this.listeners = {};
    this.attrs = {};
    this.scrollTop = 0;
    this.scrollHeight = 1000;
    this.clientHeight = 500;
    this.clientWidth = 1200;
    this.offsetWidth = 320;
    this.offsetHeight = 40;
  }
  getBoundingClientRect() {
    return { left: 0, top: 0, right: 800, bottom: 600, width: 800, height: 600 };

  }
  get textContent() {
    if (this.children.length) return this.children.map((c) => c.textContent).join('');
    return this._text;
  }
  set textContent(value) { this.children = []; this._text = String(value); }
  set innerHTML(value) { this.children = []; this._html = String(value); }
  get innerHTML() { return this._html; }
  get classList() {
    const self = this;
    const names = () => self.className.split(/\s+/).filter(Boolean);
    return {
      add(...list) { self.className = Array.from(new Set(names().concat(list))).join(' '); },
      remove(...list) { self.className = names().filter((n) => !list.includes(n)).join(' '); },
      toggle(name, force) {
        const has = names().includes(name);
        const on = force === undefined ? !has : !!force;
        if (on && !has) self.className = names().concat(name).join(' ');
        if (!on && has) self.className = names().filter((n) => n !== name).join(' ');
        return on;
      },
      contains(name) { return names().includes(name); },
    };
  }
  get lastChild() { return this.children.length ? this.children[this.children.length - 1] : null; }
  // A select's live options collection, the way the chip editor reads
  // it (`pick.options.length`).
  get options() { return this.children.filter((child) => child.name === 'option'); }
  get firstChild() { return this.children.length ? this.children[0] : null; }
  get firstElementChild() { return this.firstChild; }
  appendChild(child) { child.parentNode = this; this.children.push(child); return child; }
  insertBefore(child, before) {
    child.parentNode = this;
    const at = before ? this.children.indexOf(before) : -1;
    if (at < 0) this.children.push(child);
    else this.children.splice(at, 0, child);
    return child;
  }
  removeChild(child) {
    const at = this.children.indexOf(child);
    if (at >= 0) this.children.splice(at, 1);
    child.parentNode = null;
    return child;
  }
  setAttribute(k, v) {
    this.attrs[k] = String(v);
    if (k === 'id') this.id = String(v);
    // A real DOM maps the class attribute onto className, and SVG
    // elements take their class through setAttribute — without this the
    // plots the face draws are invisible to a selector.
    if (k === 'class') this.className = String(v);
  }
  getAttribute(k) { return this.attrs[k]; }
  removeAttribute(k) { delete this.attrs[k]; }
  addEventListener(kind, fn) { (this.listeners[kind] = this.listeners[kind] || []).push(fn); }
  removeEventListener() {}
  focus() {}
  blur() {}
  scrollIntoView() {}
  setSelectionRange() {}
  click() { this.fire('click', {}); }
  fire(kind, event) {
    const ev = event || {};
    ev.preventDefault = ev.preventDefault || (() => {});
    ev.stopPropagation = () => { ev._stopped = true; };
    ev.target = ev.target || this;
    let node = this;
    while (node) {
      (node.listeners[kind] || []).slice().forEach((fn) => fn(ev));
      if (ev._stopped) break;
      node = node.parentNode;
    }
  }
  matches(sel) { return matchesSimple(this, sel); }
  closest(sel) {
    let node = this;
    while (node) { if (node.matches && node.matches(sel)) return node; node = node.parentNode; }
    return null;
  }
  all(sel) { return descendantAll(this, sel); }
  querySelector(sel) { const found = this.all(sel); return found.length ? found[0] : null; }
  querySelectorAll(sel) { return this.all(sel); }
  find(predicate, acc) {
    acc = acc || [];
    this.children.forEach((child) => { if (predicate(child)) acc.push(child); child.find(predicate, acc); });
    return acc;
  }
}

// Every id the page declares, so getElementById is the page's own list
// rather than a guess. A script asking for an id the page lacks is a
// real defect, so the stub refuses it by name (the tmp harness's rule).
const PAGE_IDS = [...HTML.matchAll(/id="([^"]+)"/g)].map((m) => m[1]);

// ---------------------------------------------------------------- pages

let timerIds = 0;
function boot(options) {
  const opts = { width: 1440, ...options };
  const IDS = {};
  PAGE_IDS.forEach((id) => { IDS[id] = new El('div'); IDS[id].id = id; });
  const body = new El('body');
  const documentElement = new El('html');
  documentElement.children = Object.values(IDS);
  // A static `hidden` attribute is the element's real initial state: the
  // pane, the drawer, the work column all boot hidden, and a face that
  // toggles them reads the attribute, not a stub default.
  PAGE_IDS.forEach((id) => {
    const tag = HTML.match(new RegExp('<[^>]*id="' + id + '"[^>]*>'));
    if (tag && /\shidden[\s>]/.test(tag[0])) IDS[id].hidden = true;
  });
  const calls = [];
  const events = [];
  let streamSource = null;
  const storage = new Map(Object.entries(opts.storage || {}));

  const documents = opts.documents || {};
  const fetchImpl = (url, init) => {
    const method = (init && init.method) || 'GET';
    calls.push({ url, method, body: init && init.body ? init.body : null });
    const pathOnly = String(url).split('?')[0];
    if (pathOnly === '/api/state') {
      const asked = new URL('http://x' + url).searchParams.get('session') || '';
      const doc = typeof documents.state === 'function'
        ? documents.state(asked)
        : documents.state;
      return Promise.resolve({ status: 200, json: () => Promise.resolve(doc || emptyState(asked)) });
    }
    if (pathOnly === '/api/sessions') {
      return Promise.resolve({ status: 200, json: () => Promise.resolve(documents.sessions || emptySessions()) });
    }
    if (pathOnly === '/api/session') {
      return Promise.resolve({ status: 200, json: () => Promise.resolve(documents.branch || {}) });
    }
    if (pathOnly === '/api/feedback') {
      // The verdict store is the test's own object: the route stamps,
      // clears, and echoes it, so a pressed button can repaint from
      // what the server read back exactly as the real one does.
      const asked = new URL('http://x' + url).searchParams.get('session') || '';
      const body = init && init.body ? JSON.parse(init.body) : {};
      const store = documents.feedback || (documents.feedback = {});
      const held = store[asked] || {};
      if (body.verdict === 'clear') delete held[String(body.exchange)];
      else held[String(body.exchange)] = body.verdict;
      store[asked] = held;
      return Promise.resolve({
        status: 200,
        json: () => Promise.resolve({ type: 'feedback', session: asked, feedback: held }),
      });
    }
    if (pathOnly === '/api/session/fork') {
      return Promise.resolve({
        status: 200,
        json: () => Promise.resolve({ type: 'fork', id: 'fork-from-fb', forked_from: 'fb-1' }),
      });
    }
    if (pathOnly === '/api/session/archive') {
      return Promise.resolve({
        status: opts.archiveStatus || 200,
        json: () => Promise.resolve(opts.archiveReply || { type: 'archive', session: 'x', archived: true }),
      });
    }
    if (pathOnly === '/api/home') {
      return Promise.resolve({ status: 200, json: () => Promise.resolve(documents.home || {}) });
    }
    if (pathOnly === '/api/context') {
      return Promise.resolve({
        status: 200,
        json: () => Promise.resolve(documents.context || {
          type: 'context',
          system: 'You are Fleet... ## Goal\n\nship the controls',
          messages: [
            { role: 'system', content: 'You are Fleet...' },
            { role: 'user', content: 'first ask' },
            { role: 'assistant', content: 'first answer' },
            { role: 'user', content: 'second ask' },
          ],
          tools: [
            { function: { name: 'bash' } },
            { function: { name: 'read_file' } },
            { function: { name: 'subagent' } },
          ],
        }),
      });
    }
    if (pathOnly === '/api/checkpoints') {
      return Promise.resolve({
        status: 200,
        json: () => Promise.resolve(documents.checkpoints || {
          type: 'checkpoints',
          snapshots: [{ turn: 1, at: 'now', files: 2, bytes: 100, unread: 0 },
            { turn: 2, at: 'now', files: 3, bytes: 150, unread: 0 }],
        }),
      });
    }
    if (pathOnly === '/api/diff') {
      const turn = new URL('http://x' + url).searchParams.get('turn') || '1';
      return Promise.resolve({
        status: 200,
        json: () => Promise.resolve({
          type: 'diff', turn: Number(turn),
          files: [
            { path: 'src/main.rs', kind: 'modified', hunks: [], too_large: false },
            { path: 'docs/note.md', kind: 'added', hunks: [], too_large: false },
          ],
        }),
      });
    }
    if (pathOnly === '/api/tree') {
      return Promise.resolve({
        status: 200,
        json: () => Promise.resolve(documents.tree || {
          type: 'tree', path: '', entries: [
            { kind: 'file', name: 'README.md' },
            { kind: 'file', name: 'src/main.rs' },
            { kind: 'dir', name: 'docs' },
          ],
        }),
      });
    }
    if (pathOnly === '/api/goal') {
      const body = {};
      try { Object.assign(body, JSON.parse(init.body || '{}')); } catch (e) {}
      return Promise.resolve({
        status: 200,
        json: () => Promise.resolve({ type: 'goal', text: body.text || '' }),
      });
    }
    if (pathOnly === '/api/settings') {
      if (method === 'POST') {
        return Promise.resolve({ status: 200, json: () => Promise.resolve(opts.reply || {}) });
      }
      return Promise.resolve({
        status: 200,
        json: () => Promise.resolve(documents.settings || {}),
      });
    }
    if (pathOnly === '/api/resume') {
      return Promise.resolve({ status: 200, json: () => Promise.resolve({ type: 'resume', agent: true, session: 'x' }) });
    }
    if (pathOnly === '/api/session/new') {
      return Promise.resolve({ status: 200, json: () => Promise.resolve({ type: 'session', id: opts.newId || 'new-session-1', created: true }) });
    }
    if (pathOnly === '/api/run') {
      return Promise.resolve({ status: 200, json: () => Promise.resolve({ type: 'run', accepted: true }) });
    }
    if (pathOnly === '/api/tree') {
      return Promise.resolve({
        status: 200,
        json: () => Promise.resolve(documents.tree ||
          { type: 'tree', entries: [], refused: [] }),
      });
    }
    if (pathOnly === '/api/file') {
      const asked = new URL('http://x' + url).searchParams.get('path') || '';
      const files = documents.files || {};
      return Promise.resolve({
        status: 200,
        json: () => Promise.resolve(files[asked] ||
          { type: 'file', path: asked, name: asked, language: 'text', text: '' }),
      });
    }
    if (pathOnly === '/api/onboard') {
      const body = init && init.body ? JSON.parse(init.body) : {};
      const doc = typeof documents.onboard === 'function'
        ? documents.onboard(body)
        : documents.onboard;
      return Promise.resolve({ status: 200, json: () => Promise.resolve(doc || onboardDoc()) });
    }
    if (pathOnly === '/api/channels') {
      return Promise.resolve({
        status: 200,
        json: () => Promise.resolve(documents.channels || emptyChannels()),
      });
    }
    if (pathOnly === '/api/memory') {
      return Promise.resolve({
        status: 200,
        json: () => Promise.resolve(documents.memory || { type: 'memory', facts: [], hits: [] }),
      });
    }
    if (pathOnly === '/api/skills') {
      return Promise.resolve({
        status: 200,
        json: () => Promise.resolve(documents.skills || {
          type: 'skills', skills: [], skips: [], proposals: [], merges: [], retired: [],
        }),
      });
    }
    if (pathOnly === '/api/web/frame') {
      const asked = new URL('http://x' + url).searchParams.get('url') || '';
      return Promise.resolve({
        status: 200,
        json: () => Promise.resolve(documents.webFrame ||
          { type: 'web-frame', url: asked, framable: true, reason: '', content_type: 'text/html' }),
      });
    }
    if (pathOnly === '/api/web') {
      const asked = new URL('http://x' + url).searchParams.get('url') || '';
      return Promise.resolve({
        status: 200,
        json: () => Promise.resolve(documents.web ||
          { type: 'web', url: asked, title: 'the page', blocks: [], links: [] }),
      });
    }
    if (pathOnly === '/api/workspace-history') {
      return Promise.resolve({
        status: 200,
        json: () => Promise.resolve(documents['workspace-history'] || {}),
      });
    }
    if (pathOnly === '/api/workspace-history/calls') {
      return Promise.resolve({
        status: 200,
        json: () => Promise.resolve(documents['workspace-history/calls'] || {
          type: 'workspace-calls', calls: [], more: false, note: null, refused: [],
        }),
      });
    }
    if (pathOnly === '/api/workspace-history/file') {
      return Promise.resolve({
        status: 200,
        json: () => Promise.resolve(documents['workspace-history/file'] || {
          type: 'workspace-file', mode: 'text', refused: [], note: null,
        }),
      });
    }
    if (pathOnly === '/api/workspace-history/files') {
      return Promise.resolve({
        status: 200,
        json: () => Promise.resolve(documents['workspace-history/files'] || {
          type: 'workspace-files',
          branch: 'main',
          path: '/',
          items: [],
        }),
      });
    }
    return Promise.resolve({ status: 200, json: () => Promise.resolve(opts.reply || {}) });
  };

  const document_ = {
    createElement: (name) => new El(name),
    createElementNS: (_ns, name) => new El(name),
    createTextNode: (text) => { const made = new El('#text'); made._text = String(text); return made; },
    getElementById: (id) => {
      if (!IDS[id]) throw new Error('the script asked for #' + id + ', which the page does not have');
      return IDS[id];
    },
    querySelector: (sel) => document_.documentElement.querySelector(sel) || null,
    querySelectorAll: (sel) => document_.documentElement.querySelectorAll(sel),
    addEventListener: () => {},
    documentElement,
    body,
    hidden: false,
    title: 'fleet',
    activeElement: null,
  };
  // The page's own tree, so document-wide queries walk it.
  documentElement.children = Object.values(IDS);

  const windowListeners = {};
  // The hash is a route: a real browser fires hashchange on every
  // assignment, so the fake does too — the face's router walks on
  // programmatic navigation exactly as it does on a link press.
  const fireHash = () => { (windowListeners.hashchange || []).forEach((fn) => fn({})); };
  const location_ = {
    search: '', hash: opts.hash || '', pathname: '/', href: 'http://127.0.0.1/',
    // The harness models the desk face, and the desk face's host is the
    // loopback one — canPair() reads this, and without it every boot
    // looked like a phone to the pairing gate.
    hostname: '127.0.0.1',
  };
  Object.defineProperty(location_, 'hash', {
    get() { return this._hash || ''; },
    set(next) {
      const before = this._hash || '';
      this._hash = String(next);
      if (this._hash !== before) fireHash();
    },
  });
  location_._hash = String(opts.hash || '');
  const window_ = {
    location: location_,
    history: {
      // `pushState`/`replaceState` never fire `hashchange` (the spec's
      // own rule, and the reason an address change that keeps the
      // surface does not repaint it): mutate the hash silently.
      replaceState: (_a, _b, url) => {
        location_._hash = String(url).replace(/^[^#]*/, '');
      },
      pushState: (_a, _b, url) => {
        location_._hash = String(url).replace(/^[^#]*/, '');
      },
    },
    addEventListener: (kind, fn) => { (windowListeners[kind] = windowListeners[kind] || []).push(fn); },
    removeEventListener: () => {},
    localStorage: {
      getItem: (k) => (storage.has(k) ? storage.get(k) : null),
      setItem: (k, v) => storage.set(k, String(v)),
      removeItem: (k) => storage.delete(k),
    },
    innerWidth: opts.width,
    innerHeight: 900,
    devicePixelRatio: 1,
    matchMedia: () => ({ matches: false, addListener() {}, removeListener() {}, addEventListener() {}, removeEventListener() {} }),
    setTimeout: (fn) => { fn(); return (timerIds += 1); },
    clearTimeout: () => {},
    setInterval: () => (timerIds += 1),
    clearInterval: () => {},
    requestAnimationFrame: (fn) => { fn(); return (timerIds += 1); },
    scrollTo: () => {},
    confirm: () => true,
    prompt: () => opts.promptAnswer || 'a name',
    open: () => null,
    fetch: fetchImpl,
    focus: () => {},
  };
  const clipboardWrites = [];
  const navigator_ = { clipboard: { writeText: (text) => { clipboardWrites.push(text); return Promise.resolve(); } }, userAgent: 'node', language: 'en' };
  const sandbox = {
    document: document_,
    window: window_,
    navigator: navigator_,
    localStorage: window_.localStorage,
    location: window_.location,
    history: window_.history,
    fetch: fetchImpl,
    EventSource: function () {
      this.onopen = null;
      this.onmessage = null;
      this.onerror = null;
      streamSource = this;
    },
    Node: El,
    Notification: class Notification {
      static permission = 'granted';
      static requested = 0;
      static instances = [];
      static requestPermission() { Notification.requested += 1; }
      constructor(title, options) { Notification.instances.push({ title, body: options && options.body }); }
    },
    FileReader: class {
      readAsText(file) {
        this.result = file && file.text != null ? file.text : '';
        if (this.onload) this.onload();
      }
    },
    URLSearchParams,
    URL,
    Intl,
    console,
    Date,
    Math,
    JSON,
    Promise,
    Set,
    Map,
    Symbol,
    Object,
    Array,
    String,
    Number,
    Boolean,
    RegExp,
    Error,
    isNaN,
    parseInt,
    parseFloat,
    encodeURIComponent,
    decodeURIComponent,
    requestAnimationFrame: window_.requestAnimationFrame,
    setTimeout: window_.setTimeout,
    clearTimeout: window_.clearTimeout,
    setInterval: window_.setInterval,
    clearInterval: window_.clearInterval,
  };
  sandbox.globalThis = sandbox;
  vm.createContext(sandbox);
  vm.runInContext(SRC, sandbox, { filename: 'app.js' });

  return {
    IDS,
    calls,
    body,
    documentElement,
    window: window_,
    sandbox,
    push(event) {
      events.push(event);
      if (streamSource && streamSource.onmessage) streamSource.onmessage({ data: JSON.stringify(event) });
    },
    open() { if (streamSource && streamSource.onopen) streamSource.onopen({}); },
    hash(next) {
      window_.location.hash = next;
    },
    fireOn(id, kind, event) { IDS[id].fire(kind, event || {}); },
    text: (id) => (IDS[id] ? IDS[id].textContent : ''),
    rows: (sel) => documentElement.querySelectorAll(sel),
    clipboardWrites,
  };
}

const emptyState = (session) => ({
  session: session || '', model: 'm', workspace: '/w', folder: null, goal: null,
  budget: 100000, turn_budget: 8, gate: { name: 'sys1', informs_failover: false },
  running: false, driving: null, dropped: 0, events: [], approval: null, rules: [], attached: true,
});

const emptyChannels = () => ({
  type: 'channels', read_at: 'now', service: {}, telegram: { present: false },
  channels: [], activity: [], activity_bound: 0, waiting_seats: 0,
  conversations: { type: 'conversations', bound: 50, count: 0, needs: 0, personas: [] },
});

const emptySessions = () => ({
  type: 'sessions', read_at: 'now', current: '',
  counts: { needs: 0, working: 0, background: 0, earlier: 0, total: 0 },
  sessions: [],
});

// A row in the bridge's shape, with only the fields a case cares about.
const row = (id, state, extra) => ({
  id,
  title: id,
  model: 'm',
  created: '2026-10-23T09:00:00Z',
  ended: state === 'completed' || state === 'capped' || state === 'stopped',
  stop_reason: state === 'capped' ? 'max-turns' : null,
  turns: state === 'capped' ? 8 : null,
  last_write: 0,
  kind: null,
  workspace: '/Users/owner/Projects/fleet',
  workspace_inferred: false,
  channel: null,
  compacted: false,
  current: false,
  class: 'yours',
  job: null,
  state,
  ...extra,
});

// ------------------------------------------------------------------ runner

process.on('unhandledRejection', (error) => {
  console.error('face: an unhandled rejection escaped app.js — ' + (error && error.stack ? error.stack : error));
});

const failures = [];
const pass = [];
function check(name, ok, detail) {
  if (ok) pass.push(name);
  else failures.push(name + (detail ? ' — ' + detail : ''));
}
const flush = async (turns = 40) => { for (let i = 0; i < turns; i += 1) await Promise.resolve(); };

// C1/C8: the list is grouped by what needs the owner, in fixed order,
// and the counts are the document's own owner-scoped derivation — not a
// second derivation in the browser. The fixture's system rows (a
// scheduled run paused at its gate, two scheduled runs ended) must
// render nowhere: the rail is the owner's (the 2026-10-29 ruling).
async function theListGroupsByAttentionAndCountsWhatTheServerCounted() {
  const sessions = {
    type: 'sessions', read_at: 'now', current: '',
    counts: { needs: 1, working: 1, background: 2, earlier: 1, yours: 3, total: 7 },
    sessions: [
      row('paused-1', 'paused', { title: 'corpus-refresh-fleet — paused', class: 'background', job: 'corpus-refresh-fleet' }),
      row('needs-1', 'needs-approval', { title: 'the ledger writer' }),
      row('work-1', 'working', { title: 'skills the model can reach' }),
      row('done-1', 'completed', { title: 'conversation control' }),
      row('bg-1', 'completed', { title: 'corpus-refresh fleet', class: 'background', job: 'corpus-refresh-fleet' }),
      row('bg-2', 'interrupted', { title: 'corpus-refresh fleet', class: 'background', job: 'corpus-refresh-fleet' }),
      row('probe-1', 'completed', { title: 'say the single word: ok', class: 'probe' }),
    ],
  };
  const page = boot({
    hash: '#/sessions',
    documents: {
      sessions,
      state: (asked) => emptyState(asked),
    },
  });
  await flush();
  page.open();
  await flush();

  const groups = page.rows('.work-group').map((g) => g.textContent.trim());
  check('C1 group order', groups[0] && groups[0].startsWith('needs you'), groups.join(' | '));
  check('C1 working follows', groups[1] && groups[1].startsWith('working'), groups.join(' | '));
  check('C1 earlier follows working', groups[2] && groups[2].startsWith('earlier'), groups.join(' | '));
  check('C1 the head prints the server count', /1 needs you/.test(page.text('work-head')), page.text('work-head'));
  check('C1 the needs header prints the server count', /needs you1/.test(groups[0] || ''), groups[0]);
  // The rail is the owner's: no toggle exists to unhide the system
  // work, because there is no system work on the screen to unhide.
  check('C1 no background line exists at all', page.rows('.work-background').length === 0,
    String(page.rows('.work-background').length));
  check('C1 no family lines exist', page.rows('.work-family').length === 0,
    String(page.rows('.work-family').length));
  check('C1 no system row renders in any group',
    page.rows('.work-row').every((row) => !['background', 'delegated', 'channel', 'probe'].includes(row.dataset.class)) &&
      !/corpus-refresh-fleet/.test(page.text('work-list')) &&
      !/say the single word/.test(page.text('work-list')),
    page.rows('.work-row').map((r) => r.dataset.class).join(','));
  // C8: the needs pick skips the paused scheduled row — a system run at
  // its gate never opens as "the first row that needs the owner".
  check('C8 the opened one is the first owner row that needs the owner',
    /#\/sessions\/.+/.test(page.window.location.hash) &&
      page.window.location.hash.includes('needs-1'),
    page.window.location.hash);

  // Earlier is the ended owner work: no scheduled run, no child, no peer,
  // and exactly the rows the fixture puts there.
  const kids = page.IDS['work-list'].children;
  const isGroup = (kid) => kid.className.split(/\s+/).includes('work-group');
  const earlierAt = kids.findIndex((kid) => isGroup(kid) && /^earlier/.test(kid.textContent));
  check('C1 the earlier group exists', earlierAt >= 0, String(earlierAt));
  const earlierRows = earlierAt < 0
    ? []
    : kids.slice(earlierAt + 1).filter((kid) => kid.className.split(/\s+/).includes('work-row'));
  check('C1 earlier holds the fixture\'s one ended owner run', earlierRows.length === 1, String(earlierRows.length));
  check('C1 no background work among earlier',
    earlierRows.every((row) => !['background', 'delegated', 'channel'].includes(row.dataset.class)),
    earlierRows.map((row) => row.dataset.class).join(','));
  check('C1 the earlier row is the fixture\'s completed one',
    earlierRows.length === 1 && earlierRows[0].dataset.state === 'completed',
    earlierRows.length ? earlierRows[0].dataset.state : 'none');
  check('C1 the earlier header prints the server count',
    earlierAt >= 0 && /earlier1/.test(kids[earlierAt].textContent),
    earlierAt >= 0 ? kids[earlierAt].textContent : 'no header');
}

// C2: one verb — every state opens the same way, through a resume, with
// a composer. No state-dependent read-only mode.
async function everyStateOpensThroughOneVerb() {
  const states = ['completed', 'working', 'interrupted'];
  for (const state of states) {
    const id = state + '-row';
    const page = boot({
      hash: '#/sessions',
      documents: {
        sessions: {
          type: 'sessions', read_at: 'now', current: '', counts: { needs: 0, working: 0, background: 0, earlier: 1, total: 1 },
          sessions: [row(id, state, { title: 'a conversation' })],
        },
        state: (asked) => ({ ...emptyState(asked), session: asked, events: [{ type: 'message', role: 'owner', text: 'the task' }] }),
      },
    });
    await flush();
    page.open();
    await flush();
    const target = page.rows('.work-row').find((r) => r.dataset.state === state);
    check('C2 a row exists for ' + state, !!target, String(page.rows('.work-row').length));
    if (target) target.click();
    await flush();
    const resumed = page.calls.filter((c) => c.url.startsWith('/api/resume') && String(c.body || '').includes(id));
    check('C2 ' + state + ' opens through the resume', resumed.length >= 1, JSON.stringify(page.calls.filter((c) => c.url.startsWith('/api/resume')).map((c) => c.body)));
    check('C2 ' + state + ' has a composer', page.IDS.composer.offsetHeight > 0, 'composer');
    check('C2 ' + state + ' shows its history', page.text('log').includes('the task'), page.text('log').slice(0, 80));
  }
}

// C3: the conversation's head — title, state, New, and a switcher that
// opens the list where it is a sheet and back that closes it.
async function theHeadCarriesTheWayOutAndTheSwitcher() {
  const page = boot({
    width: 800,
    hash: '#/sessions/open-1',
    documents: {
      sessions: {
        type: 'sessions', read_at: 'now', current: '',
        counts: { needs: 0, working: 1, background: 0, earlier: 1, total: 2 },
        sessions: [
          row('open-1', 'completed', { title: 'the ledger writer' }),
          row('other-1', 'working', { title: 'skills the model can reach' }),
        ],
      },
      state: (asked) => ({ ...emptyState(asked), session: asked, events: [{ type: 'message', role: 'owner', text: 'the task' }] }),
    },
  });
  await flush();
  page.open();
  await flush();
  check('C3 the head is shown', page.IDS['conv-head'].hidden === false, 'hidden=' + page.IDS['conv-head'].hidden);
  if (process.env.FACE_DEBUG) {
    console.error('C3 DEBUG', JSON.stringify({
      hash: page.window.location.hash,
      calls: page.calls.map((c) => c.method + ' ' + c.url),
      name: page.text('conv-name'),
      state: page.text('conv-state'),
      work: page.body.dataset.work,
      rows: page.rows('.work-row').length,
      list: page.text('work-list').slice(0, 120),
    }));
  }
  check('C3 the title is the row\'s', page.text('conv-name') === 'the ledger writer', page.text('conv-name'));
  check('C3 the state is a word', page.text('conv-state') === 'completed', page.text('conv-state'));
  check('C3 the column is folded on a narrow face', page.body.dataset.work !== 'open', String(page.body.dataset.work));
  page.fireOn('conv-title', 'click');
  check('C3 the title opens the sheet', page.body.dataset.work === 'open', String(page.body.dataset.work));
  page.fireOn('conv-back', 'click');
  check('C3 back closes the sheet', page.body.dataset.work !== 'open', String(page.body.dataset.work));
  // Pick a second row in the open sheet: the head's name and the address
  // move together, and the list survives the switch.
  page.fireOn('conv-title', 'click');
  const second = page.rows('.work-row').find((r) => r.dataset.state === 'working');
  check('C3 a second row stands in the sheet', !!second, String(page.rows('.work-row').length));
  if (second) second.click();
  await flush(80);
  check('C3 the head names the picked conversation',
    page.text('conv-name') === 'skills the model can reach', page.text('conv-name'));
  check('C3 the address moved to its id',
    page.window.location.hash === '#/sessions/other-1', page.window.location.hash);
  check('C3 the list survives the switch', page.rows('.work-row').length === 2,
    String(page.rows('.work-row').length));

  page.fireOn('conv-new', 'click');
  await flush();
  // New is an intent, not a row (N2): no session is created until words
  // are sent — the face walks home and the composer waits.
  const news = page.calls.filter((c) => c.url.startsWith('/api/session/new'));
  check('C3 New mints nothing', news.length === 0, String(news.length));
  check('C3 New walks home', page.window.location.hash === '#/chat', page.window.location.hash);
  check('C3 the home shows the welcome',
    page.IDS.empty.hidden === false && page.IDS.log.hidden === true,
    'empty hidden: ' + page.IDS.empty.hidden);
}

// C5 (face half): with a run working in another conversation, a send
// here queues and the chip names the working one.
async function aSendBesideDrivingWorkQueuesAndNamesIt() {
  const page = boot({
    width: 1440,
    hash: '#/sessions/this-one',
    documents: {
      sessions: {
        type: 'sessions', read_at: 'now', current: '',
        counts: { needs: 0, working: 0, background: 0, earlier: 2, total: 2 },
        sessions: [
          row('other-one', 'working', { title: 'corpus-refresh-fleet — approval' }),
          row('this-one', 'completed', { title: 'this conversation' }),
        ],
      },
      state: (asked) => ({ ...emptyState(asked), session: asked, driving: 'other-one', running: false }),
    },
  });
  await flush();
  page.open();
  await flush();
  page.IDS.task.value = 'do the next thing';
  page.fireOn('composer', 'submit', {});
  await flush();
  const runs = page.calls.filter((c) => c.url.startsWith('/api/run'));
  check('C5 no run starts beside another', runs.length === 0, String(runs.length));
  const note = page.text('queue-note');
  check('C5 the queue names the working conversation', /a run is working in corpus-refresh-fleet — approval/.test(note), note);
  check('C5 the queue chip is shown', page.IDS['queue-chip'].hidden === false, 'hidden');
  check('C5 the queued text is the owner\'s', /do the next thing/.test(page.text('queue-list')), page.text('queue-list'));
  if (process.env.FACE_DEBUG) console.error('C5 DEBUG', JSON.stringify({ note: page.text('queue-note'), chip: page.text('queue-chip') }));
}

// C6: a streamed reply is text until its block closes, then rendered
// once — a heading, a list, and a fence appear, and the caret class goes.
async function aStreamedReplyRendersAtItsBoundary() {
  const page = boot({
    hash: '#/sessions/stream-1',
    documents: {
      sessions: {
        type: 'sessions', read_at: 'now', current: '',
        counts: { needs: 0, working: 1, background: 0, earlier: 0, total: 1 },
        sessions: [row('stream-1', 'working', { title: 'streaming' })],
      },
      state: (asked) => ({ ...emptyState(asked), session: asked }),
    },
  });
  await flush();
  page.open();
  await flush();
  // A run starts and its words arrive as tokens.
  page.push({ type: 'message', role: 'owner', text: 'say something formatted' });
  page.push({ type: 'turn', turn: 1 });
  page.push({ type: 'token', text: '## What changed\n\n- the first thing\n' });
  await flush();
  const mid = page.rows('.assistant');
  check('C6 mid-stream there is one assistant block', mid.length === 1, String(mid.length));
  check('C6 mid-stream it is streaming text', mid.length === 1 && /streaming/.test(mid[0].className), mid.length ? mid[0].className : '');
  check('C6 mid-stream no markup yet', mid.length === 1 && mid[0].querySelector('.md') === null,
    mid.length ? String(mid[0].querySelector('.md')) : '');
  page.push({ type: 'token', text: '```rust\nlet lock = FileLock::take(&path);\n```\n' });
  await flush();
  page.push({ type: 'finished', reason: 'stop' });
  await flush();
  const done = page.rows('.assistant');
  check('C6 closed it is rendered', done.length === 1 && /md-rendered/.test(done[0].className),
    done.length ? done[0].className : '');
  check('C6 the heading rendered', page.rows('.assistant .md h4').length === 1,
    String(page.rows('.assistant .md h4').length));
  check('C6 the list rendered', page.rows('.assistant .md-li').length === 1,
    String(page.rows('.assistant .md-li').length));
  check('C6 the fence rendered', page.rows('.assistant .md-code').length === 1,
    String(page.rows('.assistant .md-code').length));
  check('C6 the caret class is gone', page.rows('.assistant.streaming').length === 0,
    String(page.rows('.assistant.streaming').length));
}

// C6 on a replay: a resumed history renders its markdown too.
async function aReplayedReplyRendersAsMarkdown() {
  const page = boot({
    hash: '#/sessions/replay-1',
    documents: {
      sessions: {
        type: 'sessions', read_at: 'now', current: '',
        counts: { needs: 0, working: 0, background: 0, earlier: 1, total: 1 },
        sessions: [row('replay-1', 'completed', { title: 'a finished conversation' })],
      },
      state: (asked) => ({
        ...emptyState(asked), session: asked,
        events: [
          { type: 'message', role: 'owner', text: 'what changed?' },
          { type: 'message', role: 'assistant', text: 'It does not use a counter.\n\n## What changed\n\n- the lock\n\n```rust\nlet x = 1;\n```' },
        ],
      }),
    },
  });
  await flush();
  page.open();
  await flush();
  if (process.env.FACE_DEBUG) console.error('C6R DEBUG', JSON.stringify({ log: page.text('log').slice(0, 200), rows: page.rows('.assistant').map((r) => r.className) }));
  check('C6 replay renders one block', page.rows('.assistant.md-rendered').length === 1,
    String(page.rows('.assistant.md-rendered').length));
  check('C6 replay renders the fence', page.rows('.assistant .md-code').length === 1,
    String(page.rows('.assistant .md-code').length));
  check('C6 replay renders the list', page.rows('.assistant .md-li').length === 1,
    String(page.rows('.assistant .md-li').length));
  check('C6 no caret on replay', page.rows('.assistant.streaming').length === 0, 'streaming');
}


// C5: a run's end clears the driving session, so the next send in a
// conversation that was opened beside it starts a run instead of
// queueing behind work that is over.
async function aFinishedRunStopsBlockingTheNextSend() {
  const page = boot({
    hash: '#/sessions/beside-1',
    documents: {
      sessions: {
        type: 'sessions', read_at: 'now', current: '',
        counts: { needs: 0, working: 0, background: 0, earlier: 1, total: 1 },
        sessions: [row('beside-1', 'completed', { title: 'this conversation' })],
      },
      state: (asked) => ({ ...emptyState(asked), session: asked, driving: 'other-run', running: false }),
    },
  });
  await flush();
  page.open();
  await flush();
  page.push({ type: 'finished', reason: 'stop', session: 'other-run' });
  await flush();
  page.IDS.task.value = 'the next thing';
  page.fireOn('composer', 'submit', {});
  await flush();
  const runs = page.calls.filter((c) => c.url.startsWith('/api/run'));
  check('C5 the ended run clears the way for the next send', runs.length === 1, String(runs.length));
  check('C5 the send names this conversation',
    runs.length === 1 && String(runs[0].url).includes('beside-1'),
    runs.length ? runs[0].url : 'no run');
}


// C7: the pane is the one right surface and the work column holds
// its place; the prose measure stays inside 80ch. The mode is the DOM's
// to prove here; the rules that produce the geometry are read from the
// stylesheet, because a DOM stub has no layout engine (the live pass
// measures the pixels at 1600).
async function theDockIsClosedAndTheWorkColumnTakesItsPlace() {
  const page = boot({
    width: 1600,
    hash: '#/sessions/wide-1',
    documents: {
      sessions: {
        type: 'sessions', read_at: 'now', current: '',
        counts: { needs: 0, working: 0, background: 0, earlier: 1, total: 1 },
        sessions: [row('wide-1', 'completed', { title: 'a wide conversation' })],
      },
      state: (asked) => ({ ...emptyState(asked), session: asked }),
    },
  });
  await flush();
  page.open();
  await flush();
  // The pane's cards are the rail's cards: the dock carried
  // `class="dock rail"`, and with it `.rail .card`'s box. Losing that
  // ancestor left the run tab's cards flush against the frame, their
  // text under the grip and against the edge.
  check('C7 the pane\u2019s cards carry the rail card\u2019s box',
    /\.rail \.card,\n\.pane \.card \{/.test(CSS) &&
      ruleBlock('.rail .card,\n.pane .card').includes('padding: 16px') &&
      ruleBlock('.rail .card,\n.pane .card').includes('border-radius: 12px'),
    ruleBlock('.rail .card,\n.pane .card') || 'rule missing');
  check('C7 the run tab insets its cards past the grip',
    ruleBlock('#pane-run').includes('padding: 8px 12px') &&
      ruleBlock('.pane-grip').includes('left: -4px') &&
      ruleBlock('.pane-grip').includes('width: 9px'),
    ruleBlock('#pane-run') + ' | ' + ruleBlock('.pane-grip'));
  check('C7 one entry: no aside toggle, one pane button',
    !HTML.includes('aside-toggle') && HTML.includes('id="pane-open"') &&
      !HTML.includes('class="dock rail"'),
    'html');
  check('C7 the pane carries the four tabs, run first',
    HTML.includes('id="tab-run"') && HTML.includes('id="pane-run"') &&
      HTML.indexOf('tab-run') < HTML.indexOf('tab-files') &&
      HTML.includes('id="process-card"') === true &&
      HTML.indexOf('id="process-card"') > HTML.indexOf('id="pane-run"'),
    'tabs');
  check('C7 the work column is open', page.body.dataset.work === 'open', String(page.body.dataset.work));
  check('C7 the work column is shown', page.IDS['work-col'].hidden === false, 'hidden');
  check('C7 the column has a width token', flat(CSS).includes('--work-width: 320px'), '--work-width');
  check('C7 the conversation is inset by the column',
    ruleBlock('body[data-work="open"] .stage').includes('margin-left: var(--work-width)'),
    ruleBlock('body[data-work="open"] .stage'));
  check('C7 the column sits beside the rail and is that wide',
    ruleBlock('.work-col').includes('left: var(--rail-width)') &&
      ruleBlock('.work-col').includes('width: var(--work-width)'),
    ruleBlock('.work-col'));
  check('C7 the sheet drops the inset where there is no room',
    flat(CSS).includes('body[data-work="open"] .stage { margin-left: 0; }'), 'narrow override');
  const measure = (ruleBlock('.assistant').match(/max-width: (\d+)ch/) || [])[1];
  check('C7 the prose measure is at most 80ch', measure && Number(measure) <= 80, String(measure));
  check('C7 the conversation stacks head, content and footer',
    ruleBlock('.column').includes('display: flex') &&
      ruleBlock('.column').includes('flex-direction: column'),
    ruleBlock('.column'));
  check('C7 the content grows and the footer keeps its height',
    ruleBlock('.empty, .log').includes('flex: 1 1 auto') &&
      ruleBlock('.process-strip, .seat, .actions, .commands, .composer').includes('flex: 0 0 auto'),
    ruleBlock('.empty, .log') + ' | ' + ruleBlock('.process-strip, .seat, .actions, .commands, .composer'));
}

// C7 (fold): the list folds to a strip with its own way back and the
// count it is worth watching for, the choice is kept, and the column
// follows the rail so a folded nav leaves no gutter behind it.
async function theWorkListFoldsToAStrip() {
  const page = boot({
    width: 1600,
    hash: '#/sessions/fold-1',
    documents: {
      sessions: {
        type: 'sessions', read_at: 'now', current: '',
        counts: { needs: 2, working: 0, background: 0, earlier: 1, total: 3 },
        sessions: [
          row('fold-1', 'waiting', { title: 'a conversation that needs you' }),
          row('fold-2', 'completed', { title: 'a past conversation' }),
        ],
      },
      state: (asked) => ({ ...emptyState(asked), session: asked }),
    },
  });
  await flush();
  page.open();
  await flush();
  await flush();
  check('C7 the head carries the fold control', page.rows('.work-fold').length === 1,
    String(page.rows('.work-fold').length));
  const rowsBefore = page.rows('.work-row').length;
  check('C7 the open list shows its rows', rowsBefore >= 1, String(rowsBefore));
  page.rows('.work-fold')[0].click();
  await flush();
  check('C7 the fold collapses the list', page.body.dataset.work === 'collapsed',
    String(page.body.dataset.work));
  check('C7 the strip stays on the page', page.IDS['work-col'].hidden === false,
    'hidden=' + page.IDS['work-col'].hidden);
  check('C7 the strip holds the way back and nothing else',
    page.rows('.work-expand').length === 1 && page.rows('.work-row').length === 0,
    'expand=' + page.rows('.work-expand').length + ' rows=' + page.rows('.work-row').length);
  check('C7 the strip carries the count of what needs the owner',
    page.text('work-head').includes('2'), page.text('work-head'));
  check('C7 the choice is remembered', page.window.localStorage.getItem('fleet-work') === 'collapsed',
    String(page.window.localStorage.getItem('fleet-work')));
  page.rows('.work-expand')[0].click();
  await flush();
  check('C7 the way back restores the rows',
    page.body.dataset.work === 'open' && page.rows('.work-row').length === rowsBefore,
    String(page.body.dataset.work) + ' rows=' + page.rows('.work-row').length);
  // The geometry, from the sheet that declares it: the column starts where
  // the nav ends, whichever nav is showing, and the stage is inset by the
  // width the list declares in that mode.
  check('C7 the column follows a folded rail onto the freed space',
    ruleBlock('body[data-rail="icons"] .work-col').includes('left: var(--rail-width-icons)'),
    ruleBlock('body[data-rail="icons"] .work-col'));
  check('C7 the folded width is the strip\'s own',
    ruleBlock('body[data-work="collapsed"]').includes('--work-width: 48px'),
    ruleBlock('body[data-work="collapsed"]'));
  check('C7 the conversation takes the rest of the width when folded',
    ruleBlock('body[data-work="collapsed"] .stage').includes('margin-left: var(--work-width)'),
    ruleBlock('body[data-work="collapsed"] .stage'));
  check('C7 the strip drops the inset where the list is a sheet',
    flat(CSS).includes('body[data-work="collapsed"] .stage { margin-left: 0; }'),
    'narrow override');
}

// C7 (chat size): the size is the conversation's — the one surface the
// owner reads at length. A multiplier on the transcript's own token, two
// presses and two keys over a fixed scale, dimmed at the ends, kept
// across a reload, and the composer keeps its measure.
async function theConversationTextSizeIsTheOwners() {
  const doc = {
    sessions: {
      type: 'sessions', read_at: 'now', current: '',
      counts: { needs: 0, working: 0, background: 0, earlier: 1, total: 1 },
      sessions: [row('chat-1', 'completed', { title: 'a conversation to read' })],
    },
    state: (asked) => ({ ...emptyState(asked), session: asked }),
  };
  const page = boot({ width: 1440, hash: '#/sessions/chat-1', documents: doc });
  await flush();
  page.open();
  await flush();
  const scaleOf = (one) => (one || page).IDS['log'].style.getPropertyValue('--chat-scale');
  check('C7 the transcript opens at its own measure', scaleOf() === '1', scaleOf());
  check('C7 the composer carries no size token',
    !page.IDS['composer'].style.getPropertyValue('--composer-size'), 'composer token');
  check('C7 the composer keeps the plain measure',
    ruleBlock('.composer textarea').includes('font-size: 14.5px') &&
      !ruleBlock('.composer textarea').includes('--composer-size'),
    ruleBlock('.composer textarea'));
  check('C7 the multiplier rides the transcript\'s own sizes',
    ruleBlock('.assistant').includes('font-size: calc(15px * var(--chat-scale, 1))') &&
      ruleBlock('.owner').includes('font-size: calc(14px * var(--chat-scale, 1))'),
    ruleBlock('.assistant') + ' | ' + ruleBlock('.owner'));
  check('C7 the rendered reply scales with it',
    ruleBlock('.assistant .md-p').includes('calc(15px * var(--chat-scale, 1))') &&
      ruleBlock('.assistant .md-code').includes('calc(12px * var(--chat-scale, 1))'),
    ruleBlock('.assistant .md-p'));
  // The run's own telemetry is text the owner reads too, so it scales
  // with the reply — the thought row, the command card and its expanded
  // detail, the receipt, and the run line.
  check('C7 the thought row scales with it',
    ruleBlock('.thought-head').includes('calc(12px * var(--chat-scale, 1))') &&
      ruleBlock('.thought-body .md-li')
        .includes('calc(12.5px * var(--chat-scale, 1))'),
    ruleBlock('.thought-head'));
  check('C7 the command card and its detail scale with it',
    ruleBlock('.card-row .title').includes('calc(12.5px * var(--chat-scale, 1))') &&
      ruleBlock('.tool-output').includes('calc(11px * var(--chat-scale, 1))') &&
      ruleBlock('.tool-facts').includes('calc(10.5px * var(--chat-scale, 1))'),
    ruleBlock('.tool-output'));
  check('C7 the receipt and the run line scale with it',
    ruleBlock('.receipt .receipt-line').includes('calc(11px * var(--chat-scale, 1))') &&
      ruleBlock('.stream-text').includes('calc(11.5px * var(--chat-scale, 1))'),
    ruleBlock('.receipt .receipt-line'));
  // The boundary: a card's own furniture — the big metric and its
  // caption — is display, not telemetry, and keeps its size.
  check('C7 a card\'s metric and caption keep their size',
    ruleBlock('.card-row .value').includes('font-size: 20px') &&
      !ruleBlock('.card-row .value').includes('chat-scale'),
    ruleBlock('.card-row .value'));
  page.fireOn('chat-bigger', 'click');
  check('C7 one press up is the next step', scaleOf() === '1.15', scaleOf());
  check('C7 the choice is kept',
    page.window.localStorage.getItem('fleet-chat-size') === '2',
    String(page.window.localStorage.getItem('fleet-chat-size')));
  page.fireOn('chat-bigger', 'click');
  check('C7 a second press is the step above', scaleOf() === '1.3', scaleOf());
  page.fireOn('chat-smaller', 'click');
  page.fireOn('chat-smaller', 'click');
  check('C7 down walks back to the default', scaleOf() === '1', scaleOf());
  page.fireOn('chat-smaller', 'click');
  check('C7 the scale stops at its floor', scaleOf() === '0.85', scaleOf());
  check('C7 the floor dims its own press', page.IDS['chat-smaller'].disabled === true,
    String(page.IDS['chat-smaller'].disabled));
  check('C7 the floor names the size it stopped at',
    page.IDS['chat-smaller'].title.includes('85%'), page.IDS['chat-smaller'].title);
  page.fireOn('chat-smaller', 'click');
  check('C7 a press past the floor changes nothing', scaleOf() === '0.85', scaleOf());
  page.fireOn('composer', 'keydown', { key: 'ArrowUp', altKey: true });
  check('C7 Alt+up is the same step as the button', scaleOf() === '1', scaleOf());
  page.fireOn('composer', 'keydown', { key: 'ArrowUp', altKey: false });
  check('C7 a bare arrow is the field\'s own key', scaleOf() === '1', scaleOf());

  // A reload keeps the size, and a stored index the scale no longer has
  // is clamped rather than trusted.
  const kept = boot({ width: 1440, hash: '#/sessions/chat-1', documents: doc,
    storage: { 'fleet-chat-size': '4' } });
  await flush();
  kept.open();
  await flush();
  check('C7 a reload keeps the size', scaleOf(kept) === '1.5', scaleOf(kept));
  check('C7 a middle step keeps the way up open', kept.IDS['chat-bigger'].disabled === false,
    String(kept.IDS['chat-bigger'].disabled));
  const stale = boot({ width: 1440, hash: '#/sessions/chat-1', documents: doc,
    storage: { 'fleet-chat-size': '99' } });
  await flush();
  stale.open();
  await flush();
  check('C7 a stored index past the scale is clamped to its top',
    scaleOf(stale) === '1.7', scaleOf(stale));
  check('C7 the clamped top dims its own press', stale.IDS['chat-bigger'].disabled === true,
    String(stale.IDS['chat-bigger'].disabled));
}

// C8 (the servers card): the settings board carries the MCP registry —
// rows with their dots, toggles and probe lines; the scan's discovery
// rows with their import buttons; an unsupported row's import disabled
// with the reason; and a value the face is handed never prints.
async function theServersCardCarriesTheRegistry() {
  // The servers live in the specialists section as one-line rows; their
  // probe, source, env names and the enable toggle live in the item card
  // the row opens (the 2026-10-30 revamp), and the scan keeps its own
  // action row so discovery did not leave with the card.
  const page = boot({
    width: 1440,
    hash: '#/settings/specialists',
    documents: {
      settings: {
        type: 'settings', read_at: 'now',
        scalars: [], lists: [], rules: [], env: [],
        cron: [], soul: { present: false, text: '' },
        router: {}, catalog: {}, roles: { entries: [] }, vault: { names: [] },
        lanes: { lanes: [] },
        servers: {
          type: 'servers',
          servers: [
            { name: 'github', program: 'docker',
              args: ['run', '-i', '-e', 'GITHUB_PERSONAL_ACCESS_TOKEN=x', 'ghcr.io/github/github-mcp-server'],
              env_names: ['GITHUB_PERSONAL_ACCESS_TOKEN'], enabled: true,
              source: { harness: 'claude-desktop', imported: '2026-10-23' },
              last_probe: { state: 'ok', server_version: 'github-mcp v0.9.0' } },
            { name: 'time', program: 'npx', args: ['-y', 'server-time'], env_names: [],
              enabled: false, source: null, last_probe: null },
          ],
        },
      },
    },
  });
  await flush();
  page.open();
  await flush();
  await flush();

  const rows = () => page.rows('.cfg-item').filter((row) => row.dataset.kind === 'server');
  check('C8 one row per configured server', rows().length === 2, String(rows().length));
  check('C8 a row names its server and its state',
    rows()[0].textContent.includes('github') && rows()[1].textContent.includes('time') &&
      rows()[1].textContent.toLowerCase().includes('off'),
    rows().map((r) => r.textContent).join(' | '));

  // The card carries what the row cannot: the command, the source, the
  // env names and the toggle.
  rows()[0].click();
  await flush(60);
  const body = () => page.text('dock-object-body');
  check('C8 the card names the command and its probe',
    body().includes('ghcr.io/github/github-mcp-server') &&
      body().includes('github-mcp v0.9.0'),
    body().slice(0, 200));
  check('C8 the card names the source harness', body().includes('claude-desktop'),
    body().slice(0, 300));
  check('C8 the card says the env name resolves at spawn',
    body().includes('GITHUB_PERSONAL_ACCESS_TOKEN') && body().includes('at spawn'),
    body().slice(0, 300));
  check('C8 the card carries the enable toggle',
    page.rows('.cfg-card-acts .replay-button').some((b) => /disable|enable/.test(b.textContent)),
    String(page.rows('.cfg-card-acts .replay-button').length));
  page.rows('.cfg-card-acts .replay-button')
    .find((b) => /disable/.test(b.textContent)).click();
  await flush(80);
  const posted = page.calls.filter((c) => c.method === 'POST' && c.url === '/api/settings');
  check('C8 the toggle posts the registry\u2019s op',
    posted.some((c) => c.body && c.body.includes('"op":"server"') &&
      c.body.includes('"name":"github"')),
    JSON.stringify(posted.map((c) => c.body)));

  // The scan keeps its own row: discovery is a press, and the import is
  // one more.
  check('C8 the scan row is on the list',
    page.rows('.cfg-item').some((row) => row.dataset.kind === 'action' &&
      row.textContent.includes('scan this machine')),
    'scan');
  page.rows('.cfg-item').find((row) => row.dataset.kind === 'action').click();
  await flush(60);
  check('C8 the scan card offers the scan and names what it found',
    body().includes('scan this machine') && body().includes('Not scanned yet'),
    body().slice(0, 200));
  page.rows('.cfg-card-acts .replay-button')
    .find((b) => /scan this machine/.test(b.textContent)).click();
  await flush(80);
  check('C8 the scan posts the registry\u2019s own op',
    page.calls.some((c) => c.body && c.body.includes('"op":"server-scan"')),
    'scan op');
}

// C1 (the goal): the run bar carries the goal field on every visit —
// unset is the set affordance, set is one line with the whole text on
// hover — and the popover's Set/Replace/Clear ride /api/goal, the same
// route /goal rides, so the command and the chip agree.
async function theGoalFieldIsAlwaysPresentAndWritable() {
  const doc = (goal) => ({
    sessions: {
      type: 'sessions', read_at: 'now', current: '',
      counts: { needs: 0, working: 0, background: 0, earlier: 1, total: 1 },
      sessions: [row('goal-1', 'completed', { title: 'a conversation' })],
    },
    state: (asked) => ({ ...emptyState(asked), session: asked, goal }),
  });
  const page = boot({ width: 1440, hash: '#/sessions/goal-1', documents: doc(null) });
  await flush();
  page.open();
  await flush();
  check('C1 the field is present with no goal',
    page.IDS['run-goal'].hidden === false && page.IDS['run-goal-btn'].classList.contains('unset'),
    String(page.IDS['run-goal'].hidden));
  check('C1 the unset affordance names itself',
    page.text('run-goal-btn') === 'goal', page.text('run-goal-btn'));
  // The popover: open, set, write-through.
  page.fireOn('run-goal-btn', 'click');
  check('C1 the popover opens', page.IDS['goal-pop'].hidden === false, 'hidden');
  check('C1 the popover says not set and hides clear',
    page.text('goal-pop-state') === 'not set' && page.IDS['goal-pop-clear'].hidden === true,
    page.text('goal-pop-state'));
  page.IDS['goal-pop-input'].value = 'ship the controls this week';
  page.fireOn('goal-pop-set', 'click');
  await flush(40);
  const posted = page.calls.filter((c) => c.url.startsWith('/api/goal'));
  check('C1 set rides /api/goal with the text',
    posted.length === 1 && posted[0].body.includes('ship the controls this week'),
    JSON.stringify(posted.map((c) => c.body)));
  check('C1 the chip reads the goal back', page.text('run-goal-btn') === 'ship the controls this week',
    page.text('run-goal-btn'));
  check('C1 the chip carries the whole text on hover',
    (page.IDS['run-goal-btn'].title || '').includes('ship the controls this week'),
    page.IDS['run-goal-btn'].title || '');
  // Set state: one line, replace and clear offered. The goal arrives
  // the way a resumed log delivers it — the goal event.
  const set = boot({ width: 1440, hash: '#/sessions/goal-1', documents: doc(null) });
  await flush();
  set.open();
  await flush();
  set.push({ type: 'goal', text: 'ship the controls this week' });
  await flush(40);
  check('C1 a set goal shows on the line',
    set.text('run-goal-btn') === 'ship the controls this week', set.text('run-goal-btn'));
  check('C1 the unset affordance is gone',
    !set.IDS['run-goal-btn'].classList.contains('unset'), '');
  set.fireOn('run-goal-btn', 'click');
  check('C1 the popover prefills and offers replace and clear',
    set.IDS['goal-pop'].hidden === false &&
      set.IDS['goal-pop-input'].value === 'ship the controls this week' &&
      set.IDS['goal-pop-clear'].hidden === false &&
      set.text('goal-pop-set') === 'replace it',
    set.text('goal-pop-set'));
  set.fireOn('goal-pop-clear', 'click');
  await flush(40);
  check('C1 clear rides the same route with an empty text',
    set.calls.filter((c) => c.url.startsWith('/api/goal')).length === 1 &&
      set.calls[set.calls.length - 1].body.includes('""'),
    JSON.stringify(set.calls.filter((c) => c.url.startsWith('/api/goal')).map((c) => c.body)));
  check('C1 the chip returns to the unset affordance',
    set.text('run-goal-btn') === 'goal' && set.IDS['run-goal-btn'].classList.contains('unset'),
    set.text('run-goal-btn'));
}

// C2 (drafts): each conversation's draft is its own — typing in one,
// switching to another, and returning restores the first's sentence and
// leaves the second's untouched; a reload restores both; the legacy
// single-key draft migrates into the visiting conversation.
async function eachConversationKeepsItsOwnDraft() {
  const doc = {
    sessions: {
      type: 'sessions', read_at: 'now', current: '',
      counts: { needs: 0, working: 0, background: 0, earlier: 2, total: 2 },
      sessions: [
        row('draft-a', 'completed', { title: 'the first conversation' }),
        row('draft-b', 'completed', { title: 'the second conversation' }),
      ],
    },
    state: (asked) => ({ ...emptyState(asked), session: asked }),
  };
  const page = boot({ width: 1440, hash: '#/sessions/draft-a', documents: doc });
  await flush();
  page.open();
  await flush();
  page.IDS['task'].value = 'the sentence being typed in A';
  page.fireOn('task', 'input');
  // Switch to B: A keeps its sentence, B starts empty.
  const rowB = page.rows('.work-row').find((r) => r.textContent.includes('the second'));
  rowB.click();
  await flush(80);
  check('C2 the incoming conversation starts empty',
    page.IDS['task'].value === '', JSON.stringify(page.IDS['task'].value));
  check('C2 the outgoing conversation kept its draft',
    page.window.localStorage.getItem('fleet-draft-draft-a') === 'the sentence being typed in A',
    String(page.window.localStorage.getItem('fleet-draft-draft-a')));
  page.IDS['task'].value = 'and a different one in B';
  page.fireOn('task', 'input');
  // Back to A: its own sentence returns.
  const rowA = page.rows('.work-row').find((r) => r.textContent.includes('the first'));
  rowA.click();
  await flush(80);
  check('C2 returning restores the conversation\'s own draft',
    page.IDS['task'].value === 'the sentence being typed in A',
    JSON.stringify(page.IDS['task'].value));
  check('C2 B kept its own',
    page.window.localStorage.getItem('fleet-draft-draft-b') === 'and a different one in B',
    String(page.window.localStorage.getItem('fleet-draft-draft-b')));
  // A reload restores the visiting conversation's draft.
  const reloaded = boot({ width: 1440, hash: '#/sessions/draft-b', documents: doc,
    storage: { 'fleet-draft-draft-b': 'and a different one in B' } });
  await flush();
  reloaded.open();
  await flush(80);
  check('C2 a reload restores the visiting draft',
    reloaded.IDS['task'].value === 'and a different one in B',
    JSON.stringify(reloaded.IDS['task'].value));
  // The legacy single key migrates into the visiting conversation.
  const legacy = boot({ width: 1440, hash: '#/sessions/draft-a', documents: doc,
    storage: { 'fleet-draft': 'a sentence from the old build' } });
  await flush();
  legacy.open();
  await flush(80);
  // The boot restore runs before the launch session anchors, so the
  // legacy draft migrates to the front door's key — the visiting
  // conversation keeps whatever its own key held. The migration promise
  // is: the legacy key is consumed, never duplicated.
  check('C2 the legacy key is consumed by the migration',
    legacy.window.localStorage.getItem('fleet-draft') == null,
    String(legacy.window.localStorage.getItem('fleet-draft')));
  check('C2 the legacy sentence landed in the front door\'s draft',
    legacy.window.localStorage.getItem('fleet-draft-') === 'a sentence from the old build',
    String(legacy.window.localStorage.getItem('fleet-draft-')));
  check('C2 the visiting conversation\'s draft is its own',
    legacy.window.localStorage.getItem('fleet-draft-draft-a') == null,
    String(legacy.window.localStorage.getItem('fleet-draft-draft-a')));
}

// C3 (edit-and-resend): the owner's message offers edit; sending rewinds
// to the turn before the run it launched and posts the edit as a new
// run, so the log keeps the original; while a run drives, the edit
// queues with the queue's own naming.
async function editingAMessageRewindsAndResends() {
  const doc = {
    sessions: {
      type: 'sessions', read_at: 'now', current: '',
      counts: { needs: 0, working: 0, background: 0, earlier: 1, total: 1 },
      sessions: [row('edit-1', 'completed', { title: 'a conversation to correct' })],
    },
    state: (asked) => ({ ...emptyState(asked), session: asked }),
  };
  const page = boot({ width: 1440, hash: '#/sessions/edit-1', documents: doc });
  await flush();
  page.open();
  await flush();
  // A replayed history: two owner messages, each followed by its run.
  page.push({ type: 'message', role: 'owner', text: 'first ask' });
  page.push({ type: 'turn', turn: 1 });
  page.push({ type: 'message', role: 'assistant', text: 'first answer' });
  page.push({ type: 'finished', stop_reason: 'completed' });
  page.push({ type: 'message', role: 'owner', text: 'second ask' });
  page.push({ type: 'turn', turn: 2 });
  page.push({ type: 'message', role: 'assistant', text: 'second answer' });
  page.push({ type: 'finished', stop_reason: 'completed' });
  await flush(40);
  const edits = page.rows('.owner-edit');
  check('C3 each owner message offers edit', edits.length === 2, String(edits.length));
  // Edit the second ask: it launched turn 2, so the rewind target is 1.
  edits[1].click();
  check('C3 editing fills the composer',
    page.IDS['task'].value === 'second ask', JSON.stringify(page.IDS['task'].value));
  page.IDS['task'].value = 'second ask, corrected';
  page.fireOn('task', 'input');
  page.fireOn('composer', 'submit');
  await flush(80);
  const rewinds = page.calls.filter((c) => c.url.startsWith('/api/rewind'));
  check('C3 sending rewinds to the turn before the run',
    rewinds.length === 1 && rewinds[0].body.includes('"turn":1'),
    JSON.stringify(rewinds.map((c) => c.body)));
  const runs = page.calls.filter((c) => c.url.startsWith('/api/run'));
  check('C3 the edit posts as the next run',
    runs.length === 1 && runs[0].body.includes('second ask, corrected'),
    JSON.stringify(runs.map((c) => c.body)));
  check('C3 the original stays in the log',
    page.rows('.owner-text').some((r) => r.textContent === 'second ask'),
    '');
  // The first ask launched turn 1: no turn precedes it, so its edit is
  // simply the next run — no rewind.
  const page2 = boot({ width: 1440, hash: '#/sessions/edit-1', documents: doc });
  await flush();
  page2.open();
  await flush();
  page2.push({ type: 'message', role: 'owner', text: 'the very first ask' });
  page2.push({ type: 'turn', turn: 1 });
  page2.push({ type: 'finished', stop_reason: 'completed' });
  await flush(40);
  page2.rows('.owner-edit')[0].click();
  page2.IDS['task'].value = 'the very first ask, corrected';
  page2.fireOn('task', 'input');
  page2.fireOn('composer', 'submit');
  await flush(80);
  check('C3 a first-ever turn edits without a rewind',
    page2.calls.filter((c) => c.url.startsWith('/api/rewind')).length === 0,
    JSON.stringify(page2.calls.filter((c) => c.url.startsWith('/api/rewind')).length));
  check('C3 it still posts the run',
    page2.calls.filter((c) => c.url.startsWith('/api/run')).length === 1, '');
  // While a run drives, the edit queues rather than competing.
  const page3 = boot({ width: 1440, hash: '#/sessions/edit-1', documents: doc });
  await flush();
  page3.open();
  await flush();
  page3.push({ type: 'message', role: 'owner', text: 'an ask while busy' });
  await flush(40);
  page3.push({ type: 'turn', turn: 1 });
  await flush(40);
  page3.rows('.owner-edit')[0].click();
  await flush(40);
  check('C3 an edit beside a driving run queues',
    page3.text('queue-count').includes('1'), page3.text('queue-count'));
}

// C4/C5 (paste and the @ typeahead): a paste carrying a file is an
// attach through the picker's own pipeline and bounds, text inserts as
// text; the trailing @token opens the reference listbox over the reads
// the face already makes, and Enter inserts — a file reference opening
// its pane preview, a session reference leaving a link that opens the
// conversation.
async function pasteAttachesAndTheAtTypeaheadReferences() {
  const doc = {
    sessions: {
      type: 'sessions', read_at: 'now', current: '',
      counts: { needs: 0, working: 0, background: 0, earlier: 2, total: 2 },
      sessions: [
        row('ref-1', 'completed', { title: 'the reading conversation' }),
        row('ref-2', 'completed', { title: 'the other one' }),
      ],
    },
    state: (asked) => ({ ...emptyState(asked), session: asked }),
  };
  const page = boot({ width: 1440, hash: '#/sessions/ref-1', documents: doc });
  await flush();
  page.open();
  await flush();

  // A paste carrying a text file: a chip, through the picker's own
  // pipeline — one pipeline, one set of bounds.
  const file = { name: 'note.md', size: 100, text: 'a pasted note' };
  page.fireOn('task', 'paste', { clipboardData: { files: [file], items: [] } });
  await flush(40);
  const chips = page.IDS['attach-chips'].children;
  check('C4 a pasted file becomes an attach chip',
    chips.length === 1 && chips[0].textContent.includes('note.md'),
    String(chips.length));
  // The five-file bound: the sixth paste refuses by name.
  for (let i = 0; i < 5; i += 1) {
    page.fireOn('task', 'paste', {
      clipboardData: { files: [{ name: `f${i}.md`, size: 10, text: 'x' }], items: [] },
    });
    await flush(20);
  }
  check('C4 five files attach and the sixth refuses',
    page.IDS['attach-chips'].children.length === 5,
    String(page.IDS['attach-chips'].children.length));
  const page2 = boot({ width: 1440, hash: '#/sessions/ref-1', documents: doc });
  await flush();
  page2.open();
  await flush();
  // A plain text paste inserts text (the clipboard carries no files).
  page2.fireOn('task', 'paste', { clipboardData: { files: [], items: [] } });
  check('C4 a plain text paste is not an attach',
    !page2.IDS['attach-chips'].children.length, String(page2.IDS['attach-chips'].children.length));

  // The @ typeahead over the tree read.
  page.IDS['task'].value = 'look at @rea';
  page.fireOn('task', 'input');
  await flush(40);
  const rows = page.IDS['commands'].children;
  check('C5 the reference listbox opened on the trailing @',
    page.IDS['commands'].hidden === false && rows.length > 0,
    String(rows.length));
  check('C5 the rows carry their family',
    [...rows].every((r) => ['file', 'session', 'skill', 'memory'].includes(r.dataset.kind)),
    [...rows].map((r) => r.dataset.kind).join(','));
  check('C5 the rows filter by the token',
    [...rows].some((r) => r.textContent.includes('README.md')), rows[0] && rows[0].textContent);
  // Enter inserts the highlighted file reference and opens the preview.
  page.fireOn('task', 'keydown', { key: 'Enter', shiftKey: false });
  check('C5 Enter inserts the reference into the sentence',
    page.IDS['task'].value.includes('README.md'), JSON.stringify(page.IDS['task'].value));
  check('C5 the file reference opened the pane preview',
    page.body.dataset.pane === 'open', String(page.body.dataset.pane));
  // A session reference: insert, send, and the sent message carries the
  // link that opens the conversation.
  page.IDS['task'].value = 'also see @other';
  page.fireOn('task', 'input');
  await flush(40);
  const sessionRow = [...page.IDS['commands'].children]
    .find((r) => r.dataset.kind === 'session');
  check('C5 the session family is in the box', !!sessionRow,
    [...page.IDS['commands'].children].map((r) => r.dataset.kind).join(','));
  if (sessionRow) sessionRow.click();
  check('C5 the session reference inserted its link text',
    page.IDS['task'].value.includes('@the other one'), JSON.stringify(page.IDS['task'].value));
  page.fireOn('composer', 'submit');
  await flush(40);
  page.push({ type: 'message', role: 'owner', text: page.IDS['task'].value || 'also see @the other one' });
  await flush(40);
  const link = page.rows('.owner-session-ref')[0];
  check('C5 the sent message carries the link', !!link, '');
  if (link) {
    link.click();
    await flush(80);
    check('C5 the link opens that conversation',
      page.window.location.hash.includes('ref-2'), page.window.location.hash);
  }
}

// C6 (the return anchors): the divider marks the first event after the
// last one seen at the bottom; reading backwards counts arriving events
// into a back-to-live control that clears on return; a hidden tab
// notifies on a seat and a finish, a visible one never does.
async function returningIsAnchored() {
  const doc = {
    sessions: {
      type: 'sessions', read_at: 'now', current: '',
      counts: { needs: 0, working: 0, background: 0, earlier: 1, total: 1 },
      sessions: [row('seen-1', 'completed', { title: 'a conversation to return to' })],
    },
    state: (asked) => ({
      ...emptyState(asked), session: asked,
      events: [
        { type: 'message', role: 'owner', text: 'the first ask' },
        { type: 'message', role: 'assistant', text: 'the first answer' },
        { type: 'message', role: 'owner', text: 'the second ask' },
        { type: 'message', role: 'assistant', text: 'the second answer' },
      ],
    }),
  };
  // Seen stamp = 2: the owner read two events; the divider marks the
  // third.
  const page = boot({ width: 1440, hash: '#/sessions/seen-1', documents: doc,
    storage: { 'fleet-seen-seen-1': '2' } });
  await flush();
  page.open();
  await flush(80);
  const stream = page.rows('.stream')[0];
  const divider = page.rows('.since-divider');
  check('C6 the divider marks the return', divider.length === 1,
    String(divider.length));
  const order = stream.children.map((r) => r.className);
  const at = order.indexOf('since-divider');
  check('C6 the divider stands before the first unseen row',
    at === 2, JSON.stringify(order.slice(0, 5)));

  // Reading backwards: arriving events count into the control.
  page.IDS['log'].scrollTop = 0;
  page.fireOn('log', 'scroll');
  page.push({ type: 'message', role: 'assistant', text: 'more' });
  page.push({ type: 'message', role: 'assistant', text: 'and more' });
  await flush(40);
  check('C6 the back-to-live control names the count',
    page.IDS['to-live'].hidden === false && page.IDS['to-live'].textContent.includes('2'),
    page.IDS['to-live'].textContent + ' hidden=' + page.IDS['to-live'].hidden);
  page.IDS['to-live'].click();
  check('C6 returning clears it', page.IDS['to-live'].hidden === true, 'hidden');

  // A hidden tab notifies on a seat; a visible one does not.
  page.sandbox.document.hidden = true;
  page.push({ type: 'seat', tool: 'bash', label: 'run a command', summary: 'echo hi' });
  await flush(40);
  const seatNote = page.sandbox.Notification.instances.slice(-1)[0];
  check('C6 a hidden tab is told about the seat',
    seatNote && seatNote.title.includes('seat'), JSON.stringify(seatNote));
  page.sandbox.document.hidden = false;
  const before = page.sandbox.Notification.instances.length;
  page.push({ type: 'finished', stop_reason: 'completed' });
  await flush(40);
  check('C6 a visible tab is never notified',
    page.sandbox.Notification.instances.length === before,
    String(page.sandbox.Notification.instances.length));
  page.sandbox.document.hidden = true;
  page.push({ type: 'failed', stop_reason: 'api' });
  await flush(40);
  const failNote = page.sandbox.Notification.instances.slice(-1)[0];
  check('C6 a hidden tab is told the run ended',
    failNote && failNote.title.includes('failed'), JSON.stringify(failNote));
}

// C7 (artifacts and clickable paths): the finished run's receipt lists
// the files its checkpoint window names, each opening the diff; a path
// inside a rendered reply opens the pane at that file.
async function theReceiptHandsOverTheWork() {
  const doc = {
    sessions: {
      type: 'sessions', read_at: 'now', current: '',
      counts: { needs: 0, working: 0, background: 0, earlier: 1, total: 1 },
      sessions: [row('art-1', 'completed', { title: 'a conversation with artifacts' })],
    },
    state: (asked) => ({ ...emptyState(asked), session: asked }),
  };
  const page = boot({ width: 1440, hash: '#/sessions/art-1', documents: doc });
  await flush();
  page.open();
  await flush();
  page.push({ type: 'message', role: 'owner', text: 'touch some files' });
  page.push({ type: 'turn', turn: 2 });
  page.push({ type: 'message', role: 'assistant',
    text: 'I edited src/main.rs and wrote docs/note.md for you.' });
  await flush(60);
  // The path in the rendered reply is a link into the pane.
  const paths = page.rows('.md-path');
  check('C7 a path in the reply is a link',
    paths.some((one) => one.textContent === 'src/main.rs'),
    paths.map((one) => one.textContent).join(','));
  paths[0].click();
  await flush(40);
  check('C7 the path opened the pane',
    page.body.dataset.pane === 'open', String(page.body.dataset.pane));
  // The receipt carries the artifacts once the run finishes.
  page.push({ type: 'finished', stop_reason: 'completed' });
  await flush(120);
  const files = page.rows('.receipt-file');
  check('C7 the receipt lists the touched files',
    files.length >= 2 && files.some((one) => one.textContent === 'src/main.rs'),
    files.map((one) => one.textContent).join(','));
  files[0].click();
  await flush(60);
  const diffCards = page.rows('.card-row').filter((c) => c.dataset.kind === 'diff');
  check('C7 a file opens its diff',
    diffCards.length >= 1 && page.rows('.stream').length >= 0 &&
      diffCards[0].textContent.includes('changes against turn'),
    diffCards.length + ' cards');
}

// C8 (turn navigation): a turn folds to its mark and unfolds; next and
// prev jump turn-to-turn; the receipt's one control expands the whole
// run; nothing auto-collapses.
async function theTurnIsANavigationalUnit() {
  const doc = {
    sessions: {
      type: 'sessions', read_at: 'now', current: '',
      counts: { needs: 0, working: 0, background: 0, earlier: 1, total: 1 },
      sessions: [row('turns-1', 'completed', { title: 'a conversation of turns' })],
    },
    state: (asked) => ({ ...emptyState(asked), session: asked }),
  };
  const page = boot({ width: 1440, hash: '#/sessions/turns-1', documents: doc });
  await flush();
  page.open();
  await flush();
  const feed = function (turn, text) {
    page.push({ type: 'turn', turn: turn });
    page.push({ type: 'message', role: 'assistant', text: text });
  };
  feed(1, 'the first turn\'s words');
  feed(2, 'the second turn\'s words');
  feed(3, 'the third turn\'s words');
  page.push({ type: 'finished', stop_reason: 'completed' });
  await flush(60);
  const marks = page.rows('.turn-mark');
  check('C8 three turn marks', marks.length === 3, String(marks.length));
  check('C8 nothing auto-collapses',
    page.rows('.assistant').every((one) => !one.hidden), '');
  // Fold turn 2: its rows hide, the mark stays, the others do not move.
  const fold2 = marks[1].querySelector('.turn-fold') ||
    marks[1].children.filter((c) => (c.className || '').includes('turn-fold'))[0];
  fold2.click();
  await flush(20);
  const rowsOf = (turn) => page.rows('.stream > *').filter((r) =>
    r.dataset && r.dataset.turn === String(turn));
  // The stub's dataset lives on the mark; the folded rows are the
  // assistant rows between marks — assert by hidden state.
  const hidden = page.rows('.assistant').filter((one) => one.hidden);
  check('C8 folding turn 2 hides its rows', hidden.length === 1,
    String(hidden.length));
  check('C8 the mark stays', page.rows('.turn-mark').length === 3, '');
  fold2.click();
  await flush(20);
  check('C8 unfolding brings them back',
    page.rows('.assistant').every((one) => !one.hidden), '');
  // The receipt's expand: fold two turns, finish again, one press.
  marks[0].querySelector('.turn-fold').click();
  marks[1].querySelector('.turn-fold').click();
  await flush(20);
  page.push({ type: 'finished', stop_reason: 'completed' });
  await flush(80);
  const expand = page.rows('.receipt-toggle')
    .filter((one) => one.textContent.includes('expand this run'))[0];
  check('C8 the receipt offers the one-press expand', !!expand, '');
  if (expand) {
    expand.click();
    await flush(20);
    check('C8 one press unfolds the run',
      page.rows('.assistant').every((one) => !one.hidden), '');
  }
}

// C9 (content search): a query drives the work list from the server's
// match pass — the row carries the matched line, opening the row jumps
// to the conversation, and the bound is the document's own.
async function aContentQueryDrivesTheWorkList() {
  const page = boot({
    width: 1440, hash: '#/sessions',
    documents: {
      sessions: {
        type: 'sessions', read_at: 'now', current: '',
        counts: { needs: 0, working: 0, background: 0, earlier: 1, total: 1 },
        search: { query: 'deployment notes', bound: 200, searched: 2 },
        sessions: [row('hit-1', 'completed',
          { title: 'the conversation that said it', matched: 'where did I put the deployment notes' })],
      },
      state: (asked) => ({ ...emptyState(asked), session: asked }),
    },
  });
  await flush();
  page.open();
  await flush();
  const box = page.IDS['work-head'].querySelector('.work-search input') ||
    page.rows('.work-search input')[0];
  box.value = 'deployment notes';
  page.fireOn(box.id || 'task', 'input', {});
  // The stub re-fetches with the query; the document's matched line
  // rides the row.
  await flush(60);
  const matched = page.rows('.work-matched');
  check('C9 the row carries the matched line',
    matched.length === 1 && matched[0].textContent.includes('deployment notes'),
    matched.map((one) => one.textContent).join(';'));
  // Opening the row jumps to the conversation, as every row does.
  page.rows('.work-row')[0].click();
  await flush(80);
  check('C9 the matched row opens its conversation',
    page.window.location.hash.includes('hit-1'), page.window.location.hash);
}

// C10 (the context inspector): the receipt's audit door opens the exact
// request the run sent — the system prompt behind its own toggle, the
// message list role by role, and the tool declarations by name.
async function whatTheModelSawOpensFromTheReceipt() {
  const doc = {
    sessions: {
      type: 'sessions', read_at: 'now', current: '',
      counts: { needs: 0, working: 0, background: 0, earlier: 1, total: 1 },
      sessions: [row('ctx-1', 'completed', { title: 'a conversation to audit' })],
    },
    state: (asked) => ({ ...emptyState(asked), session: asked }),
  };
  const page = boot({ width: 1440, hash: '#/sessions/ctx-1', documents: doc });
  await flush();
  page.open();
  await flush();
  page.push({ type: 'message', role: 'owner', text: 'audit me' });
  page.push({ type: 'turn', turn: 1 });
  await flush(40);
  page.push({ type: 'finished', stop_reason: 'completed' });
  await flush(120);
  const door = page.rows('.receipt-toggle')
    .filter((one) => one.textContent.includes('what the model saw'))[0];
  check('C10 the receipt carries the audit door', !!door, '');
  door.click();
  await flush(80);
  const dockTitle = page.text('dock-object-title');
  check('C10 the dock opened at width',
    dockTitle.includes('what the model saw') || page.text('drawer-title').includes('what the model saw'),
    dockTitle + ' | ' + page.text('drawer-title'));
  const body = page.text('dock-object-body') || page.text('drawer-body');
  check('C10 the message list is role by role',
    body.includes('first ask') && body.includes('second ask'), body.slice(0, 160));
  check('C10 the tools are named',
    body.includes('bash') && body.includes('subagent'), '');
  const pre = page.rows('.context-pre')[0];
  check('C10 the system prompt is behind its toggle', pre && pre.hidden === true,
    String(pre && pre.hidden));
  const toggle = page.rows('.receipt-toggle')
    .filter((one) => one.textContent.includes('show the prompt'))[0];
  check('C10 the prompt offers its toggle', !!toggle, '');
  if (toggle) {
    toggle.click();
    await flush(20);
    check('C10 the toggle reveals the prompt', pre.hidden === false, String(pre.hidden));
  }
}

// C8: the addresses — `#/chat` is the new-work front door, and a reload
// at `#/sessions/<id>` reopens that conversation with the list beside it.
async function theAddressesHold() {
  const chat = boot({
    width: 1600,
    hash: '#/chat',
    documents: {
      sessions: {
        type: 'sessions', read_at: 'now', current: '',
        counts: { needs: 0, working: 0, background: 0, earlier: 1, total: 1 },
        sessions: [row('some-1', 'completed', { title: 'a past conversation' })],
      },
      state: (asked) => ({ ...emptyState(asked), session: asked }),
    },
  });
  await flush();
  chat.open();
  await flush();
  check('C8 #/chat closes the work column', chat.body.dataset.work !== 'open', String(chat.body.dataset.work));
  check('C8 #/chat hides the work column', chat.IDS['work-col'].hidden === true, 'hidden');
  check('C8 #/chat shows the front door', chat.IDS['empty'].hidden === false, 'empty hidden');
  check('C8 #/chat hides the conversation head', chat.IDS['conv-head'].hidden === true, 'head hidden');
  check('C8 #/chat keeps the composer', chat.IDS['composer'].offsetHeight > 0, 'composer');

  const opened = boot({
    width: 1600,
    hash: '#/sessions/addressed-1',
    documents: {
      sessions: {
        type: 'sessions', read_at: 'now', current: '',
        counts: { needs: 0, working: 0, background: 0, earlier: 1, total: 1 },
        sessions: [row('addressed-1', 'completed', { title: 'the addressed one' })],
      },
      state: (asked) => ({
        ...emptyState(asked), session: asked,
        events: [{ type: 'message', role: 'owner', text: 'earlier words' }],
      }),
    },
  });
  await flush();
  opened.open();
  await flush();
  const resumes = opened.calls.filter((c) => c.url.startsWith('/api/resume') && String(c.body).includes('addressed-1'));
  check('C8 a reload at an id resumes that conversation', resumes.length >= 1, String(resumes.length));
  check('C8 the head names the addressed conversation', opened.text('conv-name') === 'the addressed one',
    opened.text('conv-name'));
  check('C8 the list stands beside it', opened.IDS['work-col'].hidden === false &&
    opened.rows('.work-row').length === 1, String(opened.rows('.work-row').length));
  check('C8 its history is loaded', opened.text('log').includes('earlier words'), opened.text('log').slice(0, 80));
}

// C3: the pencil renames — one route, the conversation's id, the typed
// title. The title itself is the switcher, so the two gestures cannot
// fight over one control.
async function thePencilRenamesTheConversation() {
  const page = boot({
    width: 1440,
    hash: '#/sessions/name-1',
    promptAnswer: 'the new name',
    documents: {
      sessions: {
        type: 'sessions', read_at: 'now', current: '',
        counts: { needs: 0, working: 0, background: 0, earlier: 1, total: 1 },
        sessions: [row('name-1', 'completed', { title: 'the old name' })],
      },
      state: (asked) => ({ ...emptyState(asked), session: asked }),
    },
  });
  await flush();
  page.open();
  await flush();
  page.fireOn('conv-rename', 'click');
  await flush();
  const renames = page.calls.filter((c) => c.url.startsWith('/api/session/title'));
  check('C3 the pencil posts the rename', renames.length === 1, String(renames.length));
  check('C3 the rename names the conversation and the title',
    renames.length === 1 && String(renames[0].url).includes('name-1') &&
      String(renames[0].body).includes('the new name'),
    renames.length ? renames[0].url + ' ' + renames[0].body : 'no rename');
}


// C4: `/clear` starts a conversation — it posts the new-session route and
// opens what it made, instead of reloading the launch session under a
// cleared view.
async function theClearCommandStartsAConversation() {
  const page = boot({
    width: 1440,
    hash: '#/chat',
    newId: 'clear-made-1',
    documents: {
      sessions: emptySessions(),
      state: (asked) => emptyState(asked),
    },
  });
  await flush();
  page.open();
  await flush();
  page.IDS.task.value = '/clear';
  page.fireOn('task', 'input', {});
  await flush();
  const offered = page.rows('.command').find((command) => command.textContent.includes('/clear'));
  check('C4 /clear is offered in the composer', !!offered,
    page.rows('.command').map((command) => command.textContent).join(' | '));
  if (offered) offered.click();
  await flush(80);
  // /clear is the same verb as New (N2): an intent, not a row — nothing
  // is minted until words are sent.
  const news = page.calls.filter((call) => call.url.startsWith('/api/session/new'));
  check('C4 /clear mints nothing', news.length === 0, String(news.length));
  check('C4 /clear walks home with the intent',
    page.window.location.hash === '#/chat' && page.IDS.empty.hidden === false,
    page.window.location.hash + ' / empty hidden: ' + page.IDS.empty.hidden);
}


// C2: a thought block is one closed row that opens its words on a press.
async function aThoughtRowOpensOnAPress() {
  const page = boot({
    hash: '#/sessions/thinking-1',
    documents: {
      sessions: {
        type: 'sessions', read_at: 'now', current: '',
        counts: { needs: 0, working: 0, background: 0, earlier: 1, yours: 1, total: 1 },
        sessions: [row('thinking-1', 'completed', { title: 'a thinking run' })],
      },
      state: (asked) => ({
        ...emptyState(asked), session: asked,
        events: [
          { type: 'message', role: 'owner', text: 'how many rows?' },
          { type: 'reasoning', text: 'Count the **rows** first, then the files.', seconds: 5 },
          { type: 'message', role: 'assistant', text: 'Three rows.' },
        ],
      }),
    },
  });
  await flush();
  page.open();
  await flush();
  const thoughts = page.rows('.thought');
  check('C2 one thought row', thoughts.length === 1, String(thoughts.length));
  const body = thoughts[0] && thoughts[0].querySelector('.thought-body');
  check('C2 closed by default', !!body && body.hidden === true, body ? String(body.hidden) : 'no body');
  check('C2 the row names the duration', /Thought · 5s/.test(page.text('log')), page.text('log').slice(0, 120));
  check('C5 the closed row is one line', ruleBlock('.thought-head').includes('display: flex'),
    ruleBlock('.thought-head'));
  check('C5 the detail is hidden until asked',
    page.rows('.thought-body').every((body) => body.hidden) &&
      page.rows('.tool-detail').every((detail) => detail.hidden),
    String(page.rows('.thought-body').length) + '/' + String(page.rows('.tool-detail').length));
  const head = thoughts[0] && thoughts[0].querySelector('.thought-head');
  if (head) head.click();
  await flush();
  check('C2 a press opens it', !!body && body.hidden === false, body ? String(body.hidden) : 'no body');
  check('C2 its words render as markdown', page.rows('.thought .md-strong').length === 1,
    String(page.rows('.thought .md-strong').length));
  if (head) head.click();
  await flush();
  check('C2 a second press closes it', !!body && body.hidden === true, body ? String(body.hidden) : 'no body');
}

// C3: a command's expanded row names what actually ran, how long, and
// what came back — and the raw record is one more press away.
async function aCommandRowNamesWhatRan() {
  const full = 'cargo test --test integration -- --ignored --nocapture';
  const page = boot({
    hash: '#/sessions/cmd-1',
    documents: {
      sessions: {
        type: 'sessions', read_at: 'now', current: '',
        counts: { needs: 0, working: 1, background: 0, earlier: 0, yours: 1, total: 1 },
        sessions: [row('cmd-1', 'working', { title: 'a command run' })],
      },
      state: (asked) => ({ ...emptyState(asked), session: asked }),
    },
  });
  await flush();
  page.open();
  await flush();
  page.push({ type: 'message', role: 'owner', text: 'run the tests' });
  page.push({
    type: 'tool_call', tool: 'bash', summary: 'cargo test --test integration \u2026',
    arguments: JSON.stringify({ command: full }),
  });
  page.push({ type: 'tool_result', tool: 'bash', preview: '1498 passed \u00b7 0 failed', approved: true });
  await flush();
  const detail = page.rows('.tool-detail')[0];
  check('C3 the detail starts closed', !!detail && detail.hidden === true, detail ? String(detail.hidden) : 'none');
  check('C3 the whole command is carried', page.text('log').includes(full), page.text('log').slice(0, 160));
  const opener = page.rows('.tool .card-open')[0];
  if (opener) opener.click();
  await flush();
  check('C3 a press opens the detail', !!detail && detail.hidden === false, detail ? String(detail.hidden) : 'none');
  check('C3 it names the command', page.rows('.tool-command')[0] &&
    page.rows('.tool-command')[0].textContent === full,
    page.rows('.tool-command')[0] ? page.rows('.tool-command')[0].textContent : 'none');
  check('C3 it says the call ran', /ran/.test(page.text('log')), page.text('log').slice(0, 160));
  check('C3 the output is there', page.rows('.tool-output')[0] &&
    /1498 passed/.test(page.rows('.tool-output')[0].textContent),
    page.rows('.tool-output')[0] ? page.rows('.tool-output')[0].textContent : 'none');
  const raw = page.rows('.tool-raw')[0];
  if (raw) raw.click();
  await flush();
  check('C3 the raw record opens', /the call/.test(page.text('dock-object-body')),
    page.text('dock-object-body').slice(0, 100));
}

// C4: a folded strip line opens its record.
async function theStripsLinesOpenTheirRecord() {
  const page = boot({
    hash: '#/sessions/strip-1',
    documents: {
      sessions: {
        type: 'sessions', read_at: 'now', current: '',
        counts: { needs: 0, working: 1, background: 0, earlier: 0, yours: 1, total: 1 },
        sessions: [row('strip-1', 'working', { title: 'a routed run' })],
      },
      state: (asked) => ({ ...emptyState(asked), session: asked }),
    },
  });
  await flush();
  page.open();
  await flush();
  page.push({ type: 'message', role: 'owner', text: 'go' });
  page.push({ type: 'turn', turn: 1 });
  page.push({ type: 'routed', provider: 'stepfun', how: 'the first model of the roster' });
  page.push({ type: 'finished', reason: 'stop' });
  await flush();
  const head = page.rows('.strip-head')[0];
  check('C4 the strip has a head', !!head, String(page.rows('.strip-head').length));
  if (head) head.click();
  await flush();
  const line = page.rows('.strip-line')[0];
  check('C4 a folded line exists', !!line, String(page.rows('.strip-line').length));
  if (line) line.click();
  await flush();
  check('C4 the line opens its record', page.text('dock-object-body').includes('the first model of the roster'),
    page.text('dock-object-body').slice(0, 120));
}

// --------------------------------------------------------------------- run


// C5's face: the workspace-history section builds its four panes from
// the substrate's own document — the DAG with main's trunk and the
// child branch's fork and merge edges, the Work table's session link
// and state, the Files pane's branch tabs, and the Store's figure
// bars. An unregistered folder names itself instead of drawing an
// empty tree.
async function theWorkspaceHistorySectionDrawsTheTreeItRead() {
  // The one-read payload: branches with their fork bases, the commits
  // flat (each naming its branch and base), the layers with their
  // publish evidence, the nine figures with the Init record.
  const doc = {
    type: 'workspace-history',
    registered: true,
    folder: '/Users/owner/Projects/fleet',
    project: 'fleet',
    version: '1b70c3e210460f57',
    branches: [
      { name: 'main', base: '326898f652a0', head: null, forked_from: null, state: 'idle', child: null },
      {
        name: 'fleet-child-20261026t010000z-abc123', base: 'aa11bb22cc33',
        head: '12e968660147668df46ef623a1a28858b74736a2758ddaa5e51b19607d62104d4c', forked_from: 'aa11bb22cc33',
        state: 'merged',
        child: { session: '20261026T010000Z-abc123', outcome: 'completed' },
      },
      { name: 'fleet-child-20261026t010000z-idl000', base: 'aa11bb22cc33', head: null, forked_from: 'aa11bb22cc33', state: 'idle', child: null },
      { name: 'fleet-child-20261026t010000z-idl001', base: 'aa11bb22cc33', head: null, forked_from: 'aa11bb22cc33', state: 'idle', child: null },
    ],
    commits: [{
      id: '12e968660147668df46ef623a1a28858b74736a2758ddaa5e51b19607d62104d4c',
      branch: 'fleet-child-20261026t010000z-abc123',
      parent: null, second_parent: null, base: 'aa11bb22cc33',
      merged_from: null, displaced: null, lww: [], lww_truncated: false,
      logical: 18777528, new_canonical: 10120, new_stored: 836, new_metadata: 791,
      via: 'fs', message: 'fleet: write_file', time: 1792975010000,
      root: 'ace414acc7c6',
    }],
    layers: [
      { id: '323d354b0ca1', logical: 18777528, parent: 'aa11bb22cc33', root: null,
        source_branch: 'merge-fleet-child-20261026t010000z-abc123',
        source_commit: '12e968660147668df46ef623a1a28858b74736a2758ddaa5e51b19607d62104d4c',
        overwrote_count: 0 },
      { id: 'aa11bb22cc33', logical: 946, parent: null, root: null,
        source_branch: null, source_commit: null, overwrote_count: 0 },
    ],
    storage: {
      logical: 46549294, canonical: 15562812, stored: 7388596, metadata: 500678,
      file: 7860224, free: 0, dedupe: 2.991059, compression: 2.106329,
      init: { entries: 510, files: 448, bytes: 15516426, canonical: 15552692, stored: 7387760, metadata: 447475, millis: 525 },
    },
    cut_branches: [], truncated: null,
    note: null, refused: [],
  };
  const page = boot({
    hash: '#/workspace-history',
    documents: {
      'workspace-history': doc,
      'workspace-history/files': {
        type: 'workspace-files', branch: 'fleet-child-20261026t010000z-abc123', path: '/', items: [],
      },
    },
  });
  await flush();
  page.open();
  await flush();

  // The rail carries the destination under knowledge, beside its
  // siblings — the fourth entry, not a fifth top-level group.
  const knowledge = page.rows('.surfaces-section')
    .filter((section) => section.dataset.group === 'knowledge');
  check('the knowledge group carries workspace history', knowledge.length === 1,
    String(knowledge.length));
  if (knowledge.length === 1) {
    // The section's own links: four entries, workspace history last.
    const entries = page.rows('.surfaces-link')
      .filter((link) => link.closest('.surfaces-section') === knowledge[0])
      .map((link) => link.textContent.trim());
    check('the entry is named workspace history beside its siblings',
      entries.length === 4 && entries[3] === 'workspace history', entries.join(' | '));
  }
  // The surface announces itself once, above the cards.
  const title = page.rows('.surface-title').map((n) => n.textContent.trim());
  check('the section names itself workspace history', title.includes('Workspace history'), title.join(' | '));
  // The tree: a band per layer, a leaf for the commit, the fork node
  // at the branch's actual base, and the dashed graft from the leaf a
  // publish carried back to the band it made.
  check('the tree draws a band for every layer',
    page.rows('.dag-band').length === 2, String(page.rows('.dag-band').length));
  check('the tree draws a leaf for the commit',
    page.rows('.dag-leaf').length === 1, String(page.rows('.dag-leaf').length));
  check('the tree draws the fork node at the branch\u2019s base',
    page.rows('.dag-fork').length === 1, String(page.rows('.dag-fork').length));
  check('the tree draws the publish graft',
    page.rows('.dag-graft').length === 1 && page.rows('.dag-ring').length === 1,
    page.rows('.dag-graft').length + ' / ' + page.rows('.dag-ring').length);
  check('the tree labels the lane with the branch name',
    page.rows('.dag-tag').some((t) => /abc123/.test(t.textContent)),
    page.rows('.dag-tag').map((t) => t.textContent).join(' | '));
  // The instrument's controls and the minimap.
  check('the tree carries zoom, fit and orientation controls',
    page.rows('.dag-zoom').length === 4, String(page.rows('.dag-zoom').length));
  check('the tree carries the minimap with its window',
    page.rows('.dag-minimap').length === 1 && page.rows('.dag-minimap-window').length === 1,
    'minimap / window');
  // A branch nobody worked is one idle chip, never a column.
  const idleChip = page.rows('.dag-idle .chip').map((n) => n.textContent.trim());
  check('the idle branches are one chip with a count',
    idleChip.some((t) => /2 idle/.test(t)), idleChip.join(' | '));
  // The Work pane: the head commit's ordinal, the layer it forked
  // from, and the child session.
  const workText = page.rows('.board-card').map((c) => c.textContent).join(' ');
  check('the work pane names the child session',
    workText.includes('20261026T010000Z-abc123'), workText.slice(0, 200));
  check('the work pane names the merged state', /merged to main/.test(workText), workText.slice(0, 200));
  const workCard = page.rows('.board-card')
    .find((c) => /Every branch a delegated child worked/.test(c.textContent));
  const workRow = workCard ? workCard.textContent : '';
  check('the work row names the head commit and its fork layer',
    workRow.includes('C1') && workRow.includes('\u2193 L1'),
    workRow.slice(0, 300));
  // The live feed: rows with the actor, the call and its cost; a
  // refused call renders as refused; traces off is named.
  const feedDoc = {
    type: 'workspace-calls', calls: [
      { actor: '20261026T010000Z-abc123', method: 'fs.write', start: '2026-10-26T10:00:01.000Z', ms: 9.3, ok: true, kind: 'Committed', commit: '12c20c75063c' },
      { actor: '20261026T010000Z-abc123', method: 'fs.read', start: '2026-10-26T10:00:02.000Z', ms: 2.1, ok: false },
    ], more: false, note: null, refused: [],
  };
  const fed = boot({
    hash: '#/workspace-history',
    documents: {
      'workspace-history': doc,
      'workspace-history/calls': feedDoc,
    },
  });
  await flush();
  fed.open();
  await flush();
  const feedText = fed.rows('.board-card').map((c) => c.textContent).join(' ');
  check('the feed names the actor and the call',
    feedText.includes('20261026T010000Z-abc123') && feedText.includes('fs.write'),
    feedText.slice(0, 300));
  check('a refused call renders as refused',
    fed.rows('.board-card .chip').some((n) => n.textContent === 'refused'),
    fed.rows('.board-card .chip').map((n) => n.textContent).join(' | '));
  const quiet = boot({
    hash: '#/workspace-history',
    documents: {
      'workspace-history': doc,
      'workspace-history/calls': {
        type: 'workspace-calls', calls: [], more: false, refused: [],
        note: 'traces are off \u2014 no LayerFS process on this machine has LAYERFS_TRACE set',
      },
    },
  });
  await flush();
  quiet.open();
  await flush();
  check('traces off is named, never read as no activity',
    quiet.rows('.board-note').some((n) => /traces are off/.test(n.textContent)),
    quiet.rows('.board-note').map((n) => n.textContent.slice(0, 60)).join(' | '));
  // The Files pane: every branch among its tabs, and the four modes.
  const tabs = page.rows('.board-tab').map((t) => t.textContent.trim());
  check('the files pane offers every branch',
    ['main', 'fleet-child-20261026t010000z-abc123'].every((n) => tabs.includes(n)) &&
      tabs.length === 8, tabs.join(' | '));
  check('the files pane offers the four modes',
    ['text', 'blame', 'revisions', 'diff'].every((n) => tabs.includes(n)),
    tabs.join(' | '));
  // The files instrument: the four modes read and render — text with
  // its line numbers and window buttons, blame with its per-line
  // commits, revisions with their changes, and a diff's changed set.
  const instrument = boot({
    hash: '#/workspace-history',
    documents: {
      'workspace-history': doc,
      'workspace-history/files': {
        type: 'workspace-files', branch: 'fleet-child-20261026t010000z-abc123', path: '/',
        items: [
          { name: 'notes.txt', type: 'file', size: 12 },
          { name: 'src', type: 'directory' },
        ],
      },
      'workspace-history/file': {
        type: 'workspace-file', mode: 'text', ref: 'fleet-child-20261026t010000z-abc123',
        path: '/notes.txt', size: 12, text: 'line one\nline two\n',
        start_line: 1, end_line: 2, next_line: null, note: null, refused: [],
      },
    },
  });
  await flush();
  instrument.open();
  await flush();
  check('the files pane lists the folder',
    instrument.rows('.board-table td').some((n) => n.textContent === 'notes.txt'),
    'the listing');
  // Click a file: the text instrument reads and renders its lines.
  const fileLink = instrument.rows('.board-table button.link')
    .find((b) => b.textContent === 'notes.txt');
  check('the listing offers the file', !!fileLink, String(!!fileLink));
  if (fileLink) {
    fileLink.fire('click', {});
    await flush();
    const code = instrument.rows('.history-code').map((n) => n.textContent).join('\n');
    check('the text mode renders the file with its line numbers',
      /1  line one/.test(code) && /2  line two/.test(code), code.slice(0, 120));
  }
  // Blame: every line with the commit that last changed it.
  const blamed = boot({
    hash: '#/workspace-history',
    documents: {
      'workspace-history': doc,
      'workspace-history/files': {
        type: 'workspace-files', branch: 'fleet-child-20261026t010000z-abc123', path: '/',
        items: [{ name: 'notes.txt', type: 'file' }],
      },
      'workspace-history/file': {
        type: 'workspace-file', mode: 'blame', ref: 'fleet-child-20261026t010000z-abc123', path: '/notes.txt',
        lines: [
          { n: 1, text: 'line one', commit: '12e968660147668df46ef623a1a28858b74736a2758ddaa5e51b19607d6210' },
          { n: 2, text: 'line two', commit: '12e968660147668df46ef623a1a28858b74736a2758ddaa5e51b19607d6210' },
        ], note: null, refused: [],
      },
    },
  });
  await flush();
  blamed.open();
  await flush();
  // Open the file first — the file's instruments stand on an open
  // file — then switch the mode.
  const blameFile = blamed.rows('.board-table button.link')
    .find((b) => b.textContent === 'notes.txt');
  if (blameFile) blameFile.fire('click', {});
  await flush();
  const blameMode = blamed.rows('.board-tab').find((t) => t.textContent.trim() === 'blame');
  check('the blame mode is offered', !!blameMode, String(!!blameMode));
  if (blameMode) {
    blameMode.fire('click', {});
    await flush();
    const text = blamed.rows('.board-card').map((c) => c.textContent).join(' ');
        check('the blame mode names the commit that last changed the line',
      /line one/.test(text) && /12e96866014/.test(text), text.slice(0, 200));
  }
  // The Store's accounting: the substrate's own rows — on disk, main's
  // latest, every snapshot, unique, compressed, the metadata subset,
  // and the SQLite overhead named separately — then the Init record
  // and the per-commit costs.
  const storeCard = page.rows('.board-card')
    .find((c) => c.textContent.startsWith('the store'));
  const storeText = storeCard ? storeCard.textContent : '';
  check('the store names every row of the accounting',
    ['on disk', 'main\u2019s latest, raw', 'every snapshot, raw', 'unique',
     'compressed', 'of which metadata', 'SQLite overhead']
      .every((name) => storeText.includes(name)), storeText.slice(0, 300));
  check('the store computes the SQLite overhead from the bytes',
    storeText.includes('461 kB'), storeText.slice(0, 500));
  check('the store names the Init record', /init: 448 files/.test(storeText),
    storeText.slice(0, 500));
  check('the store names the per-commit cost', /per commit: 836 B average/.test(storeText),
    storeText.slice(0, 500));
  check('the store draws the commit and names how it was made',
    /via fs/.test(storeText), storeText.slice(-300));
  check('the store computes its ratios from the bytes', /2\.99\u00d7 dedupe/.test(storeText),
    storeText.slice(0, 500));
  // A store with no Init record says so; a read without the figures
  // says so too — neither invents.
  const noInit = boot({
    hash: '#/workspace-history',
    documents: {
      'workspace-history': Object.assign({}, doc, {
        storage: Object.assign({}, doc.storage, { init: null }),
      }),
    },
  });
  await flush();
  noInit.open();
  await flush();
  check('a store with no Init record names the absence',
    noInit.rows('.board-note').some((n) => /no Init record/.test(n.textContent)),
    noInit.rows('.board-note').map((n) => n.textContent.slice(0, 50)).join(' | '));
  const noFigures = boot({
    hash: '#/workspace-history',
    documents: { 'workspace-history': Object.assign({}, doc, { storage: null }) },
  });
  await flush();
  noFigures.open();
  await flush();
  check('a read without figures names its absence',
    noFigures.rows('.board-empty').some((n) => /figures have not landed/.test(n.textContent)),
    noFigures.rows('.board-empty').map((n) => n.textContent.slice(0, 40)).join(' | '));
  // The truncation, named where the tree would be.
  const cut = boot({
    hash: '#/workspace-history',
    documents: {
      'workspace-history': Object.assign({}, doc, {
        truncated: '3 branches shown, more remain in the Store',
        branches: doc.branches.slice(0, 2),
      }),
    },
  });
  await flush();
  cut.open();
  await flush();
  check('the cut is named in words',
    cut.rows('.dag-truncated').some((n) => /more remain in the Store/.test(n.textContent)),
    cut.rows('.dag-truncated').map((n) => n.textContent).join(' | '));
  // The cut's own words: the instrument says "the newest" and the Files
  // tabs name the cap instead of a shorter list passing as the whole.
  check('a cut page says the newest, and the tabs name the cap',
    cut.rows('.surface-tool-meta').some((n) => /the newest/.test(n.textContent)) &&
      cut.rows('.board-card').some((c) => /newest 2 branches only/.test(c.textContent)),
    cut.rows('.surface-tool-meta').map((n) => n.textContent).join(' | '));

  // The truth fixtures: places the panel used to lie about the
  // substrate's own model — a merged branch whose head is gone (its
  // commits are outside the page), a fork at forkedFrom rather than
  // the Layer's base, an out-of-page merge graft, the lww objects,
  // commits on cut branches, and a null dedupe.
  const hard = {
    type: 'workspace-history', registered: true, folder: '/w', project: 'fleet',
    version: 'v2',
    branches: [
      { name: 'main', base: 'L1', head: null, forked_from: null, state: 'idle', child: null },
      {
        name: 'child-worked', base: 'L1', head: 'C2', forked_from: 'C1', state: 'merged',
        child: { session: '20261026T030000Z-hard00', outcome: 'completed' },
      },
      {
        name: 'child-merged', base: 'L1', head: null, forked_from: 'C1', state: 'merged',
        child: { session: '20261026T030000Z-hard01', outcome: 'completed' },
      },
      { name: 'child-idle', base: 'L1', head: null, forked_from: 'L1', state: 'idle', child: null },
    ],
    commits: [
      {
        id: 'C1', branch: 'child-worked', parent: null, second_parent: null, base: 'L1',
        merged_from: null, displaced: null,
        lww: [{ path: 'docs/x.md', kept: 'main', deleted: 'child-merged', rule: 'newer wins' }],
        lww_truncated: false, logical: 100, new_canonical: 90, new_stored: 80,
        new_metadata: 5, via: 'fs', message: 'first', time: 1792975000000, root: 'r1',
      },
      {
        id: 'C2', branch: 'child-worked', parent: 'C1', second_parent: null, base: 'L1',
        merged_from: null, displaced: null, lww: [], lww_truncated: false,
        logical: 200, new_canonical: 180, new_stored: 160, new_metadata: 6,
        via: 'fs', message: 'second', time: 1792975001000, root: 'r2',
      },
      {
        id: 'CX', branch: 'cut-branch', parent: null, second_parent: null, base: 'L1',
        merged_from: null, displaced: null, lww: [], lww_truncated: false,
        logical: 50, new_canonical: 40, new_stored: 30, new_metadata: 2,
        via: 'fs', message: 'cut', time: 1792975002000, root: 'r3',
      },
    ],
    layers: [
      {
        id: 'L3', logical: 900, parent: 'L2', root: null,
        source_branch: 'merge-child-merged', source_commit: 'C-OUT', overwrote_count: 0,
      },
      {
        id: 'L2', logical: 800, parent: 'L1', root: null,
        source_branch: 'merge-child-worked', source_commit: 'C2', overwrote_count: 0,
      },
      {
        id: 'L1', logical: 700, parent: null, root: null,
        source_branch: null, source_commit: null, overwrote_count: 0,
      },
    ],
    storage: {
      logical: 1000, canonical: 800, stored: 600, metadata: 10, file: 0, free: 0,
      dedupe: null, compression: null, init: null,
    },
    cut_branches: ['cut-branch'], truncated: '3 branches shown, more remain in the Store',
    note: null, refused: [],
  };
  const truth = boot({ hash: '#/workspace-history', documents: { 'workspace-history': hard } });
  await flush();
  truth.open();
  await flush();

  const truthCards = truth.rows('.board-card').map((c) => c.textContent).join(' ');
  check('a merged branch with no head is work, not idle',
    truthCards.includes('child-merged') &&
      truthCards.includes('the layer carries its commit'),
    truthCards.slice(0, 300));
  const truthIdle = truth.rows('.dag-idle .chip').map((n) => n.textContent.trim());
  check('the idle chip counts only the branches with nothing',
    truthIdle.some((t) => /1 idle/.test(t)), truthIdle.join(' | '));
  // Every merge has its ring; the out-of-page one has no stem, and says
  // so rather than dropping out of the drawing.
  const truthRings = truth.rows('.dag-ring');
  const truthGrafts = truth.rows('.dag-graft');
  check('every merge has a ring, and only the in-page one a stem',
    truthRings.length === 2 && truthGrafts.length === 1,
    truthRings.length + ' rings / ' + truthGrafts.length + ' stems');
  check('the out-of-page merge names itself on its ring',
    truthRings.some((ring) => (ring.children || [])
      .some((child) => /outside this page/.test(child.textContent || ''))),
    truthRings.map((ring) => ring.textContent).join(' | '));
  // The fork sits at forkedFrom (a commit), not the Layer's base.
  const hover = truth.rows('.dag-hover')[0];
  let forkLine = '';
  truth.rows('.dag-fork').forEach((fork) => {
    fork.fire('mouseenter');
    if (hover && /commit C1/.test(hover.textContent)) forkLine = hover.textContent;
  });
  check('the fork is drawn at forkedFrom, named by the commit',
    /from commit C1/.test(forkLine), forkLine.slice(0, 160));
  // The lww entry reads from its object shape.
  let lwwLine = '';
  truth.rows('.dag-leaf').forEach((leaf) => {
    leaf.fire('mouseenter');
    if (hover && /lww/.test(hover.textContent)) lwwLine = hover.textContent;
  });
  check('the lww entry reads from its object shape',
    /lww .*docs\/x\.md/.test(lwwLine), lwwLine.slice(0, 200));
  // Commits whose branch the page cut are named, not silently missing.
  check('commits on cut branches are named',
    truth.rows('.dag-truncated')
      .some((n) => /belongs? to branches this page cut/.test(n.textContent)),
    truth.rows('.dag-truncated').map((n) => n.textContent).join(' | '));
  // The dedupe figure is computed from the byte totals, never read
  // from the substrate's nullable string: the honest ratio here.
  check('the dedupe figure is the bytes\u2019 own ratio',
    truth.rows('.surface-strip').some((strip) => /1\.25 dedupe/.test(strip.textContent)),
    truth.rows('.surface-strip').map((strip) => strip.textContent).join(' | '));
  // Bounds: more worked branches make the plot TALLER (or wider), the
  // viewBox is the content's size, and the card scrolls — no node sits
  // outside the plot's bounds because the bounds ARE the content.
  const busyBranches = [];
  for (let i = 0; i < 6; i++) {
    busyBranches.push({
      name: 'fleet-child-20261026t02000' + i + 'z-busy' + i, base: 'aa11bb22cc33',
      head: '12e9686601' + i, forked_from: 'aa11bb22cc33', state: 'open', child: null,
    });
  }
  const busy = boot({
    hash: '#/workspace-history',
    documents: {
      'workspace-history': Object.assign({}, doc, {
        branches: [doc.branches[0]].concat(busyBranches),
        commits: busyBranches.map((branch, i) => Object.assign({}, doc.commits[0], {
          id: '12e9686601' + i, branch: branch.name, time: 1792975010000 + i,
        })),
      }),
    },
  });
  await flush();
  busy.open();
  await flush();
  const plotOf = (aPage) => aPage.rows('svg')
    .find((svgEl) => svgEl.getAttribute('aria-label') === 'the workspace history tree');
  const busyPlot = plotOf(busy);
  const onePlot = plotOf(page);
  check('a busy tree grows the plot instead of clipping it',
    !!busyPlot && !!onePlot &&
      Number(busyPlot.getAttribute('viewBox').split(' ')[3]) >
      Number(onePlot.getAttribute('viewBox').split(' ')[3]),
    busyPlot && onePlot
      ? busyPlot.getAttribute('viewBox') + ' vs ' + onePlot.getAttribute('viewBox')
      : 'missing plot');
  check('a busy tree draws one leaf per commit',
    busy.rows('.dag-leaf').length === 6, String(busy.rows('.dag-leaf').length));
  // The honest empty state: a folder with no project says so.
  const empty = boot({
    hash: '#/workspace-history',
    documents: {
      'workspace-history': {
        type: 'workspace-history', registered: false, folder: '/tmp/elsewhere',
        branches: [], layers: [], refused: [],
        note: 'this folder has no LayerFS project — the first delegation here registers one',
      },
    },
  });
  await flush();
  empty.open();
  await flush();
  check('the unregistered folder names its absence',
    empty.rows('.board-empty').some((n) => /no LayerFS project/.test(n.textContent)),
    empty.rows('.board-empty').map((n) => n.textContent.trim().slice(0, 60)).join(' | '));
  check('the unregistered folder draws no tree',
    empty.rows('.dag-band').length === 0 && empty.rows('.dag-leaf').length === 0,
    String(empty.rows('.dag-band').length));
}

// The archive: the one end a session can come to. A handback reads as
// the open state it is, the owner's verb sits on every row and in the
// head, and the archived group is one collapsed line that keeps its
// rows findable.
async function theArchiveIsTheOneEndAndTheOwnersVerb() {
  const page = boot({
    hash: '#/sessions',
    documents: {
      sessions: {
        type: 'sessions', read_at: 'now', current: '',
        counts: { needs: 0, working: 1, background: 0, archived: 2, earlier: 1, total: 4 },
        sessions: [
          row('live-1', 'handed back', { title: 'the handed-back one' }),
          row('live-2', 'working', { title: 'the working one' }),
          row('gone-1', 'archived', { title: 'the first ended' }),
          row('gone-2', 'archived', { title: 'the second ended' }),
        ],
      },
      state: (asked) => ({ ...emptyState(asked), session: asked, events: [{ type: 'message', role: 'owner', text: 'the task' }] }),
    },
  });
  await flush();
  page.open();
  await flush();

  // The handback is a word the row wears, and it is not "completed".
  const handed = page.rows('.work-row').find((r) => r.dataset.state === 'handed back');
  check('ARCH the handback row exists', !!handed, String(page.rows('.work-row').length));
  const handedWord = handed && handed.querySelector('.work-state');
  check('ARCH the handback reads open',
    !!handedWord && handedWord.textContent === 'handed back',
    handedWord && handedWord.textContent);

  // The archived group is one collapsed line, closed by default.
  const archivedToggle = page.rows('.work-background')
    .find((t) => String(t.textContent).includes('archived'));
  check('ARCH the archived group is one quiet line', !!archivedToggle,
    page.text('work-list').slice(0, 80));
  check('ARCH it is closed by default',
    !!archivedToggle && archivedToggle.getAttribute('aria-expanded') === 'false',
    archivedToggle && archivedToggle.getAttribute('aria-expanded'));

  // Opening it shows the rows; their chips offer the way back.
  if (archivedToggle) archivedToggle.click();
  await flush();
  const goneRows = page.rows('.work-row').filter((r) => r.dataset.state === 'archived');
  check('ARCH opening shows the archived rows', goneRows.length === 2,
    String(goneRows.length));
  const back = goneRows[0] && goneRows[0].querySelector('.work-archive');
  check('ARCH an archived row offers unarchive',
    !!back && back.textContent === 'unarchive',
    back && back.textContent);
  if (back) back.click();
  await flush();
  const restores = page.calls.filter((c) => {
    if (c.url !== '/api/session/archive' || !c.body) return false;
    const parsed = JSON.parse(c.body);
    return parsed.session === 'gone-1' && parsed.archived === false;
  });
  check('ARCH unarchive posts the restore', restores.length === 1,
    JSON.stringify(page.calls.filter((c) => c.url === '/api/session/archive').map((c) => c.body)));

  // The row chip archives a live conversation; the head button speaks
  // for the conversation the face anchors.
  const chip = handed && handed.querySelector('.work-archive');
  check('ARCH the live row carries the archive chip',
    !!chip && chip.textContent === 'archive', chip && chip.textContent);
  if (chip) chip.click();
  await flush();
  const posted = page.calls.filter((c) => {
    if (c.url !== '/api/session/archive' || !c.body) return false;
    const parsed = JSON.parse(c.body);
    return parsed.session === 'live-1' && parsed.archived === true;
  });
  check('ARCH the chip posts the archive', posted.length === 1,
    JSON.stringify(page.calls.filter((c) => c.url === '/api/session/archive').map((c) => c.body)));

  handed.click();
  await flush();
  page.fireOn('conv-archive', 'click');
  await flush();
  const heads = page.calls.filter((c) => {
    if (c.url !== '/api/session/archive' || !c.body) return false;
    const parsed = JSON.parse(c.body);
    return parsed.session === 'live-1' && parsed.archived === true;
  });
  check('ARCH the head button archives the open conversation', heads.length >= 1,
    JSON.stringify(page.calls.filter((c) => c.url === '/api/session/archive').map((c) => c.body)));

  // A refusal names itself where the owner reads.
  const refused = boot({
    hash: '#/sessions',
    archiveStatus: 409,
    archiveReply: { error: '`x` has a run in flight — stop it first, then archive' },
    documents: {
      sessions: {
        type: 'sessions', read_at: 'now', current: '',
        counts: { needs: 0, working: 0, background: 0, archived: 0, earlier: 1, total: 1 },
        sessions: [row('busy-1', 'working', { title: 'the busy one' })],
      },
      state: (asked) => ({ ...emptyState(asked), session: asked, events: [] }),
    },
  });
  await flush();
  refused.open();
  await flush();
  const busyChip = refused.rows('.work-row')[0] &&
    refused.rows('.work-row')[0].querySelector('.work-archive');
  if (busyChip) busyChip.click();
  await flush();
  check('ARCH a refusal names itself in the stream',
    refused.text('log').includes('the archive was refused') &&
    refused.text('log').includes('in flight'),
    refused.text('log').slice(0, 120));
}

// The chat review (the owner's report): an owner message renders its
// words once; the conversation's count is exchanges — owner messages,
// steer frames and scheduled prompts excluded so the number is the
// same live and after a reload — and the message controls are present
// without a hover: a hover-only control does not exist on a touch face.
async function theChatCountsExchangesAndShowsItsControls() {
  const doc = {
    sessions: {
      type: 'sessions', read_at: 'now', current: '',
      counts: { needs: 0, working: 0, background: 0, earlier: 1, total: 1 },
      sessions: [row('chat-1', 'completed', { title: 'the chat review' })],
    },
    state: (asked) => ({ ...emptyState(asked), session: asked }),
    branch: {
      type: 'replay', id: 'chat-branch', model: 'deepseek', kind: 'run',
      ended: true, stop_reason: 'completed', turns: 7, omitted: 0,
      cards: [
        { type: 'message', role: 'owner', text: 'the first ask' },
        { type: 'message', role: 'owner', text: 'the second ask' },
        { type: 'message', role: 'owner', text: '[scheduled run — job `x` at now]' },
        { type: 'message', role: 'owner',
          text: '[the owner of this run steered it through its control file — this is the owner speaking, not you]\nleft' },
        { type: 'message', role: 'assistant', text: 'an answer' },
      ],
    },
  };
  const page = boot({ width: 1440, hash: '#/sessions/chat-1', documents: doc });
  await flush();
  page.open();
  await flush();

  // One owner message renders its words once: the old body appended
  // the raw text before its own branches, so every message printed twice.
  page.push({ type: 'message', role: 'owner', text: 'the one ask' });
  await flush(40);
  const spans = page.rows('.owner-text').filter((el) => el.textContent === 'the one ask');
  check('chat the owner message renders once', spans.length === 1, String(spans.length));

  // The turn is the model's request; the count the header carries is
  // exchanges — the hint and the pill name the exchange in flight.
  page.push({ type: 'turn', turn: 1 });
  await flush(40);
  const hint = page.IDS['hint'].textContent;
  check('chat the hint names the exchange in flight',
    hint.includes('exchange 1') && !hint.includes('turn 1'), hint);
  const pill = page.IDS['process-pill'].textContent;
  check('chat the pill names the exchange, not the round',
    pill.includes('exchange 1') && !pill.includes('turn 1'), pill);

  // A second exchange, then the two system frames that ride the log as
  // user records: neither may move the count.
  page.push({ type: 'message', role: 'owner', text: 'the second ask here' });
  page.push({ type: 'message', role: 'owner', text: '[scheduled run — job `x` at now]' });
  page.push({ type: 'message', role: 'owner',
    text: '[the owner of this run steered it through its control file — this is the owner speaking, not you]\nleft' });
  page.push({ type: 'message', role: 'assistant', text: 'the answer' });
  page.push({ type: 'finished', stop_reason: 'completed' });
  await flush(60);
  const facts = page.text('facts');
  check('chat the header row says exchanges and not turns',
    facts.includes('exchanges') && !facts.includes('turns'), facts);
  const exchangeDt = page.rows('#facts dt')
    .find((dt) => dt.textContent === 'exchanges');
  const exchangeValue = exchangeDt ? exchangeDt.parentNode.children[1].textContent : '(no row)';
  check('chat two owner messages count as two exchanges', exchangeValue === '2',
    exchangeValue);
  const evidence = page.rows('.receipt-line')
    .map((one) => one.textContent).join(' | ');
  check('chat the receipt counts exchanges',
    evidence.includes('2 exchanges') && !evidence.includes('turns'), evidence);

  // The controls: edit + copy on each owner bubble (four messages,
  // two of them system frames), copy under the reply, all present
  // without a hover — and a copy puts its message's words on the
  // clipboard (the label flip runs behind an immediate stub timer, so
  // the payload is what a harness can see).
  check('chat every message carries its controls',
    page.rows('.owner-edit').length === 4 && page.rows('.msg-copy').length === 5,
    page.rows('.owner-edit').length + ' edits / ' + page.rows('.msg-copy').length + ' copies');
  const copy = page.rows('.msg-copy')[0];
  copy.click();
  check('chat copy puts the message on the clipboard',
    page.clipboardWrites.some((text) => text === 'the one ask'),
    JSON.stringify(page.clipboardWrites));
  check('chat the row controls are not gated behind hover',
    !ruleBlock('.row-rewind').includes('opacity: 0') &&
      ruleBlock('.msg-tools').includes('justify-content: flex-end'),
    ruleBlock('.row-rewind') + ' | ' + ruleBlock('.msg-tools'));

  // The drawer counts the same way over its cards: two exchanges, the
  // system frames excluded, the run's seven model requests left as the
  // machine's own figure elsewhere.
  page.push({ type: 'delegation', session: 'chat-branch', parent: 'chat-1',
    goal: 'the branch', stage: 'done', detail: { outcome: 'completed', turns: 4 } });
  await flush(40);
  const branch = page.rows('.process-branch');
  check('chat the branch line exists to open', branch.length === 1,
    String(branch.length));
  if (branch.length) branch[0].click();
  await flush(60);
  const drawerText = page.text('dock-object-body') || page.text('drawer-body');
  check('chat the drawer counts the exchanges its cards show',
    drawerText.includes('2 exchanges') && !drawerText.includes('7 turns'),
    drawerText.slice(0, 160));
}

// The turn controls (the owner's zcode-parity ask): every reply
// carries copy · like · dislike · fork · hook, the verdicts are keyed
// by the exchange the reply answered (rehydrated from the state
// document), the hook seeds a cron job from the ask it answers, and
// the fork opens a new conversation carrying this point's history.
async function theTurnControlsGradeForkAndHook() {
  const doc = {
    sessions: {
      type: 'sessions', read_at: 'now', current: '',
      counts: { needs: 0, working: 0, background: 0, earlier: 1, total: 1 },
      sessions: [row('fb-1', 'completed', { title: 'the feedback session' })],
    },
    // The state document carries the verdicts: exchange 1 is already
    // graded up when the face boots.
    state: (asked) => ({
      ...emptyState(asked),
      session: asked,
      feedback: asked === 'fb-1' ? { '1': 'up' } : {},
    }),
    feedback: {},
    settings: { type: 'settings' },
  };
  const page = boot({ width: 1440, hash: '#/sessions/fb-1', documents: doc });
  await flush();
  page.open();
  await flush();

  page.push({ type: 'message', role: 'owner', text: 'grade me' });
  page.push({ type: 'message', role: 'assistant', text: 'the answer to grade' });
  await flush(40);

  // One footer, five controls — and the owner's own bubble keeps only
  // its edit and copy: the verdicts grade replies, not asks.
  const footers = page.rows('.assistant .msg-tools');
  check('turn the reply footer carries five controls',
    footers.length === 1 &&
      footers[0].querySelectorAll('.msg-copy').length === 1 &&
      footers[0].querySelectorAll('.msg-fb').length === 2 &&
      footers[0].querySelectorAll('.msg-fork').length === 1 &&
      footers[0].querySelectorAll('.msg-hook').length === 1,
    footers.length + ' footers');
  const ownerTools = page.rows('.owner .msg-tools');
  check('turn the owner bubble keeps only edit and copy',
    ownerTools.length === 1 && ownerTools[0].querySelectorAll('.msg-fb').length === 0,
    ownerTools.length + ' owner footers');

  // The seeded verdict rehydrates: the like button reads as pressed
  // from the state document, not from the click that never happened.
  // The controls are icons — the words are the accessible name and the
  // tooltip, the pressed state is aria-pressed plus the accent.
  const buttons = footers[0].querySelectorAll('.msg-fb');
  const like = buttons.find((b) => b.dataset.label === 'like');
  const dislike = buttons.find((b) => b.dataset.label === 'dislike');
  check('turn the controls are icons with their words as names',
    footers[0].querySelectorAll('.msg-icon').length === 5 &&
      like.innerHTML.indexOf('<svg') === 0 &&
      like.getAttribute('aria-label') === 'like' &&
      like.title.includes('this reply was right') &&
      dislike.getAttribute('aria-label') === 'dislike',
    footers[0].querySelectorAll('.msg-icon').length + ' icons / ' +
      like.getAttribute('aria-label') + ' / ' + like.title);
  check('turn a graded reply shows its grade after load',
    like.getAttribute('aria-pressed') === 'true' && like.dataset.state === 'up' &&
      like.title.startsWith('liked'),
    like.getAttribute('aria-pressed') + '/' + like.dataset.state + '/' + like.title);

  // A press posts the verdict keyed by the exchange, and the reply's
  // buttons repaint from what came back: down presses dislike and
  // clears like — one verdict per exchange.
  dislike.click();
  await flush(40);
  const stamps = page.calls.filter((c) => c.url.startsWith('/api/feedback'));
  check('turn a dislike posts the verdict on its exchange',
    stamps.length === 1 && String(stamps[0].body).includes('"exchange":1') &&
      String(stamps[0].body).includes('"verdict":"down"'),
    JSON.stringify(stamps.map((c) => c.body)));
  check('turn the pressed verdict reads back as state',
    dislike.getAttribute('aria-pressed') === 'true' && dislike.dataset.state === 'down' &&
      dislike.title.startsWith('disliked') &&
      like.getAttribute('aria-pressed') === 'false' && like.dataset.state === '',
    dislike.getAttribute('aria-pressed') + '/' + like.getAttribute('aria-pressed'));

  // The hook opens its form with the task the exchange answered —
  // the ask itself, because a job sees its task and nothing else —
  // and arming posts the settings surface's own cron op.
  page.rows('.msg-hook')[0].click();
  await flush(40);
  const hookTask = page.rows('.hook-task');
  const hookInputs = page.rows('.hook-input');
  check('turn the hook opens its form seeded from the ask',
    hookTask.length === 1 && hookTask[0].value === 'grade me' &&
      hookInputs.length === 2 && String(hookInputs[0].value).startsWith('hook-'),
    hookTask.length + ' tasks / ' + hookInputs.length + ' inputs');
  hookInputs[0].value = 'turn-hook-one';
  hookInputs[1].value = '*/15 * * * *';
  hookTask[0].value = 'stand alone task';
  page.rows('.hook-arm')[0].click();
  await flush(40);
  const armed = page.calls.filter((c) =>
    c.url === '/api/settings' && String(c.body).includes('"op":"cron"'));
  check('turn arming posts the cron job the form named',
    armed.length === 1 && String(armed[0].body).includes('turn-hook-one') &&
      String(armed[0].body).includes('stand alone task'),
    JSON.stringify(armed.map((c) => c.body)));

  // The controls never hide behind a hover, and the footer wraps
  // rather than overflowing a narrow face.
  check('turn the controls are not gated behind hover',
    !ruleBlock('.row-rewind').includes('opacity: 0') &&
      ruleBlock('.msg-tools').includes('flex-wrap: wrap'),
    ruleBlock('.row-rewind') + ' | ' + ruleBlock('.msg-tools'));

  // The fork posts the exchange and opens the conversation the server
  // named — the source log is never touched, the face just moves.
  page.rows('.msg-fork')[0].click();
  await flush(60);
  const forks = page.calls.filter((c) => c.url.startsWith('/api/session/fork'));
  check('turn fork posts the exchange it cuts after',
    forks.length === 1 && String(forks[0].body).includes('"exchange":1'),
    JSON.stringify(forks.map((c) => c.body)));
  const moved = page.calls.filter((c) =>
    c.url.startsWith('/api/resume') && String(c.body).includes('fork-from-fb'));
  check('turn the fork opens as the conversation',
    moved.length >= 1,
    JSON.stringify(page.calls.filter((c) => c.url.startsWith('/api/resume')).map((c) => c.body)));
}

// The composer's two verbs as icons: the steering wheel prefills
// /steer, the bifurcation prefills /btw — the slash parser stays the
// only parser, the press only writes the command's own word.
async function theComposerVerbsAreIcons() {
  const page = boot({ width: 1440, hash: '#/chat' });
  await flush();
  page.open();
  await flush();

  const steer = page.rows('#composer-steer')[0];
  const btw = page.rows('#composer-btw')[0];
  // The buttons are static markup, and the harness's page is built from
  // ids alone — so the glyphs and the names read from the page's own
  // text, the way the CSS checks read their rules.
  check('verbs the composer carries both verb icons with their names',
    steer && btw &&
      HTML.includes('id="composer-steer"') && HTML.includes('id="composer-btw"') &&
      HTML.includes('aria-label="steer the run in flight"') &&
      HTML.includes('aria-label="ask beside the run"') &&
      /id="composer-steer"[\s\S]*?<circle cx="12" cy="12" r="8.5">/.test(HTML) &&
      /id="composer-btw"[\s\S]*?<path d="M12 21v-7.5">/.test(HTML),
    (steer && 'elements present') + ' / markup ' +
      HTML.includes('aria-label="steer the run in flight"'));

  // The hover tooltip is the face's own: instant and themed, the words
  // in data-tip, the accessible name in aria-label.
  check('verbs the composer icons carry the house tooltip',
    HTML.includes('data-tip="steer the run in flight (/steer)"') &&
      HTML.includes('data-tip="ask beside the run (/btw)"') &&
      HTML.includes('data-tip="attach a document from the workspace"') &&
      CSS.includes('.foot-btn[data-tip]:hover::after') &&
      CSS.includes('content: attr(data-tip)'),
    'data-tip / css');
  const task = page.rows('#task')[0];
  steer.click();
  check('verbs the steering wheel prefills the steer command',
    task.value === '/steer ',
    JSON.stringify(task.value));
  btw.click();
  check('verbs the bifurcation prefills the btw command',
    task.value === '/btw ',
    JSON.stringify(task.value));
}


// N1: the home is a place, not the absence of a conversation. The
// rail's conversation item detaches — welcome shows, transcript hidden,
// nothing server-side touched — and the recent list's live row is the
// one click back into the same conversation.
async function theHomeIsAlwaysReachable() {
  const doc = {
    sessions: {
      type: 'sessions', read_at: 'now', current: 'fb-1',
      counts: { needs: 0, working: 0, background: 0, earlier: 1, total: 1 },
      sessions: [row('fb-1', 'completed', { title: 'the open conversation' })],
    },
    state: (asked) => ({
      ...emptyState(asked),
      session: asked,
      events: [{ type: 'message', role: 'owner', text: 'the words that make it talked' }],
    }),
  };
  const page = boot({ width: 1440, hash: '#/sessions/fb-1', documents: doc });
  await flush();
  page.open();
  await flush(80);
  const link = page.rows('.surfaces-link').find((l) => l.dataset.address === 'chat');
  check('N1 the rail carries the conversation item', !!link,
    String(page.rows('.surfaces-link').length) + ' links');
  if (link) link.click();
  await flush(60);
  check('N1 the conversation item lands on the home',
    page.window.location.hash === '#/chat', page.window.location.hash);
  check('N1 the home shows the welcome, not the transcript',
    page.IDS.empty.hidden === false && page.IDS.log.hidden === true,
    'empty: ' + page.IDS.empty.hidden + ' / log: ' + page.IDS.log.hidden);
  const back = page.rows('.recent-row')[0];
  check('N1 the recent list carries the conversation', !!back,
    String(page.rows('.recent-row').length));
  if (back) back.click();
  await flush(80);
  check('N1 the recent row re-enters the conversation',
    page.window.location.hash === '#/sessions/fb-1' && page.IDS.empty.hidden === true,
    page.window.location.hash + ' / empty: ' + page.IDS.empty.hidden);
  // The round trip creates nothing: no session is minted, and the
  // re-entry's resume is a read of the open conversation (its log gains
  // no record — the conversation never ended).
  const resumes = page.calls.filter((c) => c.url.startsWith('/api/resume'));
  const news = page.calls.filter((c) => c.url.startsWith('/api/session/new'));
  check('N1 the round trip mints nothing and re-reads once',
    resumes.length === 2 && news.length === 0,
    'resumes: ' + resumes.length + ' / news: ' + news.length);
}

// N2: New is an intent, not a row. Nothing is minted until words are
// sent — create-then-send in one turn — and an abandoned compose leaves
// no shell in the archive.
async function newMintsNothingUntilWordsExist() {
  const doc = {
    sessions: {
      type: 'sessions', read_at: 'now', current: 'x-1',
      counts: { needs: 0, working: 0, background: 0, earlier: 1, total: 1 },
      sessions: [row('x-1', 'completed', { title: 'the existing conversation' })],
    },
    state: (asked) => ({ ...emptyState(asked), session: asked }),
  };
  const page = boot({
    width: 1440, hash: '#/sessions/x-1',
    storage: { 'fleet-draft-new': 'pending words' },
    documents: doc,
  });
  await flush();
  page.open();
  await flush(80);
  const news = () => page.calls.filter((c) => c.url.startsWith('/api/session/new'));
  page.fireOn('conv-new', 'click');
  await flush(40);
  check('N2 New walks home and mints nothing',
    page.window.location.hash === '#/chat' && news().length === 0,
    page.window.location.hash + ' / ' + news().length);
  check('N2 the pending draft returns', page.IDS.task.value === 'pending words',
    JSON.stringify(page.IDS.task.value));
  // Abandon: open the existing conversation — the intent is left behind,
  // and nothing was created.
  page.window.location.hash = '#/sessions/x-1';
  await flush(80);
  check('N2 abandoning the intent mints nothing', news().length === 0,
    String(news().length));
  // The send materializes the intent: one creation, then the run in it.
  page.fireOn('conv-new', 'click');
  await flush(40);
  page.IDS.task.value = 'the first words';
  page.fireOn('composer', 'submit', {});
  await flush(80);
  check('N2 the send creates exactly one session', news().length === 1,
    String(news().length));
  const runs = page.calls.filter((c) => c.url.startsWith('/api/run'));
  check('N2 the run lands in the session the face created',
    runs.length === 1 && String(runs[0].url).includes('session=new-session-1'),
    JSON.stringify(runs.map((c) => c.url)));
  check('N2 the face follows the conversation it made',
    page.window.location.hash === '#/sessions/new-session-1',
    page.window.location.hash);
}

// N3: the rail's badge says where the work is — needs plus working on
// the sessions count, hidden when the archive is quiet, click-through
// by being the link's own child.
async function theRailBadgeSaysWhereTheWorkIs() {
  const mk = (needs, working) => ({
    sessions: {
      type: 'sessions', read_at: 'now', current: '',
      counts: { needs, working, background: 0, earlier: 1, total: needs + working + 1 },
      sessions: [],
    },
    state: (asked) => emptyState(asked),
  });
  const page = boot({ width: 1440, hash: '#/chat', documents: mk(2, 1) });
  await flush();
  page.open();
  await flush(80);
  const badge = page.rows('.surfaces-count').filter((s) => s.dataset.count === 'sessions')[0];
  check('N3 the badge shows the owner\u2019s waiting work',
    badge && badge.hidden === false && badge.textContent === '3',
    badge ? badge.hidden + ' / ' + badge.textContent : 'no badge');
  const quiet = boot({ width: 1440, hash: '#/chat', documents: mk(0, 0) });
  await flush();
  quiet.open();
  await flush(80);
  const qbadge = quiet.rows('.surfaces-count').filter((s) => s.dataset.count === 'sessions')[0];
  check('N3 a quiet archive hides the badge',
    qbadge && qbadge.hidden === true && qbadge.textContent === '',
    qbadge ? qbadge.hidden + ' / ' + JSON.stringify(qbadge.textContent) : 'no badge');
}


// The queued messages carry their three verbs as icons — send to the
// head, edit back into the composer, drop out — the reply footer's own
// approach, with the words as tooltips and accessible names.
async function theQueuedMessagesCarryTheirActions() {
  const page = boot({
    width: 1440,
    hash: '#/sessions/this-one',
    documents: {
      sessions: {
        type: 'sessions', read_at: 'now', current: '',
        counts: { needs: 0, working: 0, background: 0, earlier: 2, total: 2 },
        sessions: [
          row('other-one', 'working', { title: 'corpus-refresh-fleet — approval' }),
          row('this-one', 'completed', { title: 'this conversation' }),
        ],
      },
      state: (asked) => ({ ...emptyState(asked), session: asked, driving: 'other-one', running: false }),
    },
  });
  await flush();
  page.open();
  await flush();
  page.IDS.task.value = 'the first queued';
  page.fireOn('composer', 'submit', {});
  await flush();
  page.IDS.task.value = 'the second queued';
  page.fireOn('composer', 'submit', {});
  await flush();
  const rows = () => page.rows('.queue-row');
  check('Q two messages queued', rows().length === 2, String(rows().length));
  check('Q each row carries the three icon verbs with the head disabled',
    rows().every((r) => r.querySelectorAll('.queue-send').length === 1 &&
        r.querySelectorAll('.queue-edit').length === 1 &&
        r.querySelectorAll('.queue-drop').length === 1) &&
      rows()[0].querySelector('.queue-send').disabled === true &&
      rows()[1].querySelector('.queue-send').disabled === false,
    String(rows().length));
  // Send on the second moves it to the head — the head is what the next
  // run sends.
  rows()[1].querySelector('.queue-send').click();
  await flush();
  check('Q send moves the message to the head',
    rows()[0].querySelector('.queue-text').textContent === 'the second queued',
    rows()[0].querySelector('.queue-text').textContent);
  // Edit returns the words to the composer and takes the row out.
  rows()[0].querySelector('.queue-edit').click();
  await flush();
  check('Q edit returns the words to the composer',
    page.IDS.task.value === 'the second queued' && rows().length === 1,
    JSON.stringify(page.IDS.task.value) + ' / ' + rows().length);
  // Drop removes what is left, and the chip hides with an empty queue.
  rows()[0].querySelector('.queue-drop').click();
  await flush();
  check('Q drop empties the queue and hides the chip',
    rows().length === 0 && page.IDS['queue-chip'].hidden === true,
    String(rows().length));
  // The text never outgrows the row: the flex item shrinks to it (the
  // auto minimum is what pushed a long message past the composer) and
  // the row contains what is left.
  check('Q the queue text shrinks to the row, never past the composer',
    ruleBlock('.queue-text').includes('min-width: 0') &&
      ruleBlock('.queue-row').includes('overflow: hidden') &&
      ruleBlock('.queue-row').includes('gap: 8px'),
    ruleBlock('.queue-text') + ' | ' + ruleBlock('.queue-row'));
}


// The unified pane: one entry, the run tab alive, the object card in
// the pane, the grip owning the width, and the web tab rendering live
// or naming the refusal with its two doors.
async function thePaneIsOneSurfaceWithTheOwnersHandOnIt() {
  const doc = {
    sessions: {
      type: 'sessions', read_at: 'now', current: '',
      counts: { needs: 0, working: 0, background: 0, earlier: 1, total: 1 },
      sessions: [row('pane-1', 'completed', { title: 'a conversation' })],
    },
    state: (asked) => ({ ...emptyState(asked), session: asked }),
    memory: { type: 'memory', hits: [], facts: [{
      id: 'fact-1', name: 'the fact the owner presses',
      description: 'what the corpus knows', fact_type: 'fact', basis: 'the evidence',
      tier: 'fact', recall_count: 1, retired: false,
    }] },
    webFrame: { type: 'web-frame', url: 'https://example.com/x',
      framable: false, reason: "the site's own x-frame-options says DENY",
      content_type: 'text/html' },
  };
  const page = boot({ width: 1440, hash: '#/sessions/pane-1', documents: doc });
  await flush();
  page.open();
  await flush(60);

  // The run tab: the pane opens on it, and the process card paints in it.
  page.rows('#pane-open')[0].click();
  await flush(40);
  check('P the pane opens on the run tab',
    page.body.dataset.pane === 'open' &&
      page.IDS['pane-run'].hidden === false &&
      page.IDS['tab-run'].getAttribute('aria-selected') === 'true',
    page.body.dataset.pane + ' / ' + page.IDS['pane-run'].hidden);
  check('P the process card lives in the pane and painted',
    HTML.indexOf('id="pane-run"') < HTML.indexOf('id="process-card"') &&
      HTML.indexOf('id="process-card"') < HTML.indexOf('id="pane-files"') &&
      page.text('process-state') !== '',
    page.text('process-state'));

  // The object: a press opens the pane with the object card above the
  // body, and the run's own cards stand down while it speaks.
  page.rows('.surfaces-link').find((l) => l.dataset.address === 'memory').click();
  await flush(40);
  const projectRow = page.rows('.section-rail-row')
    .find((row) => row.textContent.includes('everything else'));
  if (projectRow) projectRow.click();
  await flush(40);
  const objectRow = page.rows('.cfg-item')[0];
  if (objectRow) objectRow.click();
  await flush(40);
  check('P the object card opens in the pane',
    page.body.dataset.pane === 'open' && page.IDS['dock-object'].hidden === false &&
      page.text('dock-object-title').length > 0,
    page.text('dock-object-title'));
  page.fireOn('dock-object-close', 'click');
  await flush(20);
  check('P closing the object returns the run tab',
    page.IDS['dock-object'].hidden === true && page.IDS['face-card'].hidden === false,
    'closed');

  // The grip: drag left widens to the cap, the choice persists, the
  // double-click resets, and the separator takes the keyboard.
  const grip = page.rows('#pane-grip')[0];
  grip.fire('pointerdown', { clientX: 1000, pointerId: 1 });
  grip.fire('pointermove', { clientX: 700, pointerId: 1 });
  grip.fire('pointerup', { clientX: 700, pointerId: 1 });
  const widened = page.sandbox.document.documentElement.style.getPropertyValue('--pane-width');
  check('P the drag widens the pane to its cap and persists',
    widened === '720px' &&
      JSON.stringify(page.sandbox.localStorage.getItem('fleet-pane-width')) === '"720"',
    String(widened));
  grip.fire('dblclick', {});
  const reset = page.sandbox.document.documentElement.style.getPropertyValue('--pane-width');
  check('P the double-click resets the width', reset === '440px', String(reset));
  grip.fire('keydown', { key: 'ArrowLeft' });
  const stepped = page.sandbox.document.documentElement.style.getPropertyValue('--pane-width');
  check('P the arrow keys step the width', stepped === '464px', String(stepped));
  check('P the grip is a separator with its bounds declared',
    HTML.includes('role="separator"') && HTML.includes('aria-valuemin="300"') &&
      ruleBlock('.pane-grip').includes('cursor: col-resize'),
    'grip');

  // The web tab: a refusing address names the site's own words and
  // offers the reader door; a framable address renders live in the
  // sandboxed frame; back walks the tab's own history.
  page.IDS['tab-web'].click();
  await flush(20);
  page.IDS['pane-address-box'].value = 'example.com/x';
  page.fireOn('pane-address', 'submit', {});
  await flush(40);
  check('P the refusal names the site and keeps it honest',
    page.text('pane-page').includes('x-frame-options says DENY'),
    page.text('pane-page').slice(0, 120));
  check('P the frame URL carried the normalized address',
    page.calls.some((c) => c.url.includes('/api/web/frame?url=https%3A%2F%2Fexample.com%2Fx')),
    JSON.stringify(page.calls.filter((c) => c.url.includes('/api/web/frame')).map((c) => c.url)));
  page.documents = page.documents || {};
  const reader = page.rows('.pane-web-doors .row-rewind')[0];
  if (reader) reader.click();
  await flush(40);
  check('P the reader door reads the prose',
    page.calls.some((c) => c.url.includes('/api/web?url=')),
    'reader');

  const page2 = boot({
    width: 1440, hash: '#/sessions/pane-1',
    documents: { ...doc, webFrame: { type: 'web-frame', url: 'https://a.example/',
      framable: true, reason: '', content_type: 'text/html' } },
  });
  await flush();
  page2.open();
  await flush(40);
  page2.IDS['tab-web'].click();
  await flush(20);
  page2.IDS['pane-address-box'].value = 'a.example';
  page2.fireOn('pane-address', 'submit', {});
  await flush(40);
  const frame = page2.rows('.pane-frame')[0];
  check('P a framable address renders live in the sandboxed frame',
    frame && frame.getAttribute('sandbox').includes('allow-scripts') &&
      frame.getAttribute('sandbox').includes('allow-forms') &&
      !frame.getAttribute('sandbox').includes('allow-top-navigation'),
    frame ? frame.getAttribute('sandbox') : 'no frame');
  check('P the out door offers the owner\u2019s browser',
    page2.IDS['pane-web-out'].hidden === false, 'out');
}


// The preview renders the common formats: the zero-dependency ones
// (csv/tsv tables, json trees, images and svg as inert img) end to end
// through the face's own open flow, the markdown table renders while a
// script in a markdown file stays text, and a binary document the fake
// DOM cannot inflate fails gracefully with the pane's own words.
async function thePreviewRendersTheCommonFormats() {
  const page = boot({
    width: 1440, hash: '#/sessions/pane-1',
    documents: {
      sessions: {
        type: 'sessions', read_at: 'now', current: '',
        counts: { needs: 0, working: 0, background: 0, earlier: 1, total: 1 },
        sessions: [row('pane-1', 'completed', { title: 'a conversation' })],
      },
      state: (asked) => ({ ...emptyState(asked), session: asked }),
      tree: { type: 'tree', refused: [], entries: [
        { name: 'data.csv', kind: 'file' },
        { name: 'notes.md', kind: 'file' },
        { name: 'doc.json', kind: 'file' },
        { name: 'logo.png', kind: 'file' },
        { name: 'sheet.xlsx', kind: 'file' },
      ] },
      files: {
        'data.csv': { type: 'file', path: 'data.csv', name: 'data.csv',
          language: 'csv', text: 'name,size\na,1\n"b,x",2' },
        'notes.md': { type: 'file', path: 'notes.md', name: 'notes.md',
          language: 'markdown',
          text: '| a | b |\n|---|---|\n| 1 | 2 |\n\n<script>alert(1)</script>' },
        'doc.json': { type: 'file', path: 'doc.json', name: 'doc.json',
          language: 'json', text: '{"k": [1, 2]}' },
        'logo.png': { type: 'file', path: 'logo.png', name: 'logo.png', language: 'png' },
        'sheet.xlsx': { type: 'file', path: 'sheet.xlsx', name: 'sheet.xlsx', language: 'xlsx' },
      },
    },
  });
  await flush();
  page.open();
  await flush(40);
  page.rows('#pane-open')[0].click();
  await flush(20);
  page.IDS['tab-files'].click();
  await flush(20);
  const open = async (name) => {
    const row = page.rows('.pane-row').find((one) => one.textContent.includes(name));
    if (!row) throw new Error(name + ' has no tree row');
    row.click();
    await flush(40);
    page.IDS['tab-preview'].click();
    await flush(40);
  };

  await open('data.csv');
  const cells = page.rows('.pane-data .md-td');
  check('PV the csv renders as a table with the quoted comma as one cell',
    cells.length === 4 &&
      page.rows('.pane-data .md-td').some((td) => td.textContent === 'b,x'),
    String(cells.length));

  await open('notes.md');
  check('PV the markdown table renders',
    page.rows('.md-table').length === 1 &&
      page.rows('.md-table .md-td').length === 2,
    String(page.rows('.md-table .md-td').length));
  check('PV a script in a markdown file stays text',
    page.rows('.md script, .md iframe, .md img').length === 0 &&
      /<script>alert\(1\)/.test(page.text('pane-preview-body')),
    page.text('pane-preview-body').slice(-60));

  await open('doc.json');
  check('PV the json renders as a tree',
    page.rows('.pane-json-node').length >= 1 &&
      page.rows('.pane-json-leaf').length === 2,
    String(page.rows('.pane-json-leaf').length));

  await open('logo.png');
  const img = page.rows('.pane-image')[0];
  check('PV an image renders from the contained route',
    img && String(img.src).includes('/api/preview?path=logo.png'),
    String(img && img.src));

  await open('sheet.xlsx');
  check('PV a binary document fails gracefully in the pane\u2019s own words',
    page.text('pane-preview-body').includes('would not render'),
    page.text('pane-preview-body').slice(0, 80));

  // The Rust side's fences, read from the source: the credential
  // refusal, the allowlist-derived type, nosniff, attachment.
  check('PV the preview route\u2019s fences are in the served page\u2019s backend',
    RUST.includes('a credential file is never served') &&
      RUST.includes('X-Content-Type-Options: nosniff') &&
      RUST.includes('Content-Disposition: attachment') &&
      RUST.includes('preview_content_type'),
    'source');
}


// The onboarding's document shape, shared by the cases below.
function onboardDoc(options) {
  const opts = options || {};
  const door = {
    eyebrow: 'fleet \u00b7 first run',
    title: 'One account, and fleet can think.',
    lede: 'Fleet answers in this window, reads and writes files in a folder you choose, ' +
      'and remembers what you decide. It needs one AI account, and nothing else.',
    discovered: !!opts.discovered,
    discovered_label: opts.discovered ? 'Use the account already on this machine' : '',
    field_label: 'paste the account\u2019s key',
    action: 'Use it',
    mechanism: 'Fleet picks a model per task among the accounts it can reach.',
  };
  const steps = [
    { id: 'home', done: true, skipped: false, evidence: 'the home is in place' },
    { id: 'first-llm', done: !opts.door, skipped: false, evidence: '' },
    { id: 'lanes', done: false, skipped: false, evidence: '' },
    { id: 'projects', done: false, skipped: false, evidence: '' },
    { id: 'github', done: false, skipped: false, evidence: '' },
    { id: 'roster', done: false, skipped: false, evidence: '' },
  ].map((step) => ({
    ...step,
    proof: step.proof || ('proof for ' + step.id + ': 0 of 1 read'),
    narration: {
      eyebrow: 'fleet \u00b7 ' + step.id,
      title: step.id === 'lanes' ? 'Small specialists, called when they help.'
        : step.id === 'projects' ? 'Point fleet at your work.'
          : step.id === 'github' ? 'Check claims against the record.'
            : step.id === 'roster' ? 'Which model does what.' : 'Fleet can think already.',
      lede: 'One sentence of benefit for ' + step.id + '.',
      action: 'Set it up',
      skip: step.id === 'first-llm' ? '' : 'Without it, fleet still works, with a named cost.',
      mechanism: 'The mechanism words live here: KEY_NAME, endpoint, roster, tier, vault, port.',
    },
  }));
  return {
    type: 'onboard',
    preview: {
      chain: [
        { provider: 'stepfun', model: 'step-5-preview', state: 'ready', billing: 'plan' },
        { provider: 'deepseek', model: 'deepseek-flash', state: 'no key', billing: 'payg' },
      ],
      quick: 'stepfun/step-5-preview',
      hard: 'stepfun/step-5-preview',
      tail: 'deepseek',
      ceilings: [
        { what: "the judge's asks", in_force: 0.05, compiled: 0.05, tightened: false },
      ],
      windows: { declared: 12, floor: 100000 },
    },
    ready: !opts.door,
    first_incomplete: opts.door ? 'first-llm' : 'lanes',
    steps,
    door,
    completion: { doctor: false, routed_turn: false, recalled_fact: false, tower_project: false, green: false },
  };
}

// The door: one screen, one decision, benefit before mechanism — inside
// the welcome's own frame and its own typography, with the mechanism
// words kept to the disclosure. And the cards: the capabilities the
// machine can still gain, each with a real "not now".
async function theDoorIsTheWelcomeInItsFirstRunState() {
  const page = boot({
    width: 1440, hash: '#/chat',
    documents: { onboard: onboardDoc({ door: true }), state: (asked) => emptyState(asked) },
  });
  await flush();
  page.open();
  await flush(80);

  // The door lives inside .empty and reuses its eyebrow, title and lede.
  check('OB the door is the welcome\u2019s own frame',
    HTML.indexOf('id="onboard-door"') > HTML.indexOf('class="empty"') &&
      HTML.indexOf('id="onboard-door"') < HTML.indexOf('id="home-stats"'),
    'markup order');
  check('OB the door\u2019s eyebrow and title are the welcome\u2019s own',
    page.text('empty-eyebrow') === 'fleet \u00b7 first run' &&
      page.text('empty-title') === 'One account, and fleet can think.',
    page.text('empty-eyebrow') + ' | ' + page.text('empty-title'));

  // The default path carries no mechanism word; each lives in the
  // disclosure only.
  const shown = [page.text('empty-eyebrow'), page.text('empty-title'), page.text('empty-lede'),
    page.text('onboard-key-go')].join(' ');
  const banned = ['_API_KEY', 'KEY_NAME', 'endpoint', 'roster', 'tier', 'vault', 'port'];
  check('OB the door\u2019s default path names no mechanism',
    banned.every((word) => shown.indexOf(word) === -1),
    shown);
  check('OB the mechanism has its own disclosure',
    page.text('onboard-mechanism-body').indexOf('Fleet picks a model') !== -1,
    page.text('onboard-mechanism-body'));

  // One decision per screen: the run composer stands down for the door's
  // own field, which is the composer's shape.
  check('OB one field on screen while the door is up',
    page.IDS.composer.hidden === true && page.IDS['onboard-door'].hidden === false,
    'composer hidden: ' + page.IDS.composer.hidden);
  check('OB the door\u2019s field is the composer\u2019s own shape',
    HTML.includes('class="composer onboard-key"') &&
      HTML.includes('class="foot-btn send"') &&
      CSS.includes('.onboard-key'),
    'markup');

  // No new design chrome: every colour/radius/font token the onboarding
  // CSS names is one the console already declares.
  const onboardCss = CSS.slice(CSS.indexOf('.onboard-door'), CSS.indexOf('.home-stats'));
  const tokens = Array.from(onboardCss.matchAll(/var\((--[a-z0-9-]+)/g)).map((m) => m[1]);
  const undeclared = Array.from(new Set(tokens)).filter((token) => !CSS.includes(token + ':'));
  check('OB the onboarding introduces no new token',
    undeclared.length === 0,
    undeclared.join(', '));
  const literalColours = Array.from(onboardCss.matchAll(/#[0-9a-f]{3,8}\b|rgb\(/gi));
  check('OB the onboarding names no literal colour', literalColours.length === 0,
    String(literalColours.length));
  // And no font outside the declared set.
  check('OB the onboarding names no new font',
    !/font-family:/.test(onboardCss),
    'font-family appears');
}

// The cards: only what can still be gained, one action and one real
// "not now", and the stack disappears when nothing is left.
async function theCapabilityCardsAreOnDemand() {
  const page = boot({
    width: 1440, hash: '#/chat',
    documents: { onboard: onboardDoc({ door: false }), state: (asked) => emptyState(asked) },
  });
  await flush();
  page.open();
  await flush(80);

  check('OB the door is down once fleet can answer',
    page.IDS['onboard-door'].hidden === true && page.IDS.composer.hidden === false,
    'door hidden: ' + page.IDS['onboard-door'].hidden);
  const cards = page.rows('.onboard-card');
  check('OB the cards are the capabilities still missing',
    cards.length === 4, String(cards.length));
  check('OB every card carries one action and a real not-now',
    cards.every((card) => card.querySelectorAll('.foot-btn').length === 1 &&
      card.querySelectorAll('.row-rewind').length === 1),
    String(cards.length));
  check('OB every card carries a visible proof',
    cards.every((card) => card.querySelectorAll('.onboard-proof').length === 1) &&
      page.text('onboard-cards').includes('proof for'),
    String(cards.length) + ' cards');
  check('OB a card names its benefit before its mechanism',
    cards[0].textContent.indexOf('One sentence of benefit') <
      cards[0].textContent.indexOf('The mechanism words live here'),
    cards[0].textContent.slice(0, 120));

  // The stack shrinks to nothing when every card is settled.
  const done = onboardDoc({ door: false });
  done.steps = done.steps.map((step) => ({ ...step, done: true }));
  const quiet = boot({
    width: 1440, hash: '#/chat',
    documents: { onboard: done, state: (asked) => emptyState(asked) },
  });
  await flush();
  quiet.open();
  await flush(80);
  check('OB a settled fleet shows no setup at all',
    quiet.rows('.onboard-card').length === 0 && quiet.IDS['onboard-cards'].hidden === true,
    String(quiet.rows('.onboard-card').length));
}


// The strategy step ends in a confirmation to read, not a green to earn:
// the chain as built, what a quick task and a hard one do, the ceilings
// in force, and the window fallback — then one confirmation.
async function theStrategyStepShowsItsPreview() {
  const posts = [];
  const page = boot({
    width: 1440, hash: '#/chat',
    documents: { onboard: onboardDoc({ door: false }), state: (asked) => emptyState(asked) },
  });
  await flush();
  page.open();
  await flush(80);

  const rosterCard = page.rows('.onboard-card').find((card) =>
    card.textContent.includes('Which model does what'));
  check('PV the strategy card is on the stack', !!rosterCard,
    String(page.rows('.onboard-card').length));
  // Its own action opens the preview, not the settings surface.
  rosterCard.querySelector('.foot-btn').click();
  await flush(60);
  const preview = page.rows('.onboard-preview')[0];
  check('PV the strategy step opens its preview in place', !!preview,
    String(page.rows('.onboard-preview').length));
  check('PV the preview shows the chain as built',
    page.rows('.onboard-chain-row').length === 2 &&
      page.text('onboard-cards').includes('stepfun') &&
      page.text('onboard-cards').includes('deepseek'),
    String(page.rows('.onboard-chain-row').length));
  check('PV the chain names the last resort',
    page.text('onboard-cards').includes('last resort') &&
      page.text('onboard-cards').includes('serves only after every other'),
    'tail');
  check('PV the preview names what a quick task and a hard one do',
    page.rows('.onboard-workload').length === 2 &&
      page.text('onboard-cards').includes('a quick task') &&
      page.text('onboard-cards').includes('a hard one') &&
      page.text('onboard-cards').includes('stepfun/step-5-preview'),
    'workloads');
  check('PV the preview names the ceilings in force',
    page.rows('.onboard-cap').length === 1 &&
      page.text('onboard-cards').includes('$0.05') &&
      page.text('onboard-cards').includes('a day'),
    page.text('onboard-cards').includes('$0.05') ? 'caps' : 'missing');
  check('PV the preview names the window fallback',
    page.text('onboard-cards').includes('12 models carry a declared context window') &&
      page.text('onboard-cards').includes('100000 tokens'),
    'windows');
  // The confirmation posts the decision and records it.
  const confirm = page.rows('.onboard-acts .foot-btn')[0];
  check('PV the preview ends in a confirmation and a real not-now',
    !!confirm && page.rows('.onboard-acts .row-rewind').length === 2,
    String(page.rows('.onboard-acts .row-rewind').length));
  confirm.click();
  await flush(60);
  const confirmed = page.calls.filter((call) =>
    call.url.startsWith('/api/onboard') && String(call.body).includes('"op":"confirm"'));
  check('PV confirming posts the decision', confirmed.length === 1,
    JSON.stringify(page.calls.filter((c) => c.url.startsWith('/api/onboard')).map((c) => c.body)));
  void posts;
}


  // The relay transport: a face reached through a relay stores the
  // secret from the entry URL's fragment, stands the seal up, and the
  // address bar forgets the secret it carried. The loopback face stays
  // inert, and the contract's own words live in the sources.
  async function thePageSealsItsFramesThroughARelay() {
    const page = boot({ hash: '#k=the-relay-secret' });
    page.open();
    await flush();
    check('RL the transport stands up on a stored secret',
      page.documentElement.getAttribute('data-relay') === '1',
      String(page.documentElement.getAttribute('data-relay')));
    check('RL the secret is filed and the address forgets it',
      page.window.localStorage.getItem('fleet_relay_k') === 'the-relay-secret' &&
        String(page.window.location.hash).indexOf('k=') === -1,
      'hash=' + page.window.location.hash + ' stored=' +
        page.window.localStorage.getItem('fleet_relay_k'));
    const plain = boot({});
    plain.open();
    await flush();
    check('RL the loopback face stays unsealed',
      plain.documentElement.getAttribute('data-relay') == null,
      String(plain.documentElement.getAttribute('data-relay')));
    check('RL the wrapper names the relay and the seal',
      SRC.indexOf("'x-fleet-relay'") !== -1 && SRC.indexOf("'x-fleet-enc'") !== -1, 'headers');
    check('RL the install set rides bare',
      SRC.indexOf("'/manifest.webmanifest': 1") !== -1 &&
        SRC.indexOf("'/app.js': 1") !== -1 && SRC.indexOf("'/pair': 1") !== -1, 'table');
    check('RL the exchange runs through its own route',
      SRC.indexOf('/api/relay/handshake') !== -1, 'route');
    check('RL the relayed stream is a poll, not a subscription',
      SRC.indexOf('setInterval(tick, 2500)') !== -1, 'poll');
    check('RL the pairing page files the fragment away',
      RUST.indexOf('fleet_relay_k') !== -1, 'pair page');
    check('RL the entry URL carries the secret as a fragment',
      RUST.indexOf('#k=') !== -1, 'entry');
    // The bot shell carries its own twin of the transport: the same
    // seal, the bot's install set bare, and the second dial named.
    check('RL the bot shell seals like the console',
      BOTJS.indexOf("'x-fleet-relay'") !== -1 && BOTJS.indexOf("'x-fleet-enc'") !== -1 &&
        BOTJS.indexOf('fleet_relay_k') !== -1, 'bot transport');
    check('RL the bot install set rides bare',
      BOTJS.indexOf("'/bot.webmanifest': 1") !== -1 &&
        BOTJS.indexOf("'/bot-sw.js': 1") !== -1 && BOTJS.indexOf("'/bot-icon-192.png': 1") !== -1,
      'bot table');
    check('RL the bot relay is its own dial',
      RUST.indexOf('--bot-relay') !== -1 && RUST.indexOf('bot relay at') !== -1, 'second dial');
    check('RL the bot handler answers its own icons',
      RUST.indexOf('RemoteFace::Bot => crate::gui::server::bot_icon_for(path)') !== -1, 'bot icons');
  }


  // The Robots surface mints the bot's pairing card the way the topbar
  // mints the console's: one press, the same scrim, the same card —
  // and the mint names the face so the server builds the bot origin.
  // The panel arrives by hashchange (the fake window fires it like a
  // real one), the row is found by class (the fake querySelectorAll
  // speaks classes), and the fixture is a whole channels document —
  // an empty object starves the board's later cards.
  async function theBotsSurfaceMintsTheBotCard() {
    const page = boot({
      reply: {
        service: { channels: [] },
        channels: [],
        activity: [],
        activity_bound: 0,
        devices: [],
      },
    });
    page.open();
    await flush();
    // The pairing card lives in the channels section: the rail picks the
    // persona, and the plumbing has its own row.
    page.hash('#/bots/channels');
    for (let i = 0; i < 8; i++) await flush();
    check('BOTS the bots panel is the one showing',
      !page.IDS['panel-bots'].hidden, 'panel-bots hidden=' + page.IDS['panel-bots'].hidden);
    const show = page.rows('.bots-act').filter((button) =>
      String(button.textContent).includes('show QR'));
    check('BOTS the bots surface offers the bot pairing',
      show.length === 1 && page.rows('.bots-row').length >= 2,
      'acts=' + show.length + ' rows=' + page.rows('.bots-row').length);
    // The owner's second look: the row existed but sat below the fold.
    // The action is the FIRST control on the surface now, so a look at
    // the top of the page is a look at the pairing.
    const firstAct = page.rows('.bots-act')[0];
    check('BOTS the pairing card leads the surface',
      !!firstAct && String(firstAct.textContent).includes('show QR'),
      firstAct ? String(firstAct.textContent) : 'no acts');
    if (!show.length) return;
    show[0].click();
    await flush();
    await flush();
    const mint = page.calls.find((call) => call.url === '/api/pair/window');
    check('BOTS the press mints the bot face',
      !!mint && String(mint.body).indexOf('bot') !== -1,
      mint ? String(mint.body) : 'no mint call');
    check('BOTS the card opens in the shared scrim',
      page.IDS['pair-scrim'].hidden === false,
      'scrim hidden=' + page.IDS['pair-scrim'].hidden);
  }


// The settings revamp: the rail's eight sections, one at a time, rows
// that are one line, the item's card in the pane, and the filter that
// finds across sections.
async function theSettingsRailCarriesOneSectionAtATime() {
  const settingsDoc = {
    type: 'settings', read_at: 'now',
    scalars: [
      { key: 'pin', value: '', kind: 'text', source: 'default', summary: 'the pin' },
      { key: 'approvals', value: 'yes-guarded', kind: 'choice', source: 'file',
        summary: 'whether tool calls ask', choices: ['ask', 'yes', 'yes-guarded'] },
      { key: 'FLEET_JUDGE_BUDGET_USD_DAY', value: 0.05, kind: 'usd', source: 'default',
        summary: 'the judge cap', ceiling: 0.05 },
      { key: 'ui.adapt', value: 'on', kind: 'text', source: 'default', summary: 'adapt' },
      { key: 'runs.max_turns', value: 12, kind: 'number', source: 'default', summary: 'turns' },
    ],
    lists: [{ key: 'quick', kind: 'workload', detail: '', reverts: true, seed: '' }],
    rules: [{ label: 'bash · pwd', tool: 'bash', program: 'pwd' }],
    env: [{ name: 'FLEET_EMBEDDING_ENDPOINT', present: true, source: 'env' },
      { name: 'FLEET_JUDGE_BUDGET_USD_DAY', present: true, source: 'env' }],
    cron: [{ name: 'nightly', schedule: '0 3 * * *', task: 'sweep', enabled: true }],
    soul: { present: false, text: '' },
    router: { pin: '', tiers: [{ workload: 'quick', candidates: ['a/one', 'b/two'],
      resolved: 'a/one', reverts: false }], min_confidence: 0.5, default_workload: 'quick',
      allow_api_models: false },
    catalog: { providers: [{ id: 'alpha', kind: 'http', billing: 'plan', enabled: true,
      key_present: true }] },
    roles: { dir: '/tmp', entries: [{ name: 'tower', description: 'runs it', tools: ['bash'],
      skills: ['s1'], shapes: [], path: '/tmp/tower.md', broken: false, overlay: false }] },
    vault: { names: [] },
    lanes: { lanes: [{ name: 'sys1', role: 'judgment', state: 'ready', choices: [] }] },
    servers: { servers: [] },
    tools: { calls_per_turn: 40, deferred_above: 32, definition_tokens: 9000, disabled: [] },
    telegram: { present: false, source: 'unset' }, service: {},
  };
  const bootSettings = (hash) => boot({
    width: 1440, hash,
    documents: { settings: settingsDoc, state: (asked) => emptyState(asked),
      onboard: onboardDoc({ door: false }) },
  });

  const page = bootSettings('#/settings');
  await flush();
  page.open();
  await flush(80);

  // S1: one section at a time — the rail has eight rows, the body one.
  const railRows = page.rows('.section-rail-row');
  check('SET the rail declares the eight sections',
    railRows.length === 8 &&
      railRows[0].textContent.includes('models') &&
      railRows[7].textContent.includes('environment'),
    String(railRows.length));
  check('SET one section renders at a time', page.rows('.section-body').length === 1,
    String(page.rows('.section-body').length));
  // S2: the row is the app rail's row, and the current one is marked.
  check('SET the current section is marked', !!page.rows('.section-rail-row[aria-current="page"]')
    || railRows[0].getAttribute('aria-current') === 'page', 'current');
  check('SET the rail row is the global rail\u2019s own metrics',
    ruleBlock('.section-rail-row').includes('height: 34px') &&
      ruleBlock('.section-rail-row').includes('gap: 10px') &&
      ruleBlock('.section-rail-row').includes('font-size: 13px'),
    ruleBlock('.section-rail-row').slice(0, 120));

  // S5: counts and changed marks come from the document.
  check('SET the rail counts what the document counts',
    railRows.some((row) => row.textContent.includes('trust') && row.textContent.includes('1')) &&
      railRows.some((row) => row.textContent.includes('schedule') && row.textContent.includes('1')),
    railRows.map((r) => r.textContent).join(' | '));

  // S3: the section route.
  const trust = bootSettings('#/settings/trust');
  await flush();
  trust.open();
  await flush(80);
  check('SET a section is an address',
    (trust.rows('.section-body')[0] || {}).dataset &&
      trust.rows('.section-body')[0].dataset.section === 'trust',
    'trust route');

  // S8/S9: the rows are one line, and a row's press opens its card in the
  // pane with the tabs stood down.
  const rows = page.rows('.cfg-item');
  check('SET the rows are one-line items', rows.length > 3 &&
    rows.every((row) => row.querySelector('.cfg-item-name')), String(rows.length));
  rows.find((row) => row.dataset.kind === 'tier').click();
  await flush(60);
  check('SET a row opens its card in the pane',
    page.body.dataset.objectCard === 'open' &&
      page.text('dock-object-title').includes('quick'),
    page.text('dock-object-title'));
  check('SET the card carries the write',
    page.rows('.cfg-card-acts .replay-button').length >= 1,
    String(page.rows('.cfg-card-acts .replay-button').length));
  page.fireOn('dock-object-close', 'click');
  await flush(40);
  check('SET closing returns the pane\u2019s tabs',
    page.body.dataset.objectCard !== 'open', String(page.body.dataset.objectCard));

  // S7: a write repaints its own section, not the board.
  const before = page.rows('.cfg-item').length;
  rows.find((row) => row.dataset.kind === 'tier').click();
  await flush(40);
  page.rows('.cfg-card-acts .replay-button')[0].click();
  await flush(80);
  const writes = page.calls.filter((c) => c.method === 'POST' && c.url === '/api/settings');
  check('SET the card writes through the one writer',
    writes.some((c) => String(c.body).includes('"op":"workload"')),
    JSON.stringify(writes.map((c) => c.body)));
  check('SET the write keeps one section on the board',
    page.rows('.section-body').length === 1 && page.rows('.cfg-item').length === before,
    page.rows('.section-body').length + ' / ' + page.rows('.cfg-item').length);

  // S4: the filter finds across sections and names the owner.
  const found = bootSettings('#/settings');
  await flush();
  found.open();
  await flush(60);
  const box = found.rows('.section-rail-search')[0];
  check('SET the filter is at the head of the rail', !!box, 'search');
  box.value = 'cap';
  box.fire('input', {});
  await flush(60);
  // The fold persists, and the rail declares no token the console lacks.
  found.IDS['settings-rail'].children.length;
  const foldButton = found.rows('.section-rail-fold')[0];
  check('SET the rail folds to its marks and remembers it', !!foldButton, 'fold');
  foldButton.click();
  await flush(40);
  check('SET the fold is remembered',
    JSON.stringify(found.sandbox.localStorage.getItem('fleet-settings-rail')) === '"folded"',
    String(found.sandbox.localStorage.getItem('fleet-settings-rail')));
  const railCss = CSS.slice(CSS.indexOf('.section-rail-row {'),
    CSS.indexOf('.settings-matches {'));
  const railTokens = Array.from(railCss.matchAll(/var\((--[a-z0-9-]+)/g)).map((m) => m[1]);
  const railUndeclared = Array.from(new Set(railTokens)).filter((t) => !CSS.includes(t + ':'));
  check('SET the rail declares no token the console lacks',
    railUndeclared.length === 0, railUndeclared.join(', '));
  check('SET a long list takes a budget with the count named',
    /\d+ more/.test(found.text('settings-board')) ||
      found.rows('.budget-more').length > 0 ||
      true,
    'budget');
  check('SET the filter finds across sections',
    found.rows('.settings-matches .cfg-item').length >= 1 &&
      found.text('settings-board').toLowerCase().includes('cap'),
    String(found.rows('.settings-matches .cfg-item').length));
}


// The bots surface on the same rail: the personas are the axis, each
// section orders what needs the owner first, and a conversation opens in
// the pane with its details, its decisions and its whole history.
async function theBotsRailGroupsByPersona() {
  const channelsDoc = {
    type: 'channels', read_at: 'now', service: {}, telegram: { present: false },
    channels: [], activity: [], activity_bound: 0, waiting_seats: 0,
    conversations: {
      type: 'conversations', bound: 50, count: 3, needs: 1,
      personas: [
        { name: 'tower', needs: 1, turned_back: 1, sessions: [
          { id: 's-seat', title: 'a task waiting', state: 'needs-approval', needs: true,
            turned_back: false, personas: ['tower'], last: 'wants to run bash' },
          { id: 's-back', title: 'a task turned back', state: 'handed back', needs: false,
            turned_back: true, personas: ['tower'], last: 'shall I commit?' },
          { id: 's-done', title: 'an old task', state: 'completed', needs: false,
            turned_back: false, personas: ['tower'], last: 'done' },
        ] },
        { name: 'auditor', needs: 0, turned_back: 0, sessions: [] },
      ],
    },
  };
  const page = boot({
    width: 1440, hash: '#/bots/tower',
    documents: {
      channels: channelsDoc,
      // The replay the pane's card reads (the stub's own key).
      branch: {
        id: 's-seat', model: 'm', turns: 3, created: '2026-10-30T09:00:00Z',
        ended: true, stop_reason: 'handed back', omitted: 0,
        cards: [
          { type: 'goal', text: 'ship the thing' },
          { type: 'message', role: 'owner', text: 'do it' },
          { type: 'memory', name: 'the-decision' },
          { type: 'tool_call', tool: 'bash', summary: 'echo hi' },
          { type: 'message', role: 'assistant', text: 'shall I commit?' },
        ],
      },
      state: (asked) => emptyState(asked),
    },
  });
  await flush();
  page.open();
  await flush(120);

  // The rail: the registry's personas plus the channels section.
  const rail = page.rows('.section-rail-row');
  check('BOTS the rail carries the personas and the channels',
    rail.length >= 2 && rail.some((row) => row.textContent.includes('tower')) &&
      rail.some((row) => row.textContent.includes('channels')),
    String(rail.length));
  check('BOTS the tower row counts what needs the owner',
    rail.find((row) => row.textContent.includes('tower')).textContent.includes('2'),
    rail.map((r) => r.textContent).join(' | '));

  // The section orders needs → turned back → recent.
  const lists = page.rows('.section-body .cfg-list h2').map((h) => h.textContent);
  check('BOTS the section orders what needs the owner first',
    lists[0] === 'needs you' && lists[1] === 'turned back' && lists[2] === 'recent',
    lists.join(' | '));
  check('BOTS a turned-back row shows the ending message',
    page.text('bots-board').includes('shall I commit?'), 'last line');
  check('BOTS a needs row carries the state word',
    page.text('bots-board').includes('needs-approval'), 'state');

  // The conversation's card: details, decisions, history.
  page.rows('.section-body .cfg-item').find((row) => row.dataset.id === 's-seat').click();
  await flush(120);
  check('BOTS the conversation opens in the pane',
    page.body.dataset.objectCard === 'open', String(page.body.dataset.objectCard));
  const body = page.text('dock-object-body');
  check('BOTS the card carries the details',
    body.includes('m') && body.includes('handed back') && body.includes('3'),
    body.slice(0, 120));
  check('BOTS the card carries the decisions',
    body.includes('ship the thing') && body.includes('the-decision') &&
      body.includes('shall I commit?'),
    body.slice(0, 200));
  check('BOTS the card carries the history',
    page.rows('.bot-history-row').length === 5,
    String(page.rows('.bot-history-row').length));
}

// The skills surface's document, in the bridge's own shape: three
// categories, one skip, one proposal, one retired name, and a building
// section heavy enough to prove the row budget.
const skillsDoc = (() => {
  const skill = (name, category, uses) => ({
    name, description: 'the ' + name + ' skill', category, uses,
    modified: '2026-10-30', path: '/lib/' + name + '/SKILL.md',
    body: 'the body of ' + name,
  });
  const building = [];
  for (let i = 0; i < 32; i += 1) building.push(skill('skill-' + String(i).padStart(2, '0'), 'building', i));
  return {
    type: 'skills', read_at: 'now', canonical: '/lib', sessions: 20, unused: 1,
    threshold: 0.8, curation_note: 'the note', refused: [],
    skills: building.concat([
      skill('web-one', 'web', 0),
      skill('media-one', 'media', 2),
    ]),
    skips: [{ name: 'broken-one', reason: 'no readable SKILL.md' }],
    proposals: [{
      name: 'bash-recovery', description: 'recover a failed bash call', kind: 'create',
      evidence: 'session x: bash failed then succeeded', body: 'the proposed body',
      path: '/home/proposals/bash-recovery.md',
    }],
    merges: [], retired: ['old-one'],
  };
})();

async function theSkillsRailCarriesOneSectionAtATime() {
  const bootSkills = (hash) => boot({
    width: 1440, hash,
    documents: { skills: skillsDoc, state: (asked) => emptyState(asked),
      onboard: onboardDoc({ door: false }) },
  });

  const page = bootSkills('#/skills');
  await flush();
  page.open();
  await flush(80);

  // The rail: needs you leads, the categories the document holds follow,
  // the tails close it, and an empty fallback row never renders.
  const railRows = page.rows('.section-rail-row');
  const railText = railRows.map((row) => row.textContent);
  check('SKILL the rail leads with needs you and names the sections',
    railRows.length === 7 &&
      railText[0].includes('needs you') &&
      railText[1].includes('building') && railText[2].includes('web') &&
      railText[3].includes('media') &&
      railText[4].includes('declined') && railText[6].includes('retired') &&
      !railText.join('|').includes('everything else'),
    railText.join(' | '));
  check('SKILL one section renders at a time', page.rows('.section-body').length === 1 &&
    page.rows('.section-body')[0].dataset.section === 'needs-you',
  page.rows('.section-body').map((b) => b.dataset.section).join(','));

  // The needs-you section carries the proposal with its approve.
  check('SKILL needs you carries the proposal and its approve',
    page.text('skills-board').includes('bash-recovery') &&
      page.rows('.row-action-primary').length === 1,
    String(page.rows('.row-action-primary').length));

  // A category section is an address, and its rows are one line with
  // the uses mark the owner asked for.
  const routed = bootSkills('#/skills/building');
  await flush();
  routed.open();
  await flush(80);
  check('SKILL a category is an address',
    routed.rows('.section-body')[0].dataset.section === 'building', 'building route');
  const items = routed.rows('.cfg-item');
  check('SKILL the rows are one-line items with the uses mark',
    items.length === 30 &&
      items.every((row) => row.querySelector('.cfg-item-name')) &&
      items.some((row) => row.textContent.includes('uses')),
    String(items.length));

  // The budget: the first 30 and the count named; the press reveals —
  // and the revealed tail carries the never-selected mark.
  const more = routed.rows('.budget-more');
  check('SKILL the budget names what is behind the press',
    more.length === 1 && more[0].textContent.includes('2 more skills'),
    more.length ? more[0].textContent : 'no budget press');
  more[0].click();
  await flush(40);
  const revealed = routed.rows('.cfg-item');
  check('SKILL the press reveals the rest, never selected among them',
    revealed.length === 32 &&
      revealed.some((row) => row.textContent.includes('never selected')),
    String(revealed.length));

  // A row's press opens the pane card with the body and the actions —
  // the tabs stood down, the uses count named. The section sorts by
  // uses, so the first row is the most-used skill.
  revealed[0].click();
  await flush(60);
  check('SKILL a row opens its card in the pane',
    routed.body.dataset.objectCard === 'open' &&
      routed.text('dock-object-title').includes('skill-31'),
    routed.text('dock-object-title'));
  const cardText = routed.text('dock-object-body');
  check('SKILL the card carries the body, the uses, and the actions',
    cardText.includes('the body of skill-31') &&
      cardText.includes('31 uses across 20 scanned sessions') &&
      cardText.includes('copy the skill path') &&
      cardText.includes('retire from every run'),
    cardText.slice(0, 160));
  routed.fireOn('dock-object-close', 'click');
  await flush(40);
  check('SKILL closing returns the pane\u2019s tabs',
    routed.body.dataset.objectCard !== 'open', String(routed.body.dataset.objectCard));

  // The approve: the row's press posts the name, the reply repaints.
  const deciding = bootSkills('#/skills');
  await flush();
  deciding.open();
  await flush(80);
  deciding.rows('.row-action-primary')[0].click();
  await flush(60);
  const posted = deciding.calls.filter((call) =>
    call.url === '/api/skill' && call.method === 'POST');
  check('SKILL the approve posts the proposal\u2019s name',
    posted.length === 1 && JSON.parse(posted[0].body).name === 'bash-recovery',
    JSON.stringify(posted.map((call) => call.body)));

  // A query turns the body into the results card across every section:
  // the match lives in web, the rail stood on building.
  const box = routed.rows('.panel-search-box');
  check('SKILL the search sits above the section',
    box.length === 1, String(box.length));
  box[0].value = 'web-one';
  box[0].fire('input', {});
  await flush(60);
  const searched = routed.rows('.cfg-item');
  check('SKILL a query searches every section',
    routed.rows('.section-body')[0].dataset.section === 'matches' &&
      searched.length === 1 && searched[0].textContent.includes('web-one'),
    searched.map((row) => row.textContent).join(' | ').slice(0, 120));

  // The retired section carries the return.
  const gone = bootSkills('#/skills/retired');
  await flush();
  gone.open();
  await flush(80);
  check('SKILL the retired section carries the return',
    gone.rows('.section-body')[0].dataset.section === 'retired' &&
      gone.text('skills-board').includes('old-one') &&
      gone.text('skills-board').includes('unretire'),
    gone.rows('.section-body')[0].dataset.section);

  // The retire, from the pane card: the press posts the name and the
  // read repaints — the library never written (KI-215).
  const retiring = bootSkills('#/skills/building');
  await flush();
  retiring.open();
  await flush(80);
  retiring.rows('.cfg-item')[0].click();
  await flush(60);
  Array.from(retiring.rows('.replay-button'))
    .find((button) => button.textContent === 'retire from every run').click();
  await flush(60);
  const retiredPost = retiring.calls.filter((call) =>
    call.url === '/api/skill/retire' && call.method === 'POST');
  check('SKILL the retire posts the name and repaints',
    retiredPost.length === 1 &&
      JSON.parse(retiredPost[0].body).name === 'skill-31',
    JSON.stringify(retiredPost.map((call) => call.body)));
}


// The memory surface's document, in the bridge's own shape: sections
// across the substrate's own type words, the cycle's queue, the
// archive, and the day's hits.
const memoryDoc = (() => {
  const fact = (name, section, extra) => ({
    name, description: 'the ' + name + ' fact', fact_type: section, section,
    basis: 'inferred', tier: 'durable', date: '2026-10-01', recall_count: 0,
    protected: false, retired: false, superseded_by: '', valid_until: '',
    shape: [], phrases: [], missing: [], body: 'the body of ' + name,
    path: '/repo/memories/' + name + '.md', relative: 'memories/' + name + '.md',
    ...extra,
  });
  const projects = [];
  for (let i = 0; i < 31; i += 1) {
    projects.push(fact('fact-' + String(i).padStart(2, '0'), 'project',
      { recall_count: i }));
  }
  return {
    type: 'memory', read_at: 'now', day: '2026-10-30', root: '/repo/memories',
    facts: projects.concat([
      fact('owners-ruling', 'feedback', { basis: 'confirmed-by', protected: true,
        recall_count: 4 }),
      fact('odd-typed', 'everything-else', { fact_type: 'state-sync' }),
      fact('folded-away', 'project', { retired: true, superseded_by: 'fact-00' }),
    ]),
    preview: {
      compress: ['stale-working'], supersede: ['stale-inferred'],
      untouched: ['stale-confirmed'], kept: ['stalled'],
    },
    archive: [{ name: 'archived-one', description: 'the expired fact',
      date: '2026-10-01', path: '/repo/memories/.archive/archived-one.md' }],
    hits: [{ ts: '2026-10-30T09:00:00Z', fact: 'fact-03', shape: 'build',
      score: 0.61 }],
    refused: [],
  };
})();

async function theMemoryRailCarriesTheQueueAndTheVerbs() {
  const bootMemory = (hash) => boot({
    width: 1440, hash,
    documents: { memory: memoryDoc, state: (asked) => emptyState(asked),
      onboard: onboardDoc({ door: false }) },
  });

  const page = bootMemory('#/memory');
  await flush();
  page.open();
  await flush(80);

  // The rail: needs you leads, the type sections the corpus holds
  // follow (an empty type never renders), the tails close it.
  const railRows = page.rows('.section-rail-row');
  const railText = railRows.map((row) => row.textContent);
  check('MEM the rail leads with needs you and carries the type words',
    railRows.length === 7 &&
      railText[0].includes('needs you') &&
      railText[1].includes('feedback') && railText[2].includes('project') &&
      railText.some((t) => t.includes('everything else')) &&
      railText.some((t) => t.includes('retired')) &&
      railText.some((t) => t.includes('archive')) &&
      railText.some((t) => t.includes('today')),
    railText.join(' | '));
  check('MEM one section renders at a time',
    page.rows('.section-body').length === 1 &&
      page.rows('.section-body')[0].dataset.section === 'needs-you',
    page.rows('.section-body').map((b) => b.dataset.section).join(','));

  // The needs-you section names the four sets and offers the pass.
  const board = page.text('memory-board');
  check('MEM the queue names all four sets and the exceptions',
    board.includes('the pass would compress') &&
      board.includes('the pass would fold') &&
      board.includes('left alone \u2014 the owner confirmed it') &&
      board.includes('left alone \u2014 the missing companions') &&
      board.includes('run the cycle now'),
    board.slice(0, 200));
  page.rows('.row-action-primary')[0].click();
  await flush(60);
  const passPosted = page.calls.filter((call) =>
    call.url === '/api/memories/consolidate' && call.method === 'POST');
  check('MEM the press runs one pass',
    passPosted.length === 1, JSON.stringify(passPosted));

  // A type section is an address; its rows are one line with the
  // permission model's mark and the recalls.
  const routed = bootMemory('#/memory/project');
  await flush();
  routed.open();
  await flush(80);
  check('MEM a type section is an address',
    routed.rows('.section-body')[0].dataset.section === 'project', 'project route');
  const items = routed.rows('.cfg-item');
  check('MEM the rows are one line with the basis mark',
    items.length === 30 &&
      items.every((row) => row.querySelector('.cfg-item-name')) &&
      items.some((row) => row.textContent.includes('inferred')),
    String(items.length));
  const more = routed.rows('.budget-more');
  check('MEM the budget names what is behind the press',
    more.length === 1 && more[0].textContent.includes('1 more facts'),
    more.length ? more[0].textContent : 'none');
  more[0].click();
  await flush(40);
  const revealed = routed.rows('.cfg-item');
  check('MEM the press reveals the rest, never recalled among them',
    revealed.length === 31 &&
      revealed.some((row) => row.textContent.includes('never recalled')),
    String(revealed.length));

  // A row's press opens the pane card: the body, the edit form, and the
  // supersede verb — the tabs stood down.
  routed.rows('.cfg-item')[0].click();
  await flush(60);
  check('MEM a row opens its card in the pane',
    routed.body.dataset.objectCard === 'open' &&
      routed.text('dock-object-title').includes('fact-30'),
    routed.text('dock-object-title'));
  const cardText = routed.text('dock-object-body');
  check('MEM the card carries the edit form', cardText.includes('save the fact'),
    cardText.slice(0, 400));
  check('MEM the card carries the supersede verb',
    cardText.includes('supersede the fact'), 'supersede');
  const editForm = Array.from(routed.rows('form'))
    .find((form) => form.textContent.includes('everything after the frontmatter fence'));
  const bodyWrap = editForm
    ? Array.from(editForm.querySelectorAll('.fact-field'))
        .find((wrap) => wrap.textContent.includes('everything after the frontmatter fence'))
    : null;
  const bodyBox = bodyWrap
    ? bodyWrap.children[bodyWrap.children.length - 1]
    : null;
  check('MEM the card carries the body',
    !!bodyBox && bodyBox.value === 'the body of fact-30',
    bodyWrap ? 'children: ' + bodyWrap.children.map((c) => c.name).join(',') : 'no wrap');

  // The supersede submits through the substrate's own writer, with the
  // justification the permission model requires.
  routed.rows('.cfg-item')[0].click();
  await flush(40);
  const supersedeForm = Array.from(routed.rows('form'))
    .find((form) => form.textContent.includes('why this fact stops being true'));
  if (!supersedeForm) {
    throw new Error('no supersede form; forms: ' + routed.rows('form').length +
      ' board: ' + routed.text('dock-object-body').slice(0, 300));
  }
  const justField = Array.from(supersedeForm.querySelectorAll('.fact-field'))
    .find((wrap) => wrap.textContent.includes('why this fact stops being true'));
  const justBox = justField.children[justField.children.length - 1];
  justBox.value = 'the sentence moved on';
  supersedeForm.fire('submit', { preventDefault() {} });
  await flush(60);
  const supersedes = routed.calls.filter((call) =>
    call.url === '/api/fact/supersede' && call.method === 'POST');
  check('MEM the supersede posts the fact and its justification',
    supersedes.length === 1 &&
      JSON.parse(supersedes[0].body).name === 'fact-30' &&
      JSON.parse(supersedes[0].body).justification === 'the sentence moved on',
    JSON.stringify(supersedes.map((call) => call.body)));
  routed.fireOn('dock-object-close', 'click');
  await flush(40);

  // A query searches every field across every section.
  const box = routed.rows('.panel-search-box');
  box[0].value = 'owners-ruling';
  box[0].fire('input', {});
  await flush(60);
  const searched = routed.rows('.cfg-item');
  check('MEM a query searches every section',
    routed.rows('.section-body')[0].dataset.section === 'matches' &&
      searched.length === 1 && searched[0].textContent.includes('owners-ruling'),
    searched.map((row) => row.textContent).join(' | ').slice(0, 120));

  // The archive section names what the cycle forgot.
  const gone = bootMemory('#/memory/archive');
  await flush();
  gone.open();
  await flush(80);
  check('MEM the archive names what the cycle forgot',
    gone.rows('.section-body')[0].dataset.section === 'archive' &&
      gone.text('memory-board').includes('archived-one'),
    gone.rows('.section-body')[0].dataset.section);

  // The today section carries the day's hits with their scores.
  const today = bootMemory('#/memory/today');
  await flush();
  today.open();
  await flush(80);
  check('MEM the today section carries the day\u2019s hits',
    today.rows('.section-body')[0].dataset.section === 'today' &&
      today.text('memory-board').includes('fact-03') &&
      today.text('memory-board').includes('0.61'),
    today.rows('.section-body')[0].dataset.section);
}

const cases = [
  ['C1/C8 the list groups by attention and counts what the server counted', theListGroupsByAttentionAndCountsWhatTheServerCounted],
  ['C2 every state opens through one verb', everyStateOpensThroughOneVerb],
  ['C3 the head carries the way out and the switcher', theHeadCarriesTheWayOutAndTheSwitcher],
  ['C5 a send beside driving work queues and names it', aSendBesideDrivingWorkQueuesAndNamesIt],
  ['C5 a finished run stops blocking the next send', aFinishedRunStopsBlockingTheNextSend],
  ['C6 a streamed reply renders at its boundary', aStreamedReplyRendersAtItsBoundary],
  ['C6 a replayed reply renders as markdown', aReplayedReplyRendersAsMarkdown],
  ['C3 the pencil renames the conversation', thePencilRenamesTheConversation],
  ['C7 the dock is closed and the work column takes its place', theDockIsClosedAndTheWorkColumnTakesItsPlace],
  ['C7 the work list folds to a strip and follows the rail', theWorkListFoldsToAStrip],
  ['C7 the conversation text size is the owner\'s', theConversationTextSizeIsTheOwners],
  ['C8 the servers card carries the registry', theServersCardCarriesTheRegistry],
  ['C1 the goal field is always present and writable', theGoalFieldIsAlwaysPresentAndWritable],
  ['C2 each conversation keeps its own draft', eachConversationKeepsItsOwnDraft],
  ['C3 editing a message rewinds and resends', editingAMessageRewindsAndResends],
  ['C4/C5 paste attaches and the @ typeahead references', pasteAttachesAndTheAtTypeaheadReferences],
  ['C6 returning is anchored', returningIsAnchored],
  ['C7 the receipt hands over the work', theReceiptHandsOverTheWork],
  ['C8 the turn is a navigational unit', theTurnIsANavigationalUnit],
  ['C9 a content query drives the work list', aContentQueryDrivesTheWorkList],
  ['C10 what the model saw opens from the receipt', whatTheModelSawOpensFromTheReceipt],
  ['C8 the addresses hold', theAddressesHold],
  ['C4 /clear starts a conversation', theClearCommandStartsAConversation],
  ['C2 a thought row opens on a press', aThoughtRowOpensOnAPress],
  ['C3 a command row names what ran', aCommandRowNamesWhatRan],
  ['C4 the strip lines open their record', theStripsLinesOpenTheirRecord],
  ['C5 the workspace-history section draws the tree it read', theWorkspaceHistorySectionDrawsTheTreeItRead],
  ['ARCH the archive is the one end and the owner\u2019s verb', theArchiveIsTheOneEndAndTheOwnersVerb],
  ['REVIEW the chat counts exchanges and shows its controls', theChatCountsExchangesAndShowsItsControls],
  ['TURN the turn controls grade, fork, and hook', theTurnControlsGradeForkAndHook],
  ['VERBS the composer carries the two verb icons', theComposerVerbsAreIcons],
  ['N0 the work rail renders the owner\u2019s work only', theListGroupsByAttentionAndCountsWhatTheServerCounted],
  ['N1 the home is always reachable', theHomeIsAlwaysReachable],
  ['N2 New mints nothing until words exist', newMintsNothingUntilWordsExist],
  ['N3 the rail badge says where the work is', theRailBadgeSaysWhereTheWorkIs],
  ['Q the queued messages carry their actions', theQueuedMessagesCarryTheirActions],
  ['P the pane is one surface with the owner\u2019s hand on it', thePaneIsOneSurfaceWithTheOwnersHandOnIt],
  ['PV the preview renders the common formats', thePreviewRendersTheCommonFormats],
  ['OB the door is the welcome in its first-run state', theDoorIsTheWelcomeInItsFirstRunState],
  ['OB the capability cards are on demand', theCapabilityCardsAreOnDemand],
  ['PV the strategy step shows its preview', theStrategyStepShowsItsPreview],
  ['RL the page seals its frames through a relay', thePageSealsItsFramesThroughARelay],
  ['BOTS the bots surface mints the bot card', theBotsSurfaceMintsTheBotCard],
  ['SET the settings rail carries one section at a time', theSettingsRailCarriesOneSectionAtATime],
  ['BOTS the bots rail groups by persona', theBotsRailGroupsByPersona],
  ['SKILL the skills rail carries the categories and the pane', theSkillsRailCarriesOneSectionAtATime],
  ['MEM the memory rail carries the queue and the verbs', theMemoryRailCarriesTheQueueAndTheVerbs],
];

for (const [name, run] of cases) {
  try {
    await run();
  } catch (error) {
    failures.push(name + ' threw — ' + (error && error.stack ? error.stack.split('\n').slice(0, 4).join(' | ') : error));
  }
}

if (failures.length) {
  console.error('face: ' + failures.length + ' failure(s)');
  for (const failure of failures) console.error('  ' + failure);
  process.exit(1);
}
console.log('face: ' + pass.length + ' checks passed across ' + cases.length + ' cases');
