// fleet bot — the remote face's app (the 2026-10-20 plan's B4).
//
// Assistant-first: the persona is the axis, the goal is the invariant,
// and only destructive acts take the seat (R1/R4/R5). The client is a
// projection of the bridge's documents — /api/inbox for the list,
// /api/session + /api/state for one conversation, /events for the live
// channel — and it invents no second state machine: the state words it
// renders are the archive's own, mapped once server-side (D5).
'use strict';

// ---------- the relay transport (the console's app.js is its twin) ------
//
// When this page was reached through the bot's relay — the entry URL
// carries the relay secret as a fragment, the part of a URL browsers
// never send — every content-bearing request is sealed end to end and
// names the relay in a header. The seal is the browser's own WebCrypto:
// an X25519 exchange through the relay (the relay holds no scalar) and
// AES-256-GCM over the derived key. On the LAN face there is no stored
// secret and every line here is inert.

const RELAY_KEY_STORE = 'fleet_relay_k';

// The paths a browser fetches on its own — the bot shell's install set
// and the pairing form. They ride as plaintext frames of public code.
const RELAY_OPEN_PATHS = {
  '/': 1, '/bot.css': 1, '/bot.js': 1, '/bot.webmanifest': 1,
  '/bot-sw.js': 1, '/bot-icon-192.png': 1, '/bot-icon-512.png': 1, '/pair': 1,
};
const RELAY_HANDSHAKE_PATH = '/api/relay/handshake';

function relaySecretOf() {
  try { return localStorage.getItem(RELAY_KEY_STORE) || ''; } catch { return ''; }
}

function bytesToB64(bytes) {
  let text = '';
  for (let i = 0; i < bytes.length; i++) text += String.fromCharCode(bytes[i]);
  return btoa(text);
}

function b64ToBytes(text) {
  const raw = atob(text);
  const out = new Uint8Array(raw.length);
  for (let i = 0; i < raw.length; i++) out[i] = raw.charCodeAt(i);
  return out;
}

function concatBytes(left, right) {
  const out = new Uint8Array(left.length + right.length);
  out.set(left, 0);
  out.set(right, left.length);
  return out;
}

// The entry URL's fragment is the secret. It exists on the screen that
// showed the QR and in this page's storage after this line — never on
// a wire.
(() => {
  const match = window.location.hash.match(/^#k=(.+)$/);
  if (match) {
    try { localStorage.setItem(RELAY_KEY_STORE, match[1]); } catch { /* storage refused; the face stays unsealed */ }
    history.replaceState(null, '', location.pathname + location.search);
  }
})();

const RELAY_SECRET = relaySecretOf();
let RELAY_READY = null;

// The exchange: GET the device's public half, derive the shared secret
// against a scalar born here, POST the phone's public half. Both sides
// derive the same key — HKDF over the shared secret, salted with the
// session id the handshake names. Flat on purpose: a chain this deep
// is a paren counting exercise, and async is the seam the bot already
// speaks.
async function relayHandshake(secret) {
  const headers = { 'x-fleet-relay': secret };
  const setup = (async () => {
    const asked = await fetch(RELAY_HANDSHAKE_PATH, { headers, cache: 'no-store' });
    if (!asked.ok) throw new Error(`the handshake did not answer: ${asked.status}`);
    const doc = await asked.json();
    let pair;
    try {
      pair = await crypto.subtle.generateKey({ name: 'X25519' }, true, ['deriveBits']);
    } catch {
      throw new Error('this browser cannot seal its frames (X25519 is unavailable); update iOS and reload');
    }
    const deviceKey = await crypto.subtle.importKey('raw', b64ToBytes(doc.public), { name: 'X25519' }, true, []);
    const shared = await crypto.subtle.deriveBits({ name: 'X25519', public: deviceKey }, pair.privateKey, 256);
    const hk = await crypto.subtle.importKey('raw', shared, 'HKDF', false, ['deriveBits']);
    const keyBits = await crypto.subtle.deriveBits(
      { name: 'HKDF', hash: 'SHA-256', salt: new TextEncoder().encode(doc.sid), info: new TextEncoder().encode('fleet relay frame v1') },
      hk, 256,
    );
    const key = await crypto.subtle.importKey('raw', keyBits, 'AES-GCM', false, ['encrypt', 'decrypt']);
    const publicRaw = await crypto.subtle.exportKey('raw', pair.publicKey);
    const posted = await fetch(RELAY_HANDSHAKE_PATH, {
      method: 'POST',
      headers: Object.assign({ 'Content-Type': 'application/json' }, headers),
      body: JSON.stringify({ public: bytesToB64(new Uint8Array(publicRaw)) }),
    });
    if (!posted.ok) throw new Error(`the handshake was refused: ${posted.status}`);
    return { key };
  })();
  setup
    .then(() => document.documentElement.setAttribute('data-relay-sealed', '1'))
    .catch(() => {});
  RELAY_READY = setup;
  return setup;
}

function openSealedReply(key, reply) {
  if (reply.headers.get('x-fleet-enc') !== '1') return Promise.resolve(reply);
  const frameId = reply.headers.get('x-fleet-frame') || '';
  return reply.arrayBuffer().then((buffer) => {
    const raw = new Uint8Array(buffer);
    return crypto.subtle.decrypt(
      { name: 'AES-GCM', iv: raw.slice(0, 12), additionalData: new TextEncoder().encode(frameId), tagLength: 128 },
      key, raw.slice(12),
    ).then((plain) => {
      const headers = new Headers();
      reply.headers.forEach((value, name) => {
        if (name.toLowerCase() !== 'content-length') headers.set(name, value);
      });
      return new Response(new TextDecoder().decode(plain), {
        status: reply.status, statusText: reply.statusText, headers,
      });
    });
  });
}

if (RELAY_SECRET && typeof window.fetch === 'function') {
  document.documentElement.setAttribute('data-relay', '1');
  relayHandshake(RELAY_SECRET);
  const RAW_FETCH = window.fetch.bind(window);
  window.fetch = function (input, init) {
    init = init || {};
    const asked = typeof input === 'string' ? input : input.url;
    const url = new URL(asked, location.href);
    if (url.origin !== location.origin) return RAW_FETCH(input, init);
    const bare = url.pathname;
    const method = (init.method || (typeof input === 'object' && input.method) || 'GET').toUpperCase();
    if (method === 'GET' && RELAY_OPEN_PATHS[bare]) return RAW_FETCH(input, init);
    const named = Object.assign({}, init.headers || {});
    if (init.headers instanceof Headers) {
      init.headers.forEach((value, name) => { named[name] = value; });
    } else if (typeof input === 'object' && input.headers instanceof Headers) {
      input.headers.forEach((value, name) => { named[name] = value; });
    }
    named['x-fleet-relay'] = RELAY_SECRET;
    if (bare === RELAY_HANDSHAKE_PATH) {
      return RAW_FETCH(url.href, Object.assign({}, init, { headers: named }));
    }
    return RELAY_READY.then((seal) => {
      named['x-fleet-enc'] = '1';
      const sendInit = Object.assign({}, init, { headers: named });
      const send = () => RAW_FETCH(url.href, sendInit).then((reply) => openSealedReply(seal.key, reply));
      const body = init.body;
      if (body == null) return send();
      const text = typeof body === 'string' ? body : String(body);
      const nonce = crypto.getRandomValues(new Uint8Array(12));
      const binding = bare + (url.search || '');
      return crypto.subtle.encrypt(
        { name: 'AES-GCM', iv: nonce, additionalData: new TextEncoder().encode(binding), tagLength: 128 },
        seal.key, new TextEncoder().encode(text),
      ).then((sealed) => {
        sendInit.body = bytesToB64(concatBytes(nonce, new Uint8Array(sealed)));
        return send();
      });
    });
  };
}

// ---------- the fleet seam: one fetch with the paired cookie ----------

async function api(path, options) {
  const response = await fetch(path, options);
  if (!response.ok && response.status !== 409) {
    throw new Error(`${path} answered ${response.status}`);
  }
  const body = await response.json().catch(() => ({}));
  if (body && body.error) throw new Error(body.error);
  return body;
}

const post = (path, payload) =>
  api(path, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload ?? {}),
  });

// ---------- the personas (D7/D10) ----------
// The roster and their tints. The tints are the console's lane palette;
// the ink is the value that clears AA on each disc in every theme — the
// mockups' measured table, carried here as the built app's constant.
const PERSONAS = {
  tower:     { role: 'runs the work',        tint: 'var(--accent)',  ink: 'var(--accent)' },
  scholar:   { role: 'knowledge work',       tint: 'var(--lane-0)',  ink: 'var(--lane-0)' },
  presenter: { role: 'presentations',        tint: 'var(--lane-3)',  ink: 'var(--lane-3)' },
  advocate:  { role: 'job hunting',          tint: 'var(--lane-0)',  ink: 'var(--lane-0)' },
  auditor:   { role: 'checks what changed',  tint: 'var(--lane-2)',  ink: 'var(--lane-2)' },
  clerk:     { role: 'keeps the ledger',     tint: 'var(--lane-1)',  ink: 'var(--lane-1)' },
};
// Per-theme inks that clear AA on the 12% disc: the paper values are
// darker than the hue, the dark themes' are their own. Matched to the
// mockups' measured table (README: "Identity without portfolios").
const INK = {
  paper:    { tower: '#2c58c1', scholar: '#0c6b64', presenter: '#b31757', advocate: '#47690a', auditor: '#994909', clerk: '#6d3ad8' },
  console:  { tower: '#6693ff', scholar: '#2dd4bf', presenter: '#fb7185', advocate: '#a3e635', auditor: '#fbbf24', clerk: '#a78bfa' },
  graphite: { tower: '#5e94ff', scholar: '#5ed2be', presenter: '#f07887', advocate: '#96d246', auditor: '#ebb43c', clerk: '#9e86f0' },
};

const initial = (name) => (name ? name.charAt(0) : 'f');

function mark(name, size, extra) {
  const persona = PERSONAS[name] || {};
  const theme = document.documentElement.dataset.theme || 'paper';
  const ink = (INK[theme] && INK[theme][name]) || '';
  const tint = persona.tint || '';
  const style =
    `--tint:${tint};--tint-ink:${ink};` +
    `background:color-mix(in srgb, ${tint || 'var(--accent)'} 12%, var(--surface));` +
    `color:${ink || 'var(--accent)'};`;
  return `<span class="mark ${size}"${extra ? ` ${extra}` : ''} style="${style}">${initial(name)}</span>`;
}

const stack = (names, size) =>
  `<span class="stack">${names.map((n, i) => mark(n, size, i ? 'data-extra' : '')).join('')}</span>`;

// ---------- the state words: rendered from the server's mapping ----------
// /api/inbox's rows already carry the mapped word (D5); the client only
// decides what each looks like.
const AGE_WORDS = true;

function age(seconds) {
  if (seconds == null) return '';
  if (seconds < 60) return `${Math.max(1, Math.round(seconds))}s`;
  if (seconds < 3600) return `${Math.round(seconds / 60)}m`;
  if (seconds < 86400) return `${Math.round(seconds / 3600)}h`;
  return `${Math.round(seconds / 86400)}d`;
}

const $ = (html) => {
  const template = document.createElement('template');
  template.innerHTML = html.trim();
  return template.content.firstElementChild;
};

let toastTimer = null;
function toast(message) {
  document.querySelector('.toast')?.remove();
  const el = $(`<div class="toast"></div>`);
  el.textContent = message;
  document.getElementById('app').appendChild(el);
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => el.remove(), 4200);
}

// ---------- the app shell ----------

const app = document.getElementById('app');
const state = {
  view: 'inbox', // 'inbox' | 'conversation'
  filter: 'everyone',
  inbox: null,
  session: null, // the open conversation's id
  conversation: null, // the /api/session document
  live: null, // the /api/state document
  events: null, // the EventSource
  clock: null, // the working chip's timer
};

function render() {
  if (state.view === 'inbox') renderInbox();
  else renderConversation();
}

// ---------- the inbox ----------

async function loadInbox() {
  try {
    state.inbox = await api('/api/inbox');
    if (state.view === 'inbox') render();
  } catch (error) {
    toast(`the inbox could not be read: ${error.message}`);
  }
}

function personaCounts(inbox) {
  const counts = {};
  for (const row of inbox.sessions || []) {
    for (const name of row.personas || []) counts[name] = (counts[name] || 0) + 1;
  }
  return counts;
}

function renderInbox() {
  const inbox = state.inbox || { sessions: [], bound: 50 };
  const counts = personaCounts(inbox);
  const names = Object.keys(PERSONAS).filter((name) => counts[name]);
  const needs = (inbox.sessions || []).filter(
    (row) => row.state === 'seat' || row.state === 'failed',
  );
  const rows = (inbox.sessions || []).filter((row) => {
    if (state.filter === 'everyone') return true;
    return (row.personas || []).includes(state.filter);
  });

  app.innerHTML = `
    <header class="app-head">
      <div class="titles">
        <h1>fleet</h1>
        <p class="sub">${names.length} persona${names.length === 1 ? '' : 's'} · ${inbox.sessions.length} conversation${inbox.sessions.length === 1 ? '' : 's'}</p>
      </div>
      ${needs.length ? `<span class="count"><span class="dot"></span>${needs.length}</span>` : ''}
    </header>
    ${needs.length ? `
      <div class="waiting">
        <div class="bar">Needs you · ${needs.length}</div>
        ${needs.map((row) => `
          <button class="row ${row.state}" data-session="${row.id}">
            ${stack(row.personas?.slice(0, 1) || [], 'sm')}
            <span class="title">${escapeHtml(row.goal || rowTitle(row))}</span>
            <span class="age">${age(row.at)}</span>
          </button>`).join('')}
      </div>` : ''}
    <nav class="persona-rail" aria-label="Personas">
      <button class="persona" data-persona="everyone" aria-pressed="${state.filter === 'everyone'}">
        ${mark('everyone', 'sm')}Everyone ${names.length ? `<span class="n">${inbox.sessions.length}</span>` : ''}
      </button>
      ${names.map((name) => `
        <button class="persona" data-persona="${name}" aria-pressed="${state.filter === name}">
          ${mark(name, 'sm')}${name} <span class="n">${counts[name]}</span>
        </button>`).join('')}
    </nav>
    <div class="scroll">
      ${rows.length ? groupedRows(rows) : `
        <div class="empty">
          ${mark('everyone', 'lg')}
          <h2>Nothing running.</h2>
          <p>Dispatch a task and it lands here with its persona, its goal, and the last thing it said.</p>
        </div>`}
      <div style="height:14px"></div>
    </div>
    <div class="dock">
      <button class="dispatch" id="dispatch">
        <span class="grow">say something to fleet…</span>
        <span class="send">${ICON_ARROW}</span>
      </button>
    </div>`;

  for (const button of app.querySelectorAll('[data-persona]')) {
    button.addEventListener('click', () => {
      state.filter = button.dataset.persona;
      render();
    });
  }
  for (const row of app.querySelectorAll('[data-session]')) {
    row.addEventListener('click', () => openConversation(row.dataset.session));
  }
  app.querySelector('#dispatch').addEventListener('click', renderDispatchSheet);
}

function rowTitle(row) {
  return row.last || rowTitleFallback(row);
}

const rowTitleFallback = (row) => (row.goal ? `${row.personas?.[0] || 'fleet'} is working` : 'a new conversation');

function groupedRows(rows) {
  const groups = new Map();
  for (const row of rows) {
    const lead = (row.personas && row.personas[0]) || '';
    const list = groups.get(lead) || [];
    list.push(row);
    groups.set(lead, list);
  }
  const parts = [];
  for (const [lead, list] of groups) {
    const persona = PERSONAS[lead];
    parts.push(`
      <div class="section-head">${lead ? `<span>${escapeHtml(lead)}</span>` : '<span>fleet</span>'}
        ${persona ? `<span class="role">${escapeHtml(persona.role)}</span>` : ''}
        <span class="n">${list.length}</span>
      </div>
      <div class="rows">${list.map(rowHtml).join('')}</div>`);
  }
  return parts.join('');
}

function rowHtml(row) {
  const names = row.personas || [];
  return `
    <button class="row ${row.state}" data-session="${row.id}">
      ${stack(names.slice(0, 2).length ? names.slice(0, 2) : [], 'sm')}
      <span>
        <p class="title">${escapeHtml(rowTitle(row))}</p>
        <p class="last ${row.refused ? 'refused' : ''}">${escapeHtml(row.refused ? `refused · ${row.refused}` : row.last || (row.goal ? `serves: ${row.goal}` : 'no words yet'))}</p>
      </span>
      <span class="age"><span class="state-dot"></span>${age(row.at)}</span>
    </button>`;
}

// ---------- the conversation ----------

async function openConversation(id) {
  state.view = 'conversation';
  state.session = id;
  state.live = null;
  state.conversation = null;
  render();
  try {
    const [conversation, live] = await Promise.all([
      api(`/api/session?id=${encodeURIComponent(id)}`),
      api(`/api/state?session=${encodeURIComponent(id)}`),
    ]);
    state.conversation = conversation;
    state.live = live;
    render();
  } catch (error) {
    toast(`the conversation could not be opened: ${error.message}`);
  }
  listen(id);
}

function listen(id) {
  // The relayed stream is a poll, not a subscription: a frame is a
  // whole reply, so the device answers /events with the session's
  // state and the conversation refreshes on a clock — the same shape
  // the console's face runs. An EventSource through the relay would be
  // a held-open connection the transport does badly and a header it
  // cannot present.
  if (document.documentElement.getAttribute('data-relay') === '1') {
    state.events?.close();
    state.events = { close: () => clearInterval(state.pollTimer) };
    const tick = () => {
      if (state.view === 'conversation' && state.session === id) refreshConversation(id);
    };
    tick();
    state.pollTimer = setInterval(tick, 2500);
    return;
  }
  state.events?.close();
  const source = new EventSource('/events');
  state.events = source;
  source.onmessage = (event) => {
    let payload;
    try { payload = JSON.parse(event.data); } catch { return; }
    if (payload.session && payload.session !== id) return;
    if (state.view !== 'conversation') return;
    // The events carry the log's records; the transcript re-renders
    // from the session document, which the server rebuilds — one
    // derivation, not two.
    refreshConversation(id);
  };
  source.onerror = () => { /* the 5s poll below is the floor */ };
}

let refreshTimer = null;
async function refreshConversation(id) {
  clearTimeout(refreshTimer);
  refreshTimer = setTimeout(async () => {
    if (state.view !== 'conversation' || state.session !== id) return;
    try {
      state.conversation = await api(`/api/session?id=${encodeURIComponent(id)}`);
      state.live = await api(`/api/state?session=${encodeURIComponent(id)}`);
      render();
    } catch { /* the poll retries; the toast is for actions */ }
  }, 250);
}

function goalOf() {
  const fromState = state.live?.goal;
  return fromState || state.inbox?.sessions?.find((r) => r.id === state.session)?.goal || '';
}

function participantsOf() {
  const fromState = state.live?.personas;
  return fromState?.length ? fromState
    : state.inbox?.sessions?.find((r) => r.id === state.session)?.personas || [];
}

function seatOf() {
  return state.live?.approval || null;
}

const runState = () => {
  const live = state.live || {};
  if (live.approval) return 'seat';
  if (live.running) return 'running';
  return 'idle';
};

function renderConversation() {
  const conversation = state.conversation;
  const view = document.createElement('div');
  view.className = 'conv';
  view.dataset.state = runState();

  const personas = participantsOf();
  const goal = goalOf();
  const seat = seatOf();
  const running = runState() === 'running';

  view.innerHTML = `
    <header class="app-head">
      <button class="act" id="back" aria-label="Back to the inbox">${ICON_BACK}</button>
      ${personas.length ? stack(personas.slice(0, 2), 'md') : mark('everyone', 'md')}
      <div class="titles">
        <h1 id="who">${escapeHtml(personas.length > 1 ? personas.join(' + ') : personas[0] || 'fleet')}</h1>
        <p class="head-state"><span class="dot"></span><span id="headstate">${escapeHtml(headState())}</span></p>
      </div>
      <button class="act" id="add" aria-label="Add a persona to this conversation" style="width:44px">${ICON_PLUS}</button>
    </header>
    ${goal ? `<p class="goal"><b>goal</b><span class="text">${escapeHtml(goal)}</span></p>` : ''}
    <div class="scroll" id="transcript">${transcriptHtml(conversation)}</div>
    <div class="dock">
      ${running ? `
        <div class="working">
          <span class="pulse"></span>
          <span class="what">${escapeHtml(state.live?.driving ? 'the run is in flight' : 'working')}</span>
          <span class="clock" id="clock"></span>
          <button class="stop" id="stop">Stop</button>
        </div>` : ''}
      ${seat ? seatHtml(seat) : ''}
      <div class="composer">
        <textarea id="steer" rows="1" enterkeyhint="send" placeholder="${running ? 'steer or answer…' : `say something to ${personas[0] || 'fleet'}…`}"></textarea>
        <button class="send ${running ? 'stop' : ''}" id="send" aria-label="${running ? 'Stop the run' : 'Send'}">${running ? ICON_STOP : ICON_ARROW}</button>
      </div>
    </div>`;

  app.innerHTML = '';
  app.appendChild(view);

  view.querySelector('#back').addEventListener('click', () => {
    state.events?.close();
    state.events = null;
    state.view = 'inbox';
    state.session = null;
    render();
    loadInbox();
  });
  view.querySelector('#add').addEventListener('click', renderAddSheet);
  view.querySelector('#stop')?.addEventListener('click', async () => {
    try { await post('/api/stop'); } catch (error) { toast(error.message); }
  });
  const textarea = view.querySelector('#steer');
  const send = view.querySelector('#send');
  const submit = async () => {
    const text = textarea.value.trim();
    if (!text) return;
    textarea.value = '';
    try {
      if (running) {
        await post('/api/steer', { text });
      } else {
        await post(`/api/run?session=${encodeURIComponent(state.session)}`, {
          task: text,
          mode: 'yes-guarded', // R5: non-destructive runs; destructive is held
        });
      }
      refreshConversation(state.session);
    } catch (error) {
      toast(error.message);
    }
  };
  send.addEventListener('click', submit);
  textarea.addEventListener('keydown', (event) => {
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault();
      submit();
    }
  });
  answerSeat(seat);
  // The working chip's clock, from the run's start stamp if the state
  // carries one; a chip without a clock is a chip without a story.
  if (running) startClock(view.querySelector('#clock'));
}

function headState() {
  const live = state.live || {};
  if (live.approval) return live.approval.destructive ? 'destructive · held for you' : 'held for you';
  if (live.running) return 'working';
  return 'idle';
}

function startClock(el) {
  if (!el) return;
  const t0 = Date.now();
  const tick = () => {
    if (!el.isConnected) { clearInterval(state.clock); return; }
    const seconds = Math.max(0, Math.round((Date.now() - t0) / 1000));
    el.textContent = `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, '0')}`;
  };
  clearInterval(state.clock);
  tick();
  state.clock = setInterval(tick, 1000);
}

function transcriptHtml(conversation) {
  if (!conversation) {
    return `<div class="empty"><div class="mark lg" style="margin:0 auto">f</div><p>opening the conversation…</p></div>`;
  }
  const cards = conversation.cards || [];
  const parts = [];
  let sawOwner = false;
  for (const card of cards) {
    switch (card.type) {
      case 'message':
        if (card.role === 'owner') {
          sawOwner = true;
          parts.push(`<div class="turn you"><div class="bubble">${escapeHtml(card.text)}</div></div>`);
        } else if (card.text) {
          const author = card.persona || personaFor();
          parts.push(`
            <div class="turn fleet">
              <div class="byline">${mark(author, 'sm')}<span>${escapeHtml(author)}</span></div>
              <p>${escapeHtml(card.text)}</p>
            </div>`);
        }
        break;
      case 'tool_call':
        parts.push(`
          <div class="turn fleet" data-tool="1">
            <div class="tools"><div class="tool">
              <span class="done">✓</span>
              <span><span class="name">${escapeHtml(card.tool)}</span> ${escapeHtml(card.summary || '')}</span>
            </div></div>
          </div>`);
        break;
      case 'join':
        parts.push(`
          <div class="join">${mark(card.persona, 'sm')}
            <span><b>${escapeHtml(card.persona)}</b> joined${card.role ? ` · ${escapeHtml(card.role)}` : ''}</span>
          </div>`);
        break;
      case 'plan':
        parts.push(`
          <div class="turn fleet">
            <div class="byline">${mark(personaFor(), 'sm')}<span>plan</span></div>
            <p>${escapeHtml(card.text)}</p>
          </div>`);
        break;
      case 'reasoning':
      case 'channel':
      case 'rewound':
        break; // the bot's transcript is the conversation, not the audit
      default:
        break;
    }
  }
  if (!parts.length) {
    return `<div class="empty"><div class="mark lg" style="margin:0 auto">f</div><p>no words yet — say something below.</p></div>`;
  }
  return parts.join('') + '<div style="height:8px"></div>';
}

// A turn's persona: the replay's records do not name a per-turn author
// yet, so the client shows the lead — the server, not the client, is
// where a per-turn persona will be stamped when P2's stream work lands.
const personaFor = () => participantsOf()[0] || 'everyone';

// ---------- the seat (R5) ----------

function seatHtml(seat) {
  const destructive = Boolean(seat.destructive);
  const actions = destructive
    ? `<button class="btn" data-decision="deny">Cancel</button>
       <button class="btn danger" data-decision="approve">Overwrite</button>`
    : `<button class="btn primary" data-decision="approve">Allow once</button>
       ${seat.allowlistable ? `<button class="btn" data-decision="always">Always</button>` : ''}
       <button class="btn" data-decision="deny">Deny</button>`;
  return `
    <div class="seat-card ${destructive ? 'destructive' : ''}" id="seat">
      <div class="head">
        <span class="badge ${destructive ? 'destructive' : ''}">${destructive ? '⚠ destructive' : 'approval'}</span>
        <span class="target">${escapeHtml(seat.tool || '')}</span>
      </div>
      <p class="summary">${escapeHtml(seat.summary || '')}</p>
      <div class="seat-actions">${actions}</div>
      <p class="seat-note ${destructive ? 'destructive' : ''}">
        ${destructive
          ? 'Held every time — there is no always-allow for a destructive act. Non-destructive work does not ask.'
          : 'Non-destructive work runs without asking; this one asked because of its rule.'}
      </p>
    </div>`;
}

function answerSeat(seat) {
  if (!seat) return;
  for (const button of app.querySelectorAll('[data-decision]')) {
    button.addEventListener('click', async () => {
      const decision = button.dataset.decision;
      try {
        await post('/api/approve', { name: seat.name, decision });
        refreshConversation(state.session);
      } catch (error) {
        toast(error.message);
      }
    });
  }
}

// ---------- the sheets ----------

function sheetShell(title, body) {
  const el = $(`
    <div>
      <div class="scrim"></div>
      <div class="sheet">
        <div class="grab"></div>
        <div class="sheet-head">
          <h2>${escapeHtml(title)}</h2>
          <button class="act" data-close aria-label="Close" style="width:44px;height:44px">${ICON_CLOSE}</button>
        </div>
        <div class="sheet-body">${body}</div>
      </div>
    </div>`);
  el.querySelector('[data-close]').addEventListener('click', () => { el.remove(); render(); });
  el.querySelector('.scrim').addEventListener('click', () => { el.remove(); render(); });
  app.appendChild(el);
  return el;
}

function renderAddSheet() {
  const goal = goalOf();
  const here = new Set(participantsOf());
  const el = sheetShell('Add a persona', `
    <p class="policy">Adding someone does not change the goal${goal ? ` — <b>${escapeHtml(goal)}</b>` : ''}. <b>Non-destructive work runs without asking.</b> Only destructive acts are held for you, and they are flagged as such.</p>
    ${Object.entries(PERSONAS).filter(([name]) => !here.has(name)).map(([name, persona]) => `
      <button class="pick" data-persona="${name}" aria-pressed="false">
        ${mark(name, 'md')}
        <span><span class="n">${name}</span><div class="d">${escapeHtml(persona.role)}</div></span>
      </button>`).join('')}
    <button class="primary" id="join" disabled>Add to this conversation</button>`);

  let chosen = null;
  for (const pick of el.querySelectorAll('[data-persona]')) {
    pick.addEventListener('click', () => {
      for (const other of el.querySelectorAll('[data-persona]')) other.setAttribute('aria-pressed', 'false');
      pick.setAttribute('aria-pressed', 'true');
      chosen = pick.dataset.persona;
      const join = el.querySelector('#join');
      join.disabled = false;
      join.textContent = `Add ${chosen} to this conversation`;
    });
  }
  el.querySelector('#join').addEventListener('click', async () => {
    if (!chosen) return;
    try {
      await post('/api/session/join', { session: state.session, persona: chosen });
      el.remove();
      refreshConversation(state.session);
    } catch (error) {
      toast(error.message);
    }
  });
}

function renderDispatchSheet() {
  const el = sheetShell('New conversation', `
    <span class="label">Who takes it</span>
    <div class="persona-rail" style="padding:0 0 12px">
      ${Object.entries(PERSONAS).map(([name, persona], index) => `
        <button class="persona" data-persona="${name}" aria-pressed="${index === 0}">
          ${mark(name, 'sm')}${name}
        </button>`).join('')}
    </div>
    <span class="label">The goal — the one thing this conversation serves</span>
    <div class="ask">
      <textarea id="brief" rows="1" enterkeyhint="send" placeholder="what should it do?"></textarea>
    </div>
    <button class="primary" id="create" disabled>Start the conversation</button>`);

  let persona = Object.keys(PERSONAS)[0];
  for (const pick of el.querySelectorAll('[data-persona]')) {
    pick.addEventListener('click', () => {
      for (const other of el.querySelectorAll('[data-persona]')) other.setAttribute('aria-pressed', 'false');
      pick.setAttribute('aria-pressed', 'true');
      persona = pick.dataset.persona;
    });
  }
  const brief = el.querySelector('#brief');
  const create = el.querySelector('#create');
  brief.addEventListener('input', () => { create.disabled = !brief.value.trim(); });
  create.addEventListener('click', async () => {
    const goal = brief.value.trim();
    if (!goal) return;
    try {
      const made = await post('/api/session/new', { persona, goal });
      el.remove();
      await loadInbox();
      openConversation(made.id);
    } catch (error) {
      toast(error.message);
    }
  });
}

// ---------- the shared bits ----------

const escapeHtml = (text) =>
  String(text ?? '').replace(/[&<>"']/g, (ch) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  })[ch]);

const ICON_ARROW = `<svg width="17" height="17" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" aria-hidden="true"><path d="M10 15.5V4.5M10 4.5 5.5 9M10 4.5l4.5 4.5"/></svg>`;
const ICON_BACK = `<svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12.4 4.5 6 10.5l6.4 6"/></svg>`;
const ICON_PLUS = `<svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" aria-hidden="true"><path d="M10 4.5v11M4.5 10h11"/></svg>`;
const ICON_STOP = `<svg width="15" height="15" viewBox="0 0 20 20" fill="currentColor" aria-hidden="true"><rect x="5.5" y="5.5" width="9" height="9" rx="1.6"/></svg>`;
const ICON_CLOSE = `<svg width="17" height="17" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" aria-hidden="true"><path d="m5 5 10 10M15 5 5 15"/></svg>`;

// ---------- the boot: the inbox, the events, the 5s floor ----------

loadInbox();
setInterval(() => {
  if (state.view === 'inbox') loadInbox();
}, 5000);
