// The mini console's own tests (docs/plans/2026-11-21-mini-console.md).
//
// Zero dependencies, the repository's rule: this file *is* the runner, and
// `tests/console.rs` shells out to it when node is present. Two halves:
//
//   1. the core — the real `src/gui/assets/console.js` loaded with no
//      `document`, which hands back its panel renderers, its wrapping law
//      and its four-sentence table. Every panel's sentences are pinned
//      here by their exact words, so a wording that drifts fails a case
//      that says which sentence it expected.
//   2. the shell — the same file booted against a stub DOM and a stub
//      bridge, driven the way a pointer and a keystroke would drive it,
//      so the wiring that prints a read is covered and not assumed.
//
// The size budget is a case like any other: the AC is arithmetic (60 KB
// uncompressed, 20 KB gzip over html+js+css), so the gate does the sum and
// a file that grows past it fails by name with both numbers.
import fs from 'node:fs';
import path from 'node:path';
import vm from 'node:vm';
import { gzipSync } from 'node:zlib';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(HERE, '../..');
const ASSETS = ['console.html', 'console.js', 'console.css']
  .map((name) => path.join(ROOT, 'src/gui/assets', name));
const SOURCE = fs.readFileSync(path.join(ROOT, 'src/gui/assets/console.js'), 'utf8');
const CSS = fs.readFileSync(path.join(ROOT, 'src/gui/assets/console.css'), 'utf8');

const BUDGET_BYTES = 60 * 1024;
const BUDGET_GZIP = 20 * 1024;

// ------------------------------------------------------------- the core

function loadCore() {
  const sandbox = { console };
  vm.createContext(sandbox);
  vm.runInContext(SOURCE, sandbox, { filename: 'console.js' });
  if (!sandbox.__console) throw new Error('console.js handed back no core');
  return sandbox.__console;
}

const C = loadCore();

// ------------------------------------------------------------ the checks

let passed = 0;
const failed = [];
const pending = [];
function check(name, body) {
  const ok = () => {
    passed += 1;
    console.log('ok   ' + name);
  };
  const no = (error) => {
    failed.push(name);
    console.log('FAIL ' + name + '\n     ' + (error && error.message ? error.message : error));
  };
  try {
    const result = body();
    // A case that drives the shell is asynchronous — the transcript is
    // written by a promise chain — so it is awaited before the verdict
    // rather than reported before it ran.
    if (result && typeof result.then === 'function') pending.push(result.then(ok, no));
    else ok();
  } catch (error) {
    no(error);
  }
}

function equal(actual, expected, what) {
  if (actual !== expected) {
    throw new Error(`${what || 'value'}: expected ${JSON.stringify(expected)}, ` +
      `got ${JSON.stringify(actual)}`);
  }
}

function contains(text, needle, what) {
  if (String(text).indexOf(needle) < 0) {
    throw new Error(`${what || 'text'} should carry ${JSON.stringify(needle)}; ` +
      `it carries ${JSON.stringify(String(text).slice(0, 240))}`);
  }
}

function lacks(text, needle, what) {
  if (String(text).indexOf(needle) >= 0) {
    throw new Error(`${what || 'text'} should not carry ${JSON.stringify(needle)}`);
  }
}

const body = (lines) => lines.map((line) => line.text).join('\n');
// Rendered text is wrapped for a column, so a sentence a case pins is
// compared with its whitespace flattened — the wrap is the face's own
// decision and a case must not fail because the break landed elsewhere.
const flat = (text) => String(text).replace(/\s+/g, ' ').trim();

// ------------------------------------------------------- the fixtures
//
// The documents are the bridge's own shapes, cut down: every field the
// panels read, and nothing the panels invent.

const BOARD = {
  type: 'board', read_at: '09:00',
  counts: { queued: 2, 'in-process': 1, done: 1, plans: 2 },
  plans: [
    {
      id: 'p-1', plan: 'PLAN-01', status: 'running', queued: false, workload: 'hard',
      tasks: [
        { id: 't-1', title: 'write the read', column: 'queued', state: 'waiting', waiting_on: '' },
        { id: 't-2', title: 'wire the route', column: 'in-process', state: 'working', waiting_on: 't-1' },
        { id: 't-3', title: 'land the gate', column: 'done', state: 'verified' },
      ],
    },
    {
      id: 'p-2', plan: 'PLAN-02', status: 'draft',
      tasks: [{ id: 't-4', title: 'draft the plan', column: 'queued', state: 'waiting' }],
    },
  ],
  tasks: [],
  refused: [],
};

const DELEGATION = {
  type: 'delegation', read_at: '09:01', depth_cap: 2,
  live: { id: 's-live', kind: 'delegated', depth: 0, goal: 'wire the route', status: 'running', turns: 4, children: [] },
  history: [],
  graph: {
    revisions: 1,
    critical_path: 'n-1',
    blocks_most_work: 'n-2',
    weakness: { sentence: 'n-2 is never spawned, and the graph says running' },
    divergences: { never_spawned: 1, ran_twice: 1, basis: "the day's delegation rows" },
    nodes: [
      {
        id: 'n-1', status: 'running', state: 'verified', cost: 3, declared_workload: 'hard',
        overlay: { spawned_sessions: ['s-1'], spawn_count: 1, divergence: null,
          brief_shape: 'bug', provider: 'deepseek', outcome: 'ok' },
      },
      {
        id: 'n-2', status: 'queued', cost: 2,
        overlay: { spawned_sessions: [], spawn_count: 0, divergence: 'never spawned',
          brief_shape: '', provider: '', outcome: '' },
      },
      {
        id: 'n-3', status: 'done', cost: 1,
        overlay: { spawned_sessions: ['a', 'b', 'c'], spawn_count: 3, divergence: 'ran twice ×3',
          brief_shape: 'feat', provider: 'sys1', outcome: 'capped at 8 turns' },
      },
    ],
  },
  refused: [],
};

const BRAIN = {
  type: 'brain', home: '/b', read_at: '09:02',
  attention: {
    total: 3, counts: { drift: 1, stale: 1, handoff: 1, conflicts: 0 },
    drift: [{ project: 'fleet', what: 'the corpus moved', evidence: 'HEAD != revision' }],
    stale: [], handoff: [], conflicts: [],
  },
  reports: { doing_total: 2, pending_verification: 1, open_conflicts: 0, feed: [] },
  harness: { loops: [{ name: 'l-1', runs: 3, converged: 2 }], recent: [{ ts: '08:10', kind: 'run', label: 'gate', outcome: 'green' }] },
  loops: { loops: [{ name: 'l-1', family: 'build', verdict: 'keep', iterations: 4, pending_review: 0 }], findings: [] },
  maturity: { weeks: ['w1'], series: [], milestones: [] },
  tower: { projects: [{ name: 'fleet', commits_30d: 12, dirty: 1 }], timeline: ['d1'] },
  refused: [],
};

const SESSIONS = {
  type: 'sessions', read_at: '09:03',
  counts: { yours: 2, needs: 1, working: 1, archived: 0 },
  sessions: [
    { id: 's-1', title: 'build the console', state: 'working', turns: 9, current: true },
    { id: 's-2', title: 'land the relay route', state: 'needs-approval', turns: 3 },
  ],
  refused: [],
};

// -------------------------------------------------------- the law itself

check('every panel owes the four sentences, and only the four', () => {
  const want = ['empty', 'loading', 'refused', 'selected'];
  Object.keys(C.SENTENCES).forEach((panel) => {
    const have = Object.keys(C.SENTENCES[panel]).sort();
    equal(have.join(','), want.slice().sort().join(','), `SENTENCES.${panel}`);
  });
  equal(Object.keys(C.SENTENCES).length, 4, 'the four panels');
});

// ------------------------------------------------------------- the board

check('board · the empty state is a sentence, not a zero', () => {
  const said = body(C.boardLines({ plans: [], tasks: [], counts: {}, refused: [] }, {}));
  contains(said, 'no approved plan carries work right now', 'the board empty state');
  contains(said, 'approve a plan and its tasks land here', 'the way forward');
  lacks(said, '\n0\n', 'a bare zero in place of a sentence');
});

check('board · the loading sentence is its own words', () => {
  equal(C.SENTENCES.board.loading, 'reading the board\u2026');
});

check('board · a refusal prints by name', () => {
  const said = body(C.boardLines({ refused: ['the plans directory is unreadable'] }, {}));
  contains(said, 'the board refused: the plans directory is unreadable.', 'the refusal');
});

check('board · the selected sentence names the plan and its count', () => {
  const lines = C.boardLines(BOARD, {});
  const selected = lines.filter((line) => line.sentence);
  equal(selected.length, 1, 'one selected sentence');
  equal(selected[0].text,
    'focused: PLAN-01 · 3 tasks — every plan is one press away.', 'the selected sentence');
  contains(body(lines), '▸ queued · 2', 'the queued column');
  // One card per column, carrying that column's count — the TUI's own
  // grouping, printed as text.
  contains(body(lines), 'PLAN-01 · 1 task · running', 'the plan card');
});

check('board · an empty column says so in words', () => {
  const said = body(C.boardLines({
    counts: { queued: 1, 'in-process': 0, done: 0, plans: 1 },
    plans: [{ id: 'p', plan: 'PLAN-9', status: 'queued', tasks: [{ id: 't', title: 'x', column: 'queued' }] }],
    tasks: [], refused: [],
  }, {}));
  contains(said, 'nothing in this column yet', 'the empty column');
  contains(said, '▸ done · 0', 'the column heading still counts its own zero');
});

check('board · a plan that is not on the board is refused by name', () => {
  const said = body(C.boardLines(BOARD, { focus: 'p-404' }));
  contains(said, 'no plan on this board is named `p-404`', 'the refusal');
});

check('board · a named plan prints its tasks', () => {
  const said = body(C.boardLines(BOARD, { focus: 'p-2' }));
  contains(said, 't-4 · draft the plan · queued', 'the focused plan’s task');
  contains(said, 'focused: PLAN-02 · 1 task —', 'the singular count');
});

// -------------------------------------------------------- the delegation

check('delegation · the overlay tile prints both divergence words', () => {
  const said = body(C.delegationLines(DELEGATION, {}));
  contains(said, 'never spawned 1 · ran twice 1', 'the tile');
  contains(said, "the day's delegation rows", 'the basis');
});

check('delegation · a divergence word prints on its node', () => {
  const lines = C.delegationLines(DELEGATION, {});
  const n2 = lines.find((line) => line.text.trim().indexOf('n-2 ·') === 0);
  const n3 = lines.find((line) => line.text.trim().indexOf('n-3 ·') === 0);
  contains(n2.text, 'never spawned', 'the node the rows never spawned');
  contains(n3.text, 'ran twice ×3', 'the node that ran more than twice');
  equal(n2.kind, 'warn', 'a divergence is a warning line');
});

check('delegation · why this child is the read’s own three fields', () => {
  const said = body(C.delegationLines(DELEGATION, {}));
  contains(said, 'why this child: bug on deepseek — ok · 1 child spawned', 'the line');
  const focused = body(C.delegationLines(DELEGATION, { focus: 'n-3' }));
  contains(focused, 'why this child: feat on sys1 — capped at 8 turns · 3 children spawned',
    'the latest child wins');
});

check('delegation · a graph with no divergence says so instead of two zeros', () => {
  const clean = JSON.parse(JSON.stringify(DELEGATION));
  clean.graph.divergences = { never_spawned: 0, ran_twice: 0, basis: "the day's delegation rows" };
  const said = body(C.delegationLines(clean, {}));
  contains(said, 'never spawned 0 · ran twice 0 · the rows and the graph agree', 'the tile');
});

check('delegation · the empty state names what is missing and why', () => {
  const said = body(C.delegationLines({ refused: [] }, {}));
  contains(said, 'this home has declared no dependency graph', 'the empty state');
  contains(said, 'nothing here is inferred from what a run did', 'the reason');
});

check('delegation · the loading and refused sentences are its own', () => {
  equal(C.SENTENCES.delegation.loading, 'reading the delegation\u2026');
  contains(body(C.delegationLines({ refused: ['the graph file is malformed'] }, {})),
    'the delegation refused: the graph file is malformed.', 'the refusal');
});

check('delegation · the selected sentence counts the graph', () => {
  const selected = C.delegationLines(DELEGATION, {}).filter((line) => line.sentence);
  equal(selected.length, 1, 'one selected sentence');
  equal(selected[0].text,
    'showing the run in flight · 3 nodes — the overlay is the day’s rows, not a guess.',
    'the selected sentence');
});

// ------------------------------------------------------ the second brain

check('second-brain · the morning read is five lines over one document', () => {
  const lines = C.morningLines(BRAIN);
  equal(lines.length, 5, 'five lines');
  equal(lines[0].text, '3 things need you — 1 drift · 1 stale · 1 handoff · 0 conflicts');
  equal(lines[1].text, 'first: fleet · the corpus moved · HEAD != revision');
  equal(lines[2].text, '2 in flight · 1 awaiting verification · 0 conflicts open');
  equal(lines[3].text, '08:10 run: gate — green');
  equal(lines[4].text, 'the loops are quiet');
});

check('second-brain · an empty morning says nothing needs you, not zero', () => {
  const quiet = JSON.parse(JSON.stringify(BRAIN));
  quiet.attention.total = 0;
  quiet.attention.counts = { drift: 0, stale: 0, handoff: 0, conflicts: 0 };
  quiet.attention.drift = [];
  const lines = C.morningLines(quiet);
  equal(lines[0].text, 'nothing needs you today');
  equal(lines[1].text, 'nothing is queued first');
});

check('second-brain · a section that did not land is named, never counted as zero', () => {
  const partial = JSON.parse(JSON.stringify(BRAIN));
  delete partial.harness;
  const said = body(C.secondBrainLines(partial));
  contains(said, 'the harness section did not land — its refusal is named below', 'the section');
  contains(said, '▸ harness · not on this read', 'the heading');
});

check('second-brain · the empty state is the home sentence', () => {
  const said = body(C.secondBrainLines({ refused: [] }));
  contains(said, 'the second brain has no home on this machine', 'the empty state');
});

check('second-brain · the loading and refused sentences are its own', () => {
  equal(C.SENTENCES.secondBrain.loading, 'reading the second brain\u2026');
  contains(body(C.secondBrainLines({ refused: ['the tower file is unreadable'] }, {})),
    'the second brain refused: the tower file is unreadable.', 'the refusal');
});

check('second-brain · the selected sentence names the queue', () => {
  const selected = C.secondBrainLines(BRAIN).filter((line) => line.sentence);
  equal(selected[0].text,
    'the morning read · 3 things need you — the six sections are below.', 'the selected sentence');
  const one = JSON.parse(JSON.stringify(BRAIN));
  one.attention.total = 1;
  equal(C.secondBrainLines(one).filter((line) => line.sentence)[0].text,
    'the morning read · 1 thing needs you — the six sections are below.', 'the singular');
});

// ------------------------------------------------------------ the archive

check('sessions · the empty state is a sentence with the way forward', () => {
  const said = body(C.sessionsLines({ sessions: [], counts: {}, refused: [] }, {}));
  contains(said, 'no conversation under this home yet', 'the empty state');
});

check('sessions · the loading and refused sentences are its own', () => {
  equal(C.SENTENCES.sessions.loading, 'reading the archive\u2026');
  contains(body(C.sessionsLines({ refused: ['the archive directory is unreadable'] }, {})),
    'the archive refused: the archive directory is unreadable.', 'the refusal');
});

check('sessions · the selected sentence names the open conversation', () => {
  const selected = C.sessionsLines(SESSIONS, { limit: 12 }).filter((line) => line.sentence);
  equal(selected[0].text,
    'open: build the console · working — the transcript below is the whole of it.',
    'the selected sentence');
});

check('sessions · a row is a press that names its own address', () => {
  const rows = C.sessionsLines(SESSIONS, { limit: 12 }).filter((line) => line.press);
  equal(rows.length, 2, 'two pressable rows');
  equal(rows[0].press.cmd, '/sessions', 'the command');
  equal(rows[0].press.arg, 's-1', 'the session it opens');
});

check('sessions · one conversation prints as pure transcript text', () => {
  const said = body(C.transcriptLines({
    type: 'replay', id: 's-1', model: 'deepseek', ended: true, stop_reason: 'done',
    turns: 9, compactions: 1, omitted: 2,
    cards: [
      { type: 'message', role: 'owner', text: 'build it' },
      { type: 'message', role: 'assistant', persona: 'tower', text: 'built.\n\nsecond para' },
      { type: 'tool_call', tool: 'bash', summary: 'cargo test' },
      { type: 'tool_result', tool: 'bash', preview: 'ok' },
    ],
    refused: [],
  }));
  contains(said, 'owner ›', 'the owner’s byline');
  contains(said, '  build it', 'the owner’s words');
  contains(said, 'tower ›', 'the persona’s byline');
  contains(said, '  second para', 'the second paragraph, as its own line');
  contains(said, '· bash: cargo test', 'the call');
  contains(said, '· bash returned: ok', 'the result');
  contains(said, '2 rows omitted from this read', 'the bound, named');
});

// ------------------------------------------------------------ the wrapping

check('the wrap is terminal-grade: a column, a hard break, nothing lost', () => {
  equal(C.wrap('x'.repeat(100), 40).join('|'), 'x'.repeat(40) + '|' + 'x'.repeat(40) + '|' + 'x'.repeat(20));
  equal(C.wrap('the quick brown fox jumps over the lazy dog', 20).join('|'),
    'the quick brown fox|jumps over the lazy|dog');
  // Nothing is dropped: the characters come back in order, whatever the
  // break did.
  const said = 'a-token-with-no-spaces-at-all-'.repeat(4);
  equal(C.wrap(said, 24).join('').replace(/\s/g, ''), said, 'no character lost');
  equal(C.wrap('', 40).length, 1, 'an empty paragraph is one empty line');
  equal(C.wrap('one\ntwo', 40).length, 2, 'a paragraph per line');
});

check('a wrapped line keeps its indent and hangs its continuations', () => {
  const lines = C.wrapLine('  ' + 'word '.repeat(30).trim(), 30);
  lines.forEach((line) => equal(line.slice(0, 2), '  ', 'the indent is kept'));
  lines.slice(1).forEach((line) => equal(line.slice(0, 4), '    ', 'the continuation hangs'));
  lines.forEach((line) => {
    if (line.length > 30) throw new Error('a line ran past the column: ' + JSON.stringify(line));
  });
});

check('a line that already fits is left alone', () => {
  equal(C.wrapLine('  short', 40).join('|'), '  short');
});

// -------------------------------------------------------------- the shell

/// A DOM just large enough for the console's own wiring: elements with a
/// class, text, attributes and listeners, and an event that a test can
/// dispatch. No CSS engine, no layout — the assertions are on the
/// transcript the program built, not on pixels.
function makeDom() {
  function node(tag) {
    return {
      tagName: tag.toUpperCase(),
      className: '',
      textContent: '',
      children: [],
      attributes: {},
      listeners: {},
      clientWidth: 0,
      scrollTop: 0,
      scrollHeight: 0,
      value: '',
      setAttribute(name, value) { this.attributes[name] = String(value); },
      getAttribute(name) { return Object.prototype.hasOwnProperty.call(this.attributes, name)
        ? this.attributes[name] : null; },
      removeAttribute(name) { delete this.attributes[name]; },
      appendChild(child) {
        this.children.push(child);
        if (child && child.__text) this.textContent += child.__text;
        else if (child) this.textContent += child.textContent || '';
        return child;
      },
      addEventListener(name, fn) { (this.listeners[name] = this.listeners[name] || []).push(fn); },
      dispatch(name, event) { (this.listeners[name] || []).forEach((fn) => fn(event || {})); },
      focus() { this.focused = true; },
    };
  }
  const app = node('div');
  app.id = 'app';
  const documentElement = node('html');
  const document = {
    documentElement,
    defaultView: null,
    createElement: (tag) => node(tag),
    createTextNode: (text) => ({ __text: String(text), textContent: String(text) }),
    getElementById: (id) => (id === 'app' ? app : null),
  };
  return { document, app, documentElement };
}

/// Boot the real console against a stub bridge. `routes` maps a path to the
/// document it answers with; a path that is absent refuses the way the
/// bridge refuses — an HTTP status and a body.
function bootConsole(routes) {
  const dom = makeDom();
  const asked = [];
  const fetchStub = async (url) => {
    const key = String(url);
    asked.push(key);
    if (!Object.prototype.hasOwnProperty.call(routes, key)) {
      return { ok: false, status: 404, json: async () => ({ error: 'no such read' }) };
    }
    return { ok: true, status: 200, json: async () => routes[key] };
  };
  const sandbox = {
    console,
    document: dom.document,
    window: {
      addEventListener() {},
      fetch: fetchStub,
      localStorage: { getItem: () => null },
    },
    fetch: fetchStub,
    location: { hash: '', href: 'http://127.0.0.1:7777/console/' },
    URL,
    Headers: class { get() { return null; } },
    Response: class {},
    TextEncoder,
    crypto: { getRandomValues: (bytes) => bytes },
    btoa: (text) => Buffer.from(text, 'binary').toString('base64'),
    atob: (text) => Buffer.from(text, 'base64').toString('binary'),
    setTimeout,
    clearTimeout,
    Date,
    Math,
    JSON,
    Object,
    Array,
    String,
    Number,
    isFinite,
    parseFloat,
  };
  sandbox.globalThis = sandbox;
  sandbox.window.document = dom.document;
  vm.createContext(sandbox);
  vm.runInContext(SOURCE, sandbox, { filename: 'console.js' });
  const view = dom.app.children.find((child) => child.id === 'log');
  const form = dom.app.children.find((child) => child.className === 'prompt');
  const input = form.children.find((child) => child.id === 'cmd');
  const rows = () => view.children.map((child) => ({
    kind: child.getAttribute('data-kind'),
    text: child.textContent,
  }));
  const said = () => rows().map((row) => row.text).join('\n');
  const spoken = () => flat(said());
  return { dom, asked, view, form, input, rows, said, spoken };
}

check('the shell boots with a welcome that is not a blank page', () => {
  const page = bootConsole({});
  contains(page.spoken(), 'fleet console', 'the welcome');
  contains(page.spoken(), '/help lists them', 'the way forward');
  equal(page.input.placeholder.indexOf('/board') >= 0, true, 'the placeholder names a command');
});

check('a typed command echoes, reads, and prints its receipt', async () => {
  const page = bootConsole({ '/api/board': BOARD });
  page.input.value = '/board';
  page.form.dispatch('submit', { preventDefault() {} });
  await new Promise((resolve) => setTimeout(resolve, 0));
  const said = page.spoken();
  contains(said, '» /board', 'the echo — every press says what it did');
  contains(said, 'reading the board…', 'the loading sentence');
  contains(said, '· read in', 'the receipt');
  contains(said, '▸ queued · 2', 'the panel');
  contains(said, 'focused: PLAN-01 · 3 tasks', 'the selected sentence');
  equal(page.asked[0], '/api/board', 'the read the face joins');
});

check('a tapped row runs its own command', async () => {
  const page = bootConsole({ '/api/board': BOARD });
  page.input.value = '/board';
  page.form.dispatch('submit', { preventDefault() {} });
  await new Promise((resolve) => setTimeout(resolve, 0));
  const card = page.view.children.find((child) => child.getAttribute('data-press') === '1');
  card.dispatch('click', {});
  await new Promise((resolve) => setTimeout(resolve, 0));
  contains(page.spoken(), '» /board p-1', 'the press echoes the address it ran');
});

check('a read that refuses prints the refusal by name, not a blank', async () => {
  const page = bootConsole({});
  page.input.value = '/delegation';
  page.form.dispatch('submit', { preventDefault() {} });
  await new Promise((resolve) => setTimeout(resolve, 0));
  const said = page.spoken();
  contains(said, 'the delegation refused:', 'the panel’s refusal');
  contains(said, '/api/delegation answered 404 — no such read', 'the reason, with the path');
});

check('a word the console does not know answers by name', async () => {
  const page = bootConsole({});
  page.input.value = '/kanzo';
  page.form.dispatch('submit', { preventDefault() {} });
  await new Promise((resolve) => setTimeout(resolve, 0));
  contains(page.spoken(), 'not on this surface yet — the console says so by name: /kanzo',
    'the not-on-this-surface sentence');
});

check('the prompt completes a command from the keyboard', () => {
  const page = bootConsole({});
  page.input.value = '/sec';
  page.input.dispatch('keydown', { key: 'Tab', preventDefault() {} });
  equal(page.input.value, '/second-brain ', 'the completion');
});

// ------------------------------------------------------------ the budget
//
// The AC is arithmetic, so the gate does the sum. Both numbers are pinned:
// 60 KB uncompressed over the three files, 20 KB gzip over the same three.

check('the shell fits its budget: under 60 KB uncompressed, under 20 KB gzip', () => {
  const table = ASSETS.map((file) => {
    const bytes = fs.readFileSync(file);
    return {
      name: path.basename(file),
      raw: bytes.length,
      gzip: gzipSync(bytes, { level: 9 }).length,
    };
  });
  const raw = table.reduce((sum, row) => sum + row.raw, 0);
  const gz = table.reduce((sum, row) => sum + row.gzip, 0);
  table.forEach((row) => console.log(`     ${row.name.padEnd(12)} ${String(row.raw).padStart(6)} B  ` +
    `${String(row.gzip).padStart(5)} B gz`));
  console.log(`     ${'total'.padEnd(12)} ${String(raw).padStart(6)} B  ${String(gz).padStart(5)} B gz` +
    `   (budget ${BUDGET_BYTES} B / ${BUDGET_GZIP} B gz)`);
  if (raw > BUDGET_BYTES) {
    throw new Error(`console.html+console.js+console.css are ${raw} bytes, ` +
      `${raw - BUDGET_BYTES} past the ${BUDGET_BYTES}-byte budget (60 KB uncompressed)`);
  }
  if (gz > BUDGET_GZIP) {
    throw new Error(`the three files gzip to ${gz} bytes, ` +
      `${gz - BUDGET_GZIP} past the ${BUDGET_GZIP}-byte budget (20 KB gzip)`);
  }
});

// ---------------------------------------------------------- the palette
//
// The console's own no-hex audit, the census's rule over this face: a
// literal colour may live in the token block and nowhere else. Every
// component rule names a token.

check('no hex lives outside the console’s token block', () => {
  const withoutComments = CSS.replace(/\/\*[\s\S]*?\*\//g, '');
  const lines = withoutComments.split('\n');
  let inside = false;
  const allowed = new Set();
  lines.forEach((line, index) => {
    const trimmed = line.trim();
    if (trimmed === ':root {') { inside = true; allowed.add(index); return; }
    if (inside) {
      allowed.add(index);
      if (trimmed === '}') inside = false;
    }
  });
  const offenders = [];
  lines.forEach((line, index) => {
    if (allowed.has(index)) return;
    const declaration = line.slice(line.indexOf(':') + 1);
    const found = declaration.match(/#[0-9a-fA-F]{3,8}\b/g);
    if (found) offenders.push(`${index + 1}: ${found.join(' ')}`);
  });
  if (offenders.length) {
    throw new Error('hex literals outside the token block — name a token instead:\n' +
      offenders.join('\n'));
  }
  // And the palette is one block: a second `:root` would be a second home
  // for the same fact.
  equal((withoutComments.match(/^:root \{/gm) || []).length, 1, 'one token block');
});

// ---------------------------------------------------------------- verdict

await Promise.all(pending);
console.log('');
if (failed.length) {
  console.log(`${failed.length} check${failed.length === 1 ? '' : 's'} failed:`);
  failed.forEach((name) => console.log('  - ' + name));
  process.exitCode = 1;
} else {
  console.log(`${passed} checks passed`);
}
