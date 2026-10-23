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
    this.style = { setProperty() {}, removeProperty() {} };
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
  const storage = new Map();

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
