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
  const tag = text.match(/^[a-zA-Z][\w-]*/);
  if (tag && el.name !== tag[0]) return false;
  const ids = text.match(/#[^.#\s]+/g) || [];
  if (ids.some((id) => el.id !== id.slice(1))) return false;
  const classes = text.match(/\.[^.#\s]+/g) || [];
  const held = el.className.split(/\s+/).filter(Boolean);
  if (classes.some((cls) => !held.includes(cls.slice(1)))) return false;
  return Boolean(tag || ids.length || classes.length);
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
  setAttribute(k, v) { this.attrs[k] = String(v); if (k === 'id') this.id = String(v); }
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
  const window_ = {
    location: { search: '', hash: opts.hash || '', pathname: '/', href: 'http://127.0.0.1/' },
    history: {
      replaceState: (_a, _b, url) => { window_.location.hash = String(url).replace(/^[^#]*/, ''); },
      pushState: (_a, _b, url) => { window_.location.hash = String(url).replace(/^[^#]*/, ''); },
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
    focus: () => {},
  };
  const navigator_ = { clipboard: { writeText: () => Promise.resolve() }, userAgent: 'node', language: 'en' };
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
    window: window_,
    sandbox,
    push(event) {
      events.push(event);
      if (streamSource && streamSource.onmessage) streamSource.onmessage({ data: JSON.stringify(event) });
    },
    open() { if (streamSource && streamSource.onopen) streamSource.onopen({}); },
    hash(next) {
      window_.location.hash = next;
      (windowListeners.hashchange || []).forEach((fn) => fn({}));
    },
    fireOn(id, kind, event) { IDS[id].fire(kind, event || {}); },
    text: (id) => (IDS[id] ? IDS[id].textContent : ''),
    rows: (sel) => documentElement.querySelectorAll(sel),
  };
}

const emptyState = (session) => ({
  session: session || '', model: 'm', workspace: '/w', folder: null, goal: null,
  budget: 100000, turn_budget: 8, gate: { name: 'sys1', informs_failover: false },
  running: false, driving: null, dropped: 0, events: [], approval: null, rules: [], attached: true,
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
// the background is one line per family, and the counts are the
// document's — not a second derivation in the browser. The fixture's
// counts deliberately disagree with the rows it carries: only a page
// that reads the server's numbers can print them.
async function theListGroupsByAttentionAndCountsWhatTheServerCounted() {
  const sessions = {
    type: 'sessions', read_at: 'now', current: '',
    counts: { needs: 3, working: 1, background: 2, earlier: 1, yours: 4, total: 7 },
    sessions: [
      row('paused-1', 'paused', { title: 'corpus-refresh-fleet — paused', class: 'background', job: 'corpus-refresh-fleet' }),
      row('needs-1', 'needs-approval', { title: 'the ledger writer' }),
      row('work-1', 'working', { title: 'skills the model can reach' }),
      row('done-1', 'completed', { title: 'conversation control' }),
      row('bg-1', 'completed', { title: 'corpus-refresh fleet', class: 'background', job: 'corpus-refresh-fleet' }),
      row('bg-2', 'interrupted', { title: 'corpus-refresh fleet', class: 'background', job: 'corpus-refresh-fleet' }),
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
  check('C1 the head prints the server count', /3 need you/.test(page.text('work-head')), page.text('work-head'));
  check('C1 the needs header prints the server count', /needs you3/.test(groups[0] || ''), groups[0]);
  // The app's own work is one quiet line, closed by default: no family
  // and no scheduled row is in the list until the owner asks for them.
  const background = page.rows('.work-background');
  check('C1 the background is one line', background.length === 1, String(background.length));
  check('C1 the background says how much is hidden', /background/.test(background[0] ? background[0].textContent : '') &&
    /2 runs/.test(background[0] ? background[0].textContent : '') &&
    /hidden/.test(background[0] ? background[0].textContent : ''),
    background[0] ? background[0].textContent : 'none');
  check('C1 no family is shown until asked', page.rows('.work-family').length === 0,
    String(page.rows('.work-family').length));
  const backgroundAt = page.IDS['work-list'].children.indexOf(background[0]);
  const workingAt = page.IDS['work-list'].children.findIndex((kid) => /^working/.test(kid.textContent));
  const earlierHeaderAt = page.IDS['work-list'].children.findIndex((kid) => /^earlier/.test(kid.textContent));
  check('C1 the background sits between working and earlier',
    backgroundAt > workingAt && backgroundAt < earlierHeaderAt,
    backgroundAt + ' vs ' + workingAt + '/' + earlierHeaderAt);
  if (background[0]) background[0].click();
  await flush();
  check('C1 expanding shows the families', page.rows('.work-family').length === 1,
    String(page.rows('.work-family').length));
  check('C1 the family names the job and its runs', /corpus-refresh-fleet/.test(page.text('work-list')),
    page.text('work-list').slice(0, 200));
  if (page.rows('.work-background')[0]) page.rows('.work-background')[0].click();
  await flush();
  check('C8 no id opens a conversation first', /#\/sessions\/.+/.test(page.window.location.hash), page.window.location.hash);
  check('C8 the opened one is the first row that needs the owner', page.window.location.hash.includes('paused-1'), page.window.location.hash);

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
  const news = page.calls.filter((c) => c.url.startsWith('/api/session/new'));
  check('C3 New posts the route', news.length === 1, String(news.length));
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


// C7: the dock is closed by default at 1600 and the work column holds
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
  check('C7 the dock is closed by default at 1600', page.body.dataset.aside === 'closed',
    String(page.body.dataset.aside));
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
  const page = boot({
    width: 1440,
    hash: '#/settings',
    documents: {
      settings: {
        type: 'settings', read_at: 'now',
        scalars: [], lists: [], rules: [], env: [],
        cron: [], soul: { present: false, text: '' },
        router: {}, catalog: {}, roles: { entries: [] }, vault: { names: [] },
        lanes: [],
        servers: {
          servers: [
            { name: 'github', enabled: true, program: 'docker',
              args: ['run', '-i', '--rm', '-e', 'GITHUB_PERSONAL_ACCESS_TOKEN', 'ghcr.io/github/github-mcp-server'],
              env_names: ['GITHUB_PERSONAL_ACCESS_TOKEN'],
              last_probe: { at: '2026-10-23T16:30Z', ok: true, server_name: 'github-mcp',
                server_version: '0.9.0', tools: 12, reason: '' },
              source: { harness: 'claude-desktop', imported: '2026-10-23' } },
            { name: 'time', enabled: false, program: 'npx', args: ['-y', 'server-time'],
              env_names: [], last_probe: null, source: null },
          ],
        },
      },
    },
    reply: { type: 'settings_write', op: 'server-scan',
      scan: { rows: [
        { name: 'weather', harness: 'claude-desktop', file: '/x/config.json',
          badge: 'bare interpreter', status: 'available',
          launch: 'python3 -m weather', env_names: ['OWM_API_KEY'] },
        { name: 'tinyfish', harness: 'cursor', file: '/x/mcp.json',
          badge: 'remote endpoint', status: 'unsupported',
          launch: 'remote endpoint: needs a transport', env_names: [] },
      ], skips: [] } },
  });
  await flush();
  page.open();
  await flush();
  await flush();
  const board = () => page.rows('#settings-servers')[0];
  // The server rows live under the servers card; the plain query caught
  // none — the board's own rows sit beneath it, so scope the walk.
  // The stub's selector engine takes no :not() — the scan rows are
  // filtered by their own class.
  const serverRows = () =>
    board() ? board().all('.server-row').filter((row) => !row.className.includes('scan')) : [];
  check('C8 the servers card is on the board', !!board(), String(!!board()));
  check('C8 one row per configured server', serverRows().length === 2,
    String(serverRows().length));
  const text = board().textContent;
  check('C8 the row names its server and its probe', text.includes('github') &&
      text.includes('github-mcp v0.9.0'), text.slice(0, 200));
  check('C8 the source badge names the harness',
    text.includes('imported from claude-desktop'), text.slice(0, 300));
  check('C8 the env line says it resolves at spawn',
    text.includes('env GITHUB_PERSONAL_ACCESS_TOKEN') && text.includes('resolves at spawn'),
    text.slice(0, 300));
  check('C8 the disabled server says so', text.includes('disabled'), '');
  // The scan: the discovery rows carry the badge, and the unsupported
  // import is disabled. Every action repaints the board, so each step
  // re-queries what it clicks.
  board().all('.bots-act').find((b) => b.dataset.act === 'server-scan').click();
  await flush(80);
  check('C8 the scan\'s discovery rows are on the card',
    serverRows().length === 2 && board().all('.server-row.scan').length === 2,
    String(board().all('.server-row.scan').length));
  check('C8 the badge rides the discovery row',
    board().textContent.includes('bare interpreter'), '');
  const importButtons = board().all('.server-row.scan').map((row) =>
    row.all('.bots-act').find((b) => b.dataset.act === 'server-import'));
  check('C8 the available entry imports', importButtons[0] && !importButtons[0].disabled, '');
  check('C8 the unsupported entry\'s import is disabled',
    importButtons[1] && importButtons[1].disabled, '');
  // The toggle posts the registry's own op and the read repaints.
  serverRows()[1].all('.bots-act').find((b) => b.dataset.act === 'server-toggle').click();
  await flush(80);
  const posted = page.calls.filter((c) => c.method === 'POST' && c.url === '/api/settings');
  check('C8 the toggle posts the registry\'s op',
    posted.some((c) => c.body && c.body.includes('"op":"server"') &&
      c.body.includes('"name":"time"')),
    JSON.stringify(posted.map((c) => c.body)));
  // A probe writes its outcome through the same route.
  serverRows()[0].all('.bots-act').find((b) => b.dataset.act === 'server-probe').click();
  await flush(80);
  check('C8 the probe posts its op',
    page.calls.some((c) => c.body && c.body.includes('"op":"server-probe"')), '');
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
  const news = page.calls.filter((call) => call.url.startsWith('/api/session/new'));
  check('C4 /clear posts the new-session route', news.length === 1, String(news.length));
  check('C4 /clear opens the conversation it made',
    page.window.location.hash === '#/sessions/clear-made-1', page.window.location.hash);
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
