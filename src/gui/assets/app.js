// Fleet's browser face: the client.
//
// One event log drives everything. The server's `/api/state` is the
// replay and `/events` is the live half of the same log, so a refresh
// and a stream are the same derivation run twice — there is no second
// place a figure can come from, which is why the page cannot disagree
// with itself. Every number in the header (spend, context, route,
// turns, the stop reason) is reduced out of the log rather than
// reported, and the inspector shows the reduction beside the event it
// came from.
//
// The launch token rides the stream's URL once, is stripped from the
// address bar, and never reaches this script again: the HttpOnly
// cookie the page arrived with carries it for the requests that follow.
(function () {
  'use strict';

  var params = new URLSearchParams(window.location.search);
  var token = params.get('token');

  // The plan's route table. The addresses are hash addresses and the
  // panels are keyed by them, so the browser never asks the server for
  // a page and a surface that is not built yet can say so honestly.
  var ROUTES = [
    ['chat', 'the conversation'],
    ['router', 'router'],
    ['delegation', 'delegation'],
    ['memory', 'memory'],
    ['skills', 'skills'],
    ['sessions', 'sessions'],
    ['calibration', 'calibration'],
    ['settings', 'settings'],
    ['tower', 'tower'],
    ['second-brain', 'second brain']
  ];

  // The same ten surfaces, grouped by what the owner came to do rather
  // than by subsystem. ONE declaration: the rail is built from it and
  // the palette searches it, so there is a single structure and nothing
  // to keep in sync between the two. The research is explicit that no
  // numeric cap on destinations is defensible — what it supports is
  // chunking, one flat level, and never a second tier, which is what
  // this is: four groups, one level beneath each, and the palette as
  // the parallel path that makes the grouping a convenience rather than
  // a maze.
  var GROUPS = [
    ['work', 'work', [
      ['chat', 'conversation'],
      ['sessions', 'sessions']
    ]],
    ['intelligence', 'intelligence', [
      ['router', 'router'],
      ['delegation', 'delegation']
    ]],
    ['knowledge', 'knowledge', [
      ['memory', 'memory'],
      ['skills', 'skills'],
      ['second-brain', 'second brain']
    ]],
    ['system', 'system', [
      ['settings', 'settings'],
      ['calibration', 'calibration'],
      ['tower', 'tower']
    ]]
  ];

  // A short mark per destination, so the collapsed rail still reads at a
  // glance. The house rule for glyphs: a mark is always the surface's own
  // first letter rather than an icon set shipped beside the binary.
  var GROUP_MARKS = { work: 'w', intelligence: 'i', knowledge: 'k', system: 's' };

  // Which group an address belongs to, and its label in that group —
  // derived once so the rail, the palette and the panel all agree.
  var WHERE = {};
  GROUPS.forEach(function (group) {
    group[2].forEach(function (item) { WHERE[item[0]] = [group[0], item[1]]; });
  });

  var el = {
    rail: document.getElementById('rail'),
    railToggle: document.getElementById('rail-toggle'),
    palette: document.getElementById('palette'),
    paletteScrim: document.getElementById('palette-scrim'),
    paletteInput: document.getElementById('palette-input'),
    paletteRows: document.getElementById('palette-rows'),
    paletteCount: document.getElementById('palette-count'),
    paletteOpen: document.getElementById('palette-open'),
    themeToggle: document.getElementById('theme-toggle'),
    panelChat: document.getElementById('panel-chat'),
    panelRouter: document.getElementById('panel-router'),
    panelDelegation: document.getElementById('panel-delegation'),
    panelMemory: document.getElementById('panel-memory'),
    panelSkills: document.getElementById('panel-skills'),
    panelLater: document.getElementById('panel-later'),
    laterTitle: document.getElementById('later-title'),
    laterNote: document.getElementById('later-note'),
    routerBoard: document.getElementById('router-board'),
    delegationBoard: document.getElementById('delegation-board'),
    memoryBoard: document.getElementById('memory-board'),
    skillsBoard: document.getElementById('skills-board'),
    routePill: document.getElementById('route-pill'),
    statePill: document.getElementById('state-pill'),
    spend: document.getElementById('spend'),
    context: document.getElementById('context'),
    contextMeter: document.getElementById('context-meter'),
    contextFill: document.getElementById('context-fill'),
    log: document.getElementById('log'),
    empty: document.getElementById('empty'),
    offers: document.getElementById('offers'),
    seat: document.getElementById('seat'),
    seatLabel: document.getElementById('seat-label'),
    seatSummary: document.getElementById('seat-summary'),
    seatActions: document.getElementById('seat-actions'),
    seatNote: document.getElementById('seat-note'),
    composer: document.getElementById('composer'),
    task: document.getElementById('task'),
    send: document.getElementById('send'),
    hint: document.getElementById('hint'),
    facts: document.getElementById('facts'),
    rules: document.getElementById('rules'),
    ruleCount: document.getElementById('rule-count'),
    scrim: document.getElementById('scrim'),
    drawer: document.getElementById('drawer'),
    drawerTitle: document.getElementById('drawer-title'),
    drawerBody: document.getElementById('drawer-body'),
    drawerClose: document.getElementById('drawer-close'),
    inspectFace: document.getElementById('inspect-face')
  };

  // ------------------------------------------------------------ the face

  // Everything the header and the rail derive, in one place. `rows` is
  // the transcript; `events` is the raw log the inspector reads.
  var face = {
    session: '',
    model: '',
    budget: 0,
    spend: 0,
    context: 0,
    turn: 0,
    route: null,
    stop: null,
    running: false,
    dropped: 0,
    seat: null,
    seatKey: '',
    rules: [],
    events: [],
    rows: [],
    stream: null,
    assistant: null,
    pending: [],
    delegations: {},
    lastTurn: 0,
    stick: true,
    buffer: '',
    flush: 0,
    // Which keyed panel is showing, and the router board's last read. The
    // read is the board's document; the live decision is not in it, and
    // the panel keeps the two apart.
    panel: 'chat',
    router: null,
    routerAt: 0,
    // The delegation panel's read, and when it last landed. It is the
    // session logs and the graph file, so it moves as a run delegates
    // and as the plan's work declares nodes.
    delegation: null,
    delegationAt: 0,
    // The memory panel's read, its clock, and the query the search box
    // holds. The corpus is one read; the search is the panel's own.
    memory: null,
    memoryAt: 0,
    query: '',
    // The skills panel's read and its clock.
    skills: null,
    skillsAt: 0,
    // The palette's rows and which of them is selected, so the arrow keys
    // move a cursor the same way the panel reads move theirs.
    paletteRows: [],
    paletteAt: 0
  };

  // A short mark for a tool, so a card reads at a glance without an
  // icon set shipped beside the binary. The first letter is the honest
  // fallback: it is always the tool's own.
  var GLYPHS = {
    bash: '>', read: '<', write: 'w', edit: 'e', search: '?',
    recall: 'm', skill: 's', task: '*', delegate: '*', web: '@'
  };

  var OFFERS = [
    'walk the router\u2019s chain for a small task and report what it did',
    'what do the memories say about the fleet router',
    'read the plan and tell me what the next increment owes',
    'summarise this repository in five lines'
  ];

  // ----------------------------------------------------------- formatting

  var whole = new Intl.NumberFormat('en-US');

  // Tokens climb past five figures fast, and a row of them must not
  // shuffle: tabular numerals in the CSS, compact notation here.
  function tokens(n) {
    n = Number(n) || 0;
    if (n < 10000) return whole.format(n);
    if (n < 1000000) return (n / 1000).toFixed(n < 100000 ? 1 : 0).replace(/\.0$/, '') + 'k';
    return (n / 1000000).toFixed(1).replace(/\.0$/, '') + 'M';
  }

  // ------------------------------------------------------------- the rows

  function node(name, className, text) {
    var made = document.createElement(name);
    if (className) made.className = className;
    if (text != null) made.textContent = text;
    return made;
  }

  function stick() {
    return el.log.scrollHeight - el.log.scrollTop - el.log.clientHeight < 90;
  }

  function settle() {
    if (face.stick) el.log.scrollTop = el.log.scrollHeight;
  }

  // A card row: the one shape every non-prose thing on the face takes.
  // Clicking it opens the inspector on the event that made it, which is
  // why every row carries its own event.
  function card(kind, glyph, title, sub, value, label) {
    var el0 = node('div', 'card-row');
    el0.dataset.kind = kind;
    var button = node('button', 'card-open');
    button.type = 'button';
    button.appendChild(node('span', 'glyph', glyph));
    var body = node('span', 'body');
    body.appendChild(node('span', 'title', title));
    if (sub) body.appendChild(node('span', 'sub', sub));
    button.appendChild(body);
    if (value != null) button.appendChild(node('span', 'value', value));
    if (label) button.appendChild(node('span', 'label', label));
    el0.appendChild(button);
    var note = node('p', 'note');
    note.hidden = true;
    el0.appendChild(note);
    return el0;
  }

  function row(el0, event, update) {
    var made = { el: el0, event: event, update: update || null };
    face.rows.push(made);
    face.stream.appendChild(el0);
    return made;
  }

  function ownerRow(event) {
    var el0 = node('div', 'owner');
    el0.appendChild(node('span', 'owner-tag', 'you'));
    el0.appendChild(node('span', 'owner-text', event.text));
    return row(el0, event);
  }

  function turnRow(event) {
    var el0 = node('div', 'turn-mark', 'turn ' + event.turn);
    return row(el0, event);
  }

  function assistantRow() {
    var el0 = node('div', 'assistant streaming');
    return row(el0, null, function (text) {
      el0.textContent += text;
    });
  }

  function toolRow(event) {
    var status = node('span', 'status', 'running');
    var el0 = card('tool', GLYPHS[event.tool] || event.tool.charAt(0).toUpperCase(),
      event.tool, event.summary, null, null);
    el0.classList.add('tool');
    el0.dataset.tool = event.tool;
    el0.dataset.status = 'running';
    el0.querySelector('.body').appendChild(status);
    var made = row(el0, event, function (result) {
      el0.dataset.status = result.approved ? 'ok' : 'refused';
      status.textContent = result.approved ? 'done' : 'refused';
      if (result.preview) {
        var note = el0.querySelector('.note');
        note.textContent = result.preview;
        note.hidden = false;
      }
      made.event = { type: 'tool_result', tool: result.tool, preview: result.preview, approved: result.approved };
    });
    face.pending.push(made);
    return made;
  }

  function routeRow(event) {
    var el0 = card('route', '~', 'route', event.how, event.provider, 'served');
    return row(el0, event);
  }

  function memoryRow(event) {
    var el0 = card('memory', 'm', 'memory', event.shape, String(event.facts), 'facts');
    return row(el0, event);
  }

  function skillsRow(event) {
    var el0 = card('skills', 's', 'skills',
      event.skipped && event.skipped.length
        ? 'skipped: ' + event.skipped.join(', ')
        : 'all of the library',
      String(event.loaded), 'loaded');
    return row(el0, event);
  }

  function delegationRow(event) {
    var el0 = card('delegation', '*', event.goal, 'spawned', null, event.session);
    var made = row(el0, event, function (next) {
      var sub = el0.querySelector('.sub');
      if (next.stage === 'working') {
        sub.textContent = 'working \u00b7 ' + next.detail.calls + ' calls \u00b7 ' + next.detail.tool;
      } else if (next.stage === 'done') {
        sub.textContent = 'done \u00b7 ' + next.detail.outcome + ' \u00b7 ' + next.detail.turns + ' turns';
      } else {
        sub.textContent = 'spawned';
      }
      made.event = next;
    });
    return made;
  }

  function laneRow(event) {
    var el0 = card('lane', '\u2387', 'lane', event.lane, String(event.tools), 'tools');
    return row(el0, event);
  }

  function compactedRow(event) {
    var saved = Math.max(0, Number(event.before) - Number(event.after));
    var el0 = card('compacted', '\u2193', 'compaction',
      event.turns + ' turns \u00b7 ' + tokens(event.before) + ' \u2192 ' + tokens(event.after),
      '\u2212' + tokens(saved), 'tokens');
    return row(el0, event);
  }

  function cronRow(event) {
    var el0 = card('cron', '@', event.job,
      event.minute + (event.catch_up ? ' \u00b7 catch-up' : ''), event.outcome, 'fired');
    if (event.detail) {
      var note = el0.querySelector('.note');
      note.textContent = event.detail;
      note.hidden = false;
    }
    return row(el0, event);
  }

  function endRow(event) {
    var failed = event.type === 'failed';
    var el0 = card(failed ? 'failed' : 'finished', failed ? '!' : '\u2713',
      failed ? 'failed' : 'finished', event.reason, null, null);
    return row(el0, event);
  }

  // The log is a ring, and the ring let one go. The bridge says so once,
  // when it first happens; after that the rail's own dropped count is
  // the standing answer, which is why this row is not repeated.
  function droppedRow(event) {
    var el0 = card('dropped', '!', 'the log let an event go',
      'the face holds the last 4096; one fell off the front', String(event.dropped), 'missed');
    return row(el0, event);
  }

  // A row the client owes the owner and the server never sent: a run
  // the route refused, a rule the writer would not take. It is not in
  // the log the server replays and the inspector counts only what the
  // server sent — a page that invented its own history would be the one
  // place this face lies.
  function notice(title, text) {
    var el0 = card('notice', '!', title, text, null, null);
    face.stream.appendChild(el0);
    settle();
  }

  // ---------------------------------------------------------- the reducer

  // One event in, the face changed. Replay and live are this function
  // called once per event from either end, which is the whole reason
  // the two cannot drift.
  function apply(event) {
    face.events.push(event);
    switch (event.type) {
      case 'message':
        if (event.role === 'owner') { ownerRow(event); break; }
        break;
      case 'token':
        if (!face.assistant) face.assistant = assistantRow();
        face.buffer += event.text;
        schedule();
        break;
      case 'turn':
        if (event.turn !== face.lastTurn) {
          face.lastTurn = event.turn;
          face.turn = event.turn;
          turnRow(event);
        }
        break;
      case 'usage':
        face.spend += Number(event.prompt) + Number(event.completion);
        // The context meter is the last prompt against the budget, not
        // a running sum: a prompt is the whole conversation resent, and
        // the sum would double-count every turn.
        face.context = Number(event.prompt);
        break;
      case 'tool_call':
        face.assistant = null;
        toolRow(event);
        break;
      case 'tool_result':
        resolveTool(event);
        break;
      case 'routed':
        face.route = event;
        routeRow(event);
        break;
      case 'lane':
        laneRow(event);
        break;
      case 'memory':
        memoryRow(event);
        break;
      case 'skills':
        skillsRow(event);
        break;
      case 'delegation':
        applyDelegation(event);
        break;
      case 'compacted':
        compactedRow(event);
        break;
      case 'cron':
        cronRow(event);
        break;
      case 'finished':
      case 'failed':
        face.assistant = null;
        face.stop = event;
        endRow(event);
        break;
      case 'seat':
        face.seat = event;
        break;
      case 'seat_cleared':
        face.seat = null;
        break;
      case 'rules':
        face.rules = event.rules || [];
        break;
      case 'dropped':
        face.dropped = Number(event.dropped) || face.dropped;
        droppedRow(event);
        break;
      default:
        break;
    }
    paint();
  }

  // A result pairs with the oldest call of the same tool still waiting.
  // A lane can interleave two calls, and a stack would answer the wrong
  // one; the oldest unanswered is the one the agent just finished.
  function resolveTool(event) {
    for (var i = 0; i < face.pending.length; i += 1) {
      if (face.pending[i].el.dataset.tool === event.tool) {
        var made = face.pending.splice(i, 1)[0];
        made.update(event);
        return;
      }
    }
    var orphan = toolRow({ type: 'tool_call', tool: event.tool, summary: '' });
    orphan.update(event);
    face.pending.splice(face.pending.indexOf(orphan), 1);
  }

  // A child is one card for its whole life: spawned opens it, working
  // and done rewrite its line, keyed by the session the child owns.
  function applyDelegation(event) {
    var made = face.delegations[event.session];
    if (!made) {
      made = delegationRow(event);
      face.delegations[event.session] = made;
      return;
    }
    made.update(event);
  }

  // Streamed tokens land one at a time; the DOM is written once a
  // frame so a fast turn does not lay out per token.
  function schedule() {
    if (face.flush) return;
    face.flush = requestAnimationFrame(function () {
      face.flush = 0;
      if (!face.assistant) return;
      face.assistant.update(face.buffer);
      face.buffer = '';
      settle();
    });
  }

  // ------------------------------------------------------------- painting

  function paint() {
    var pct = face.budget ? Math.min(100, Math.round((face.context / face.budget) * 100)) : 0;
    el.spend.textContent = tokens(face.spend);
    el.context.textContent = pct + '%';
    el.contextFill.style.width = pct + '%';
    el.contextMeter.dataset.level = pct >= 100 ? 'full' : pct >= 75 ? 'high' : 'calm';

    if (face.route) {
      el.routePill.textContent = face.route.provider;
      el.routePill.dataset.state = 'routed';
      el.routePill.title = 'this run served by ' + face.route.provider + ' \u2014 ' + face.route.how;
    } else {
      el.routePill.textContent = 'no route yet';
      el.routePill.dataset.state = 'idle';
    }

    if (face.seat) {
      el.statePill.textContent = 'waiting for you';
      el.statePill.dataset.state = 'waiting';
    } else if (face.running) {
      el.statePill.textContent = 'running';
      el.statePill.dataset.state = 'working';
    } else {
      el.statePill.textContent = face.stop ? (face.stop.type === 'failed' ? 'failed' : 'idle') : 'idle';
      el.statePill.dataset.state = face.stop && face.stop.type === 'failed' ? 'down' : 'idle';
    }

    paintFacts();
    paintRules();
    paintSeat();
    paintComposer();
    // The router's live decision is the run's own `routed` event, so it
    // is repainted with the stream rather than waiting for the board's
    // clock. The rest of that panel is the board's read and does not move
    // on an event.
    if (face.panel === 'router') paintRouter();
  }

  function paintFacts() {
    var rows = [
      ['session', face.session],
      ['model', face.model],
      ['spend', tokens(face.spend) + ' tokens'],
      ['context', face.context ? tokens(face.context) + ' of ' + tokens(face.budget) : '\u2014'],
      ['turns', face.turn ? String(face.turn) : '\u2014'],
      ['route', face.route ? face.route.provider : '\u2014'],
      ['state', face.seat ? 'a seat is standing' : face.running ? 'running' : face.stop ? face.stop.type : 'idle']
    ];
    if (face.dropped) rows.push(['dropped', String(face.dropped)]);
    el.facts.textContent = '';
    rows.forEach(function (pair) {
      var wrap = node('div', 'fact');
      wrap.appendChild(node('dt', null, pair[0]));
      wrap.appendChild(node('dd', null, pair[1]));
      el.facts.appendChild(wrap);
    });
  }

  function paintRules() {
    el.ruleCount.textContent = String(face.rules.length);
    el.rules.textContent = '';
    if (!face.rules.length) {
      var empty = node('li', 'empty-rule', 'nothing remembered yet');
      el.rules.appendChild(empty);
      return;
    }
    face.rules.forEach(function (rule) {
      var item = node('li');
      item.appendChild(node('span', 'rule-label', rule.label));
      if (rule.program) item.appendChild(node('span', 'rule-program', rule.program));
      el.rules.appendChild(item);
    });
  }

  // The seat: four words, and the fourth only when the writer would
  // honour it. Offering a promise the one writer refuses is the kind of
  // lie this face exists to not tell — so a destructive call is asked
  // every time, and the seat says why rather than hiding a button.
  var DECISIONS = [
    ['once', 'Yes', 'allow this call'],
    ['always', 'Yes, and the rest of this run', 'allow this call and its class until the run ends'],
    ['deny', 'No', 'refuse this call'],
    ['remember', 'Yes, and don\u2019t ask again', 'allow this call and write the rule through the one writer']
  ];

  function paintSeat() {
    var seat = face.seat;
    if (!seat) {
      el.seat.hidden = true;
      face.seatKey = '';
      return;
    }
    // The seat stands over the composer, so a rebuild per token would
    // move the buttons under the owner's cursor mid-click. The key is
    // what the seat actually says; a new seat, and only a new seat,
    // builds the buttons again.
    var key = [seat.tool, seat.summary, seat.label, seat.allowlistable].join('\u0000');
    el.seat.hidden = false;
    el.seatLabel.textContent = seat.label || seat.tool;
    el.seatSummary.textContent = seat.summary;
    if (key === face.seatKey) return;
    face.seatKey = key;
    el.seatActions.textContent = '';
    DECISIONS.forEach(function (pair) {
      if (pair[0] === 'remember' && !seat.allowlistable) return;
      var button = node('button', 'decision', pair[1]);
      button.type = 'button';
      button.dataset.decision = pair[0];
      button.title = pair[2];
      button.addEventListener('click', function () { answer(pair[0]); });
      el.seatActions.appendChild(button);
    });
    if (seat.allowlistable) {
      el.seatNote.hidden = true;
    } else {
      el.seatNote.textContent = 'this class of call is never remembered \u2014 it is asked every time.';
      el.seatNote.hidden = false;
    }
  }

  function paintComposer() {
    var busy = face.running || !!face.seat;
    el.send.disabled = busy;
    if (face.seat) {
      el.hint.textContent = 'the seat above is waiting for an answer';
    } else if (face.running) {
      el.hint.textContent = 'a run is in flight \u00b7 the composer opens when it ends';
    } else {
      el.hint.textContent = 'Enter sends \u00b7 Shift+Enter is a newline';
    }
  }

  // ------------------------------------------------------- the router board

  // Two sources feed this panel and the panel names both wherever a
  // figure lands. The board's read is one document from one route: the
  // chain, the ledger, the rows, the cache. The live decision is the
  // `routed` event already in the log, so it moves with the stream and
  // never waits for a refresh. Nothing here is derived twice, and a card
  // with nothing to say says so.

  // The donut's segments, in the order the legend lists them. These come
  // off the reference palette directly: a segment's job is to be told
  // apart from its neighbour, not to carry a meaning of its own.
  var MIX_COLORS = [
    'var(--dsw-alias-accent)',
    'var(--dsw-static-deepseek-300)',
    'var(--dsw-static-deepseek-600)',
    'var(--dsw-static-ink-400)',
    'var(--dsw-static-ink-500)',
    'var(--dsw-static-ink-600)'
  ];

  // An SVG element, its attributes, and the text it carries. A node's
  // `<title>` is the whole of what its box had to clip, so the third
  // argument is not optional in spirit: an element that says nothing
  // shows no tooltip and the clipped words are simply gone.
  function svg(name, attrs, said) {
    var made = document.createElementNS('http://www.w3.org/2000/svg', name);
    Object.keys(attrs || {}).forEach(function (key) { made.setAttribute(key, attrs[key]); });
    if (said != null) made.textContent = said;
    return made;
  }

  // A card's inner width, from the board it stands in. Every chart is
  // laid out against this, so a chart fills the card it was drawn for at
  // the scale it was drawn at rather than being stretched to whatever
  // width it happens to land in. The board is twelve equal tracks with a
  // gap between them, so a card of span n is n tracks and n-1 gaps.
  // A board nobody has laid out yet measures zero, and the board's own
  // designed width stands in for it.
  var BOARD_WIDE = 1280, CARD_GAP = 14, CARD_PAD = 32;

  function cardWide(board, span) {
    var wide = board.clientWidth;
    if (wide < 240) wide = BOARD_WIDE;
    var track = (wide - CARD_GAP * 11) / 12;
    return Math.max(240, Math.round(track * span + CARD_GAP * (span - 1) - CARD_PAD));
  }

  function boardCard(span, title, stamp, note) {
    var el0 = node('section', 'board-card');
    if (span) el0.dataset.span = String(span);
    var head = node('header', 'board-head');
    head.appendChild(node('h2', null, title));
    if (stamp) head.appendChild(node('span', 'board-stamp', stamp));
    el0.appendChild(head);
    if (note) {
      var said = node('p', 'board-note', note);
      el0.appendChild(said);
    }
    return el0;
  }

  function tile(value, label, tone) {
    var el0 = node('div', 'tile');
    if (tone) el0.dataset.tone = tone;
    el0.appendChild(node('div', 'tile-value', String(value)));
    el0.appendChild(node('div', 'tile-label', label));
    return el0;
  }

  // The clock a stamp carries, not the stamp: a route row's own time is
  // the crate's UTC vocabulary, and the panel shows the hour and minute
  // the row was written at.
  function clock(stamp) {
    var at = String(stamp || '');
    return at.length >= 16 ? at.slice(11, 16) : (at || '\u2014');
  }

  // The day column chart. One column per hour the ledger saw, its height
  // the hour's calls, and the calls that did not serve stacked at the top
  // in the down colour. An hour with no calls is not a zero-height column
  // — the board already left those out, and a chart that drew them would
  // be inventing the shape of a day it never saw. The width is the card's
  // own, so the columns widen with the card instead of the whole chart
  // being scaled up to it.
  function dayChart(hours, wide) {
    var wrap = node('div', 'chart');
    var W = Math.max(240, Math.round(wide || 320)), H = 118, base = H - 18, top = 14, gap = 6;
    var width = hours.length ? (W - gap * (hours.length - 1)) / hours.length : 0;
    var most = hours.reduce(function (peak, column) { return Math.max(peak, column.calls); }, 0);
    var plot = svgPlot(W, H, 'the day\u2019s calls by their own hour');
    plot.appendChild(svg('line', { x1: 0, y1: base + 0.5, x2: W, y2: base + 0.5, class: 'chart-base' }));
    hours.forEach(function (column, index) {
      var x = index * (width + gap);
      var full = most ? Math.round((column.calls / most) * (base - top)) : 0;
      var bad = column.calls ? Math.round((column.errors / column.calls) * full) : 0;
      if (full > 0) {
        plot.appendChild(svg('rect', { x: x, y: base - full, width: width, height: Math.max(1, full),
          rx: 2, class: 'chart-col' }));
      }
      if (bad > 0) {
        plot.appendChild(svg('rect', { x: x, y: base - full, width: width, height: Math.max(1, bad),
          rx: 2, class: 'chart-col', 'data-tone': 'down' }));
      }
      if (column.calls) {
        var cap = svg('text', { x: x + width / 2, y: base - full - 4, class: 'chart-cap',
          'text-anchor': 'middle' });
        cap.textContent = column.calls > 999 ? tokens(column.calls) : String(column.calls);
        plot.appendChild(cap);
      }
      var hour = svg('text', { x: x + width / 2, y: H - 5, class: 'chart-hour', 'text-anchor': 'middle' });
      hour.textContent = String(column.hour).padStart(2, '0');
      plot.appendChild(hour);
    });
    wrap.appendChild(plot);
    return wrap;
  }

  // The model-mix donut. Each segment is a dash of one circumference, so
  // a model that took one call draws a hairline and one that took a third
  // of the day draws an arc, and no path arithmetic is involved. The total
  // is the day's calls, not its serves: the mix says where calls landed,
  // and the ledger beside it already says how many served.
  function donut(models) {
    var wrap = node('div', 'donut');
    var R = 34, C = 2 * Math.PI * R;
    var total = models.reduce(function (sum, entry) { return sum + entry.calls; }, 0);
    var plot = svgPlot(92, 92, 'the day\u2019s calls by model');
    plot.appendChild(svg('circle', { cx: 46, cy: 46, r: R, class: 'donut-track', 'stroke-width': 13 }));
    var run = 0;
    models.forEach(function (entry, index) {
      var share = total ? entry.calls / total : 0;
      if (share <= 0) return;
      plot.appendChild(svg('circle', {
        cx: 46, cy: 46, r: R, class: 'donut-seg', 'stroke-width': 13,
        stroke: MIX_COLORS[index % MIX_COLORS.length],
        'stroke-dasharray': (share * C).toFixed(2) + ' ' + (C - share * C).toFixed(2),
        'stroke-dashoffset': (-run * C).toFixed(2),
        transform: 'rotate(-90 46 46)'
      }));
      run += share;
    });
    var mid = svg('text', { x: 46, y: 43, 'text-anchor': 'middle',
      fill: 'var(--dsw-alias-ink)', 'font-size': 15, 'font-weight': 600 });
    mid.textContent = total > 999 ? tokens(total) : String(total);
    var under = svg('text', { x: 46, y: 56, 'text-anchor': 'middle',
      fill: 'var(--dsw-alias-ink-tertiary)', 'font-size': 8 });
    under.textContent = 'calls';
    plot.appendChild(mid);
    plot.appendChild(under);
    wrap.appendChild(plot);
    var legend = node('ul', 'legend');
    models.forEach(function (entry, index) {
      var item = node('li');
      var swatch = node('span', 'swatch');
      swatch.style.background = MIX_COLORS[index % MIX_COLORS.length];
      item.appendChild(swatch);
      item.appendChild(node('span', 'legend-name', entry.model));
      item.appendChild(node('span', 'legend-value',
        entry.calls + (total ? ' \u00b7 ' + Math.round((entry.calls / total) * 100) + '%' : '')));
      legend.appendChild(item);
    });
    wrap.appendChild(legend);
    return wrap;
  }

  // The declared chain: the workload preference table, in its own order.
  // A workload's first candidate is what a route selects; the rest are
  // what the workload's own failover walks, so the order is the content.
  function chainCard(read) {
    var el0 = boardCard(12, 'the declared chain', 'the board\u2019s read',
      'Every workload\u2019s candidates in the table\u2019s own order. The first is what a route selects; the rest are what the walk attempts when it does not.');
    var chains = node('div', 'chains');
    read.tiers.forEach(function (chain) {
      var line = node('div', 'chain');
      line.appendChild(node('span', 'chain-name', chain.tier));
      var row = node('div', 'chain-candidates');
      chain.candidates.forEach(function (candidate, index) {
        if (index) row.appendChild(node('span', 'chain-arrow', '\u203a'));
        var chip = node('span', 'cand', candidate);
        chip.dataset.role = index === 0 ? 'primary' : 'fallback';
        chip.title = index === 0 ? 'what a route selects for this workload' : 'the walk\u2019s next candidate';
        row.appendChild(chip);
      });
      line.appendChild(row);
      chains.appendChild(line);
    });
    el0.appendChild(chains);
    // The gates, beside the chain: what the owner pinned, and whether the
    // payg reserve may serve at all.
    var gates = node('div', 'gates');
    var pin = node('div', 'gate');
    pin.dataset.state = read.pin ? 'pinned' : 'off';
    pin.appendChild(node('span', 'gate-dot'));
    if (read.pin) {
      pin.appendChild(node('span', null, 'pinned to '));
      pin.appendChild(node('code', null, read.pin));
    } else {
      pin.appendChild(node('span', null, 'nothing pinned \u00b7 the table decides'));
    }
    gates.appendChild(pin);
    var payg = node('div', 'gate');
    payg.dataset.state = read.payg.allowed ? 'on' : 'off';
    payg.appendChild(node('span', 'gate-dot'));
    payg.appendChild(node('span', null, read.payg.allowed
      ? 'payg may serve \u2014 a chain names it'
      : 'payg may not serve \u2014 ' + (read.payg.providers.join(', ') || 'nothing') + ' is the reserve'));
    gates.appendChild(payg);
    var judge = node('div', 'gate');
    judge.dataset.state = read.judge.lane ? 'on' : 'off';
    judge.appendChild(node('span', 'gate-dot'));
    judge.appendChild(node('span', null, read.judge.lane
      ? 'the judge is ' + read.judge.lane +
        (read.judge.default_workload ? ' \u00b7 an unmapped ask defaults to ' + read.judge.default_workload : '')
      : 'no judge lane is configured'));
    gates.appendChild(judge);
    el0.appendChild(gates);
    return el0;
  }

  // The live decision, off the run's own `routed` event. This is the one
  // card on the panel that is not the board's read, and it says so.
  function decisionCard() {
    var el0 = boardCard(7, 'the live decision', 'the run\u2019s own routed event');
    var body = node('div', 'decision-body');
    var route = face.route;
    if (!route) {
      body.appendChild(node('p', 'board-empty',
        'No route has been made this session. Send a task and the decision lands here, ' +
        'and in the conversation, on the same event.'));
      el0.appendChild(body);
      return el0;
    }
    var what = node('div', 'decision-what');
    what.appendChild(node('span', 'decision-model', route.model || route.provider));
    what.appendChild(node('span', 'decision-provider', route.provider));
    what.appendChild(node('span', 'decision-how', route.how));
    body.appendChild(what);
    if (route.reason) body.appendChild(node('p', 'decision-reason', route.reason));
    var facts = node('dl', 'facts');
    [
      ['workload', route.workload || 'none judged'],
      ['verdict', route.verdict || '\u2014'],
      ['judged from', route.judged_from || 'none']
    ].forEach(function (pair) {
      var wrap = node('div', 'fact');
      wrap.appendChild(node('dt', null, pair[0]));
      wrap.appendChild(node('dd', null, pair[1]));
      facts.appendChild(wrap);
    });
    body.appendChild(facts);
    // The walk: the candidates the route attempted, in order. A walk of
    // one candidate served on the first try or did not serve at all, and
    // the card shows which without pretending to know more.
    var order = route.order || [];
    if (order.length) {
      var label = node('p', 'board-note', 'the walk, in the order it was attempted');
      body.appendChild(label);
      var walk = node('div', 'decision-order');
      order.forEach(function (step, index) {
        if (index) walk.appendChild(node('span', 'chain-arrow', '\u203a'));
        var chip = node('span', 'cand', step.model || step.provider);
        chip.dataset.role = index === order.length - 1 ? 'primary' : 'fallback';
        walk.appendChild(chip);
      });
      body.appendChild(walk);
    }
    el0.appendChild(body);
    return el0;
  }

  // The cache stats: how many decisions the route-decision cache holds,
  // whether they belong to the lane that would judge now, and the
  // workload mix of what it has remembered.
  function cacheCard(read) {
    var cache = read.cache;
    var el0 = boardCard(5, 'the decision cache', 'the board\u2019s read',
      cache.keyed
        ? 'A cache written by the lane that judges now, so these entries are the ones a route reads.'
        : 'The cache on disk was written by another lane, which a route discards whole \u2014 so the entries below are held, not read.');
    var tiles = node('div', 'tiles');
    tiles.appendChild(tile(cache.entries, 'entries'));
    tiles.appendChild(tile(cache.keyed ? 'live' : 'held', cache.keyed ? 'for this lane' : 'for another lane',
      cache.keyed ? 'ok' : 'down'));
    el0.appendChild(tiles);
    if (cache.workloads && cache.workloads.length) {
      var most = cache.workloads.reduce(function (peak, entry) { return Math.max(peak, entry.count); }, 0);
      var bars = node('div', 'bars');
      bars.style.marginTop = '14px';
      cache.workloads.forEach(function (entry) {
        var bar = node('div', 'bar');
        var top = node('div', 'bar-top');
        top.appendChild(node('span', 'bar-label', entry.workload));
        top.appendChild(node('span', 'bar-value', String(entry.count)));
        bar.appendChild(top);
        var track = node('div', 'bar-track');
        var fill = node('div', 'bar-fill');
        fill.style.width = (most ? Math.max(3, Math.round((entry.count / most) * 100)) : 0) + '%';
        track.appendChild(fill);
        bar.appendChild(track);
        bars.appendChild(bar);
      });
      el0.appendChild(bars);
    } else {
      el0.appendChild(node('p', 'board-empty', 'The cache holds no workload counts yet.'));
    }
    return el0;
  }

  // The day: the ledger's own tiles and the day column chart.
  function dayCard(read, wide) {
    var el0 = boardCard(7, 'today\u2019s ledger', 'the board\u2019s read \u00b7 ' + clock(read.read_at));
    var totals = read.totals;
    if (!totals) {
      el0.appendChild(node('p', 'board-empty',
        'The ledger has seen no route rows today. The chain above still stands, and the tiles ' +
        'fill in as the day is routed.'));
      return el0;
    }
    var tiles = node('div', 'tiles');
    tiles.appendChild(tile(tokens(totals.calls), 'calls'));
    tiles.appendChild(tile(tokens(totals.ok), 'served', 'ok'));
    tiles.appendChild(tile(tokens(totals.errors), 'failed', totals.errors ? 'down' : null));
    tiles.appendChild(tile(totals.success_rate == null ? '\u2014' : totals.success_rate + '%', 'served rate'));
    el0.appendChild(tiles);
    if (read.hours && read.hours.length) {
      var note = node('p', 'board-note',
        'The day\u2019s calls by the hour their own stamp says. An hour with no calls is left out rather ' +
        'than drawn as a zero.');
      note.style.marginTop = '14px';
      el0.appendChild(note);
      el0.appendChild(dayChart(read.hours, wide));
    }
    return el0;
  }

  // The model mix, as a donut and its legend: where the day's calls
  // actually landed.
  function mixCard(read) {
    var el0 = boardCard(5, 'the model mix', 'the board\u2019s read');
    if (!read.models || !read.models.length) {
      el0.appendChild(node('p', 'board-empty', 'No model took a call today.'));
      return el0;
    }
    el0.appendChild(donut(read.models));
    return el0;
  }

  // Today's route rows: the audit trail's own tail, newest first. A row
  // is the call, the walk it took, and where it landed.
  function rowsCard(read) {
    var el0 = boardCard(7, 'today\u2019s route rows', 'the board\u2019s read');
    if (!read.recent || !read.recent.length) {
      el0.appendChild(node('p', 'board-empty', 'No route rows today.'));
      return el0;
    }
    var rows = node('div', 'rows');
    read.recent.forEach(function (entry) {
      var line = node('div', 'rrow');
      line.appendChild(node('span', 'rrow-at', clock(entry.at)));
      var what = node('div', 'rrow-what');
      what.appendChild(node('span', 'rrow-model', entry.model));
      var meta = [];
      if (entry.workload) meta.push(entry.workload);
      if (entry.reason) meta.push(entry.reason);
      what.appendChild(node('span', 'rrow-meta', meta.length ? meta.join(' \u00b7 ') : 'no reason recorded'));
      // The walk the row actually took. The board carries the candidates
      // in the order they were attempted, and the last one is where the
      // row landed — so the ones before it are the ones that did not.
      var walk = entry.providers_used && entry.providers_used.length
        ? entry.providers_used : [entry.provider];
      var strip = node('div', 'rrow-walk');
      walk.forEach(function (step, index) {
        var chip = node('span', 'walk-step', step);
        chip.dataset.outcome = index === walk.length - 1 ? entry.outcome : 'failed';
        chip.title = index === walk.length - 1
          ? 'where the row landed: ' + entry.outcome
          : 'attempted and did not serve';
        strip.appendChild(chip);
      });
      what.appendChild(strip);
      line.appendChild(what);
      var side = node('div', 'rrow-side');
      side.appendChild(node('span', 'rrow-outcome', entry.outcome));
      side.lastChild.dataset.ok = entry.ok ? 'yes' : 'no';
      side.appendChild(node('span', 'rrow-ms', entry.ms == null ? '\u2014' : entry.ms + ' ms'));
      line.appendChild(side);
      rows.appendChild(line);
    });
    el0.appendChild(rows);
    return el0;
  }

  // The workload mix and the judged split: where the day's routes went,
  // and how many of the judgments were made rather than remembered.
  function mixSplitCard(read) {
    var el0 = boardCard(5, 'the workload mix', 'the board\u2019s read');
    if (!read.workloads || !read.workloads.length) {
      el0.appendChild(node('p', 'board-empty', 'No workload was judged today.'));
    } else {
      var most = read.workloads.reduce(function (peak, entry) { return Math.max(peak, entry.count); }, 0);
      var total = read.workloads.reduce(function (sum, entry) { return sum + entry.count; }, 0);
      var bars = node('div', 'bars');
      read.workloads.forEach(function (entry) {
        var bar = node('div', 'bar');
        var top = node('div', 'bar-top');
        top.appendChild(node('span', 'bar-label', entry.workload));
        top.appendChild(node('span', 'bar-value',
          entry.count + (total ? ' \u00b7 ' + Math.round((entry.count / total) * 100) + '%' : '')));
        bar.appendChild(top);
        var track = node('div', 'bar-track');
        var fill = node('div', 'bar-fill');
        fill.style.width = (most ? Math.max(3, Math.round((entry.count / most) * 100)) : 0) + '%';
        track.appendChild(fill);
        bar.appendChild(track);
        bars.appendChild(bar);
      });
      el0.appendChild(bars);
    }
    var judged = read.judged;
    if (judged) {
      var note = node('p', 'board-note', 'How the day\u2019s judgments were obtained.');
      note.style.margin = '14px 0 8px';
      el0.appendChild(note);
      var split = node('div', 'split');
      var total = judged.fresh + judged.cached + judged.errors;
      var track = node('div', 'split-track');
      [['fresh', judged.fresh], ['cached', judged.cached], ['errors', judged.errors]]
        .forEach(function (pair) {
          var fill = node('div', 'split-fill');
          fill.dataset.tone = pair[0];
          fill.style.width = (total ? (pair[1] / total) * 100 : 0) + '%';
          track.appendChild(fill);
        });
      split.appendChild(track);
      var legend = node('ul', 'split-legend');
      [['fresh', judged.fresh, 'judged this run'],
       ['cached', judged.cached, 'served from the route-decision cache'],
       ['errors', judged.errors, 'never obtained']]
        .forEach(function (triple) {
          var item = node('li');
          var swatch = node('span', 'swatch');
          swatch.style.background = triple[0] === 'fresh' ? 'var(--dsw-alias-accent)'
            : triple[0] === 'cached' ? 'var(--dsw-static-deepseek-300)' : 'var(--dsw-alias-down)';
          item.appendChild(swatch);
          item.appendChild(node('span', 'legend-name', triple[0]));
          item.appendChild(node('span', 'legend-value', triple[1] + ' \u00b7 ' + triple[2]));
          legend.appendChild(item);
        });
      split.appendChild(legend);
      el0.appendChild(split);
    }
    return el0;
  }

  // Fleet's own failover feedback: the candidates the calibration is
  // demoting for this day, and the clause that says why. A day with no
  // demotion draws no card at all rather than an empty one.
  function guidanceCard(read) {
    if (!read.guidance || !read.guidance.length) return null;
    var el0 = boardCard(12, 'failover feedback', 'the calibration\u2019s own');
    var rules = node('ul', 'rules');
    read.guidance.forEach(function (entry) {
      var item = node('li');
      item.appendChild(node('span', 'rule-label', entry.candidate));
      item.appendChild(node('span', 'rule-program', entry.clause));
      rules.appendChild(item);
    });
    el0.appendChild(rules);
    return el0;
  }

  function paintRouter() {
    var read = face.router;
    var wide7 = cardWide(el.routerBoard, 7);
    el.routerBoard.textContent = '';
    // Before the first read lands, the panel says so rather than drawing
    // an empty grid. The live decision is not in the read and paints
    // either way, so it is drawn first in both cases.
    if (!read) {
      var waiting = boardCard(12, 'the router', null, null);
      waiting.appendChild(node('p', 'board-empty',
        'The board\u2019s read has not landed yet. The live decision below is the run\u2019s own event ' +
        'and does not wait for it.'));
      el.routerBoard.appendChild(waiting);
      el.routerBoard.appendChild(decisionCard());
      return;
    }
    el.routerBoard.appendChild(decisionCard());
    el.routerBoard.appendChild(cacheCard(read));
    el.routerBoard.appendChild(dayCard(read, wide7));
    el.routerBoard.appendChild(mixCard(read));
    el.routerBoard.appendChild(chainCard(read));
    el.routerBoard.appendChild(rowsCard(read));
    el.routerBoard.appendChild(mixSplitCard(read));
    var guidance = guidanceCard(read);
    if (guidance) el.routerBoard.appendChild(guidance);
    if (read.refused && read.refused.length) {
      var el0 = boardCard(12, 'reads that did not land', 'the board\u2019s read');
      el0.appendChild(node('p', 'board-empty', read.refused.join(' \u00b7 ')));
      el.routerBoard.appendChild(el0);
    }
  }

  // The board's read is a document, not a derivation: it is fetched, and
  // it is refreshed while the panel is showing because a day's ledger
  // moves without the log saying anything.
  function readRouter(force) {
    var now = Date.now();
    if (!force && face.router && now - face.routerAt < 4000) return Promise.resolve();
    face.routerAt = now;
    return fetch('/api/router', { headers: { accept: 'application/json' } })
      .then(function (reply) {
        if (reply.status === 401) throw new Error('token');
        return reply.json();
      })
      .then(function (doc) {
        face.router = doc;
        if (face.panel === 'router') paintRouter();
      })
      .catch(function () { });
  }

  // ------------------------------------------------- the delegation panel

  // The two pictures the panel owes: the tree of the run in flight and
  // the runs before it, and the declared dependency graph. Both are
  // nodes and edges, because what blocks what is a shape and not a row.

  // The gaps that separate a tree node from its children, and a graph
  // node from the layer under it; the height a node needs to carry what
  // it says. A node's own width is not a constant: it is derived from the
  // card the chart stands in, so the nodes fill their card at the scale
  // they were drawn at rather than being stretched to it.
  var PLOT_MARGIN = 8;
  var TREE_H = 58, TREE_X = 26, TREE_Y = 40, TREE_MIN_W = 150;
  var GRAPH_H = 66, GRAPH_X = 30, GRAPH_Y = 54, GRAPH_MIN_W = 150, GRAPH_MAX_W = 300;

  // The state words the graph's own `state_word` returns, and the tone
  // each reads in. The panel inherits the vocabulary rather than
  // inventing a second one, so a node cannot be named two ways here and
  // in the terminal.
  var GRAPH_TONE = {
    verified: 'ok', ready: 'ok', running: 'live',
    held: 'warn', waiting: 'idle', failed: 'down'
  };

  // A tree node's own words: `running` while its log carries no end
  // record, `stopped` once it does — the session module's distinction.
  var TREE_TONE = { running: 'live', stopped: 'idle' };

  function clip(text, most) {
    var said = String(text == null ? '' : text);
    return said.length > most ? said.slice(0, most - 1) + '\u2026' : said;
  }

  // A tone word to the colour the face already uses for it, so the
  // panel's dots are the same dots the rest of the face draws.
  function toneColour(tone) {
    return tone === 'ok' ? 'var(--dsw-alias-ok)'
      : tone === 'down' ? 'var(--dsw-alias-down)'
      : tone === 'warn' ? 'var(--dsw-alias-warn)'
      : tone === 'live' ? 'var(--dsw-alias-accent)'
      : 'var(--dsw-alias-ink-tertiary)';
  }

  function dot(x, y, tone, radius) {
    return svg('circle', {
      cx: x, cy: y, r: radius || 3.5,
      fill: toneColour(tone), class: 'node-dot'
    });
  }

  // A plot carries its own size as well as its own coordinate system. The
  // size matters: a chart that only declares a viewBox is scaled by the
  // stylesheet to whatever width it lands in, and an SVG scales its text
  // with its geometry — a 12px label blown up three times is not a label
  // any more. Declaring the size lets a chart shrink to fit a narrow card
  // without ever growing past the scale it was drawn at.
  function svgPlot(w, h, label) {
    return svg('svg', {
      width: w, height: h,
      viewBox: '0 0 ' + w + ' ' + h,
      role: 'img',
      'aria-label': label
    });
  }

  // A line of text in a plot, anchored where it starts: SVG text does
  // not wrap, so a label longer than its box is clipped by the caller
  // rather than by the renderer.
  function label(x, y, className, said, anchor) {
    var made = svg('text', { x: x, y: y, class: className });
    if (anchor) made.setAttribute('text-anchor', anchor);
    made.textContent = said;
    return made;
  }

  // --- the tree ---

  // How many leaf slots a subtree needs. A node with no children needs
  // one; a node with them needs their sum, which is what centres a
  // parent over its children rather than over the leftmost one.
  function treeLeaves(node) {
    return node.children && node.children.length
      ? node.children.reduce(function (sum, child) { return sum + treeLeaves(child); }, 0)
      : 1;
  }

  // The leaf range a subtree occupies, left to right. A node's own centre
  // is the middle of its range, so a parent sits over its children and a
  // lone child sits over its own slot.
  function layTree(node, depth, left, placed) {
    var leaves = treeLeaves(node);
    placed.push({ node: node, depth: depth, lo: left, hi: left + leaves - 1 });
    var cursor = left;
    (node.children || []).forEach(function (child) {
      cursor = layTree(child, depth + 1, cursor, placed);
    });
    return left + leaves;
  }

  // A node's left edge: the middle of its leaf range, less half a box.
  // The half-gap between slots is taken off the ends so the outermost box
  // starts flush with the plot's own margin.
  function treeLeft(one, boxW) {
    return PLOT_MARGIN + (one.lo + one.hi + 1) / 2 * (boxW + TREE_X) - TREE_X / 2 - boxW / 2;
  }

  // The width the boxes take when `leaves` of them share a card of
  // `wide` units. They share the whole card: one node alone is a wide
  // node rather than a small one swimming in whitespace, and a bushy
  // subtree narrows them until they stop being readable, at which point
  // the whole chart is scaled down instead and keeps its proportions.
  function treeBoxWide(wide, leaves) {
    var usable = Math.max(320, wide) - PLOT_MARGIN * 2 - TREE_X * (leaves - 1);
    return Math.max(TREE_MIN_W, usable / leaves);
  }

  // A parent to its child: one cubic, whose control points sit halfway
  // down the gap, so an edge leaves the parent's bottom and enters the
  // child's top going straight down on both ends.
  function treeEdges(node, at, into, boxW) {
    (node.children || []).forEach(function (child) {
      var a = at[node.id], b = at[child.id];
      var x1 = a.x + boxW / 2, y1 = a.y + TREE_H;
      var x2 = b.x + boxW / 2, y2 = b.y;
      var mid = y1 + (y2 - y1) / 2;
      into.appendChild(svg('path', {
        d: 'M' + x1 + ' ' + y1 + 'C' + x1 + ' ' + mid + ' ' + x2 + ' ' + mid + ' ' + x2 + ' ' + y2,
        class: 'tree-edge'
      }));
      treeEdges(child, at, into, boxW);
    });
  }

  // One node of a tree: the model it ran, the task it was handed, the
  // depth it sits at, and its state. A node at the cap is dashed,
  // because the code refuses to make it a parent and a subtree drawn
  // there would be a subtree that could not have run.
  function treeBox(one, at, cap, boxW) {
    var made = svg('g', { class: 'tree-node' });
    var x = at.x, y = at.y;
    var tone = TREE_TONE[one.status] || 'idle';
    var box = svg('rect', {
      x: x, y: y, width: boxW, height: TREE_H, rx: 8,
      class: 'tree-box', 'data-tone': tone
    });
    if (cap && one.depth >= cap) box.classList.add('is-cap');
    made.appendChild(box);
    made.appendChild(dot(x + 15, y + 16, tone));
    // What a label may say is the box's own width: SVG text does not
    // wrap, so a name or a goal longer than its box would run into the
    // next one rather than be clipped by the renderer.
    var nameMost = Math.max(10, Math.floor((boxW - 40) / 6.9));
    var goalMost = Math.max(18, Math.floor((boxW - 24) / 6.0));
    made.appendChild(label(x + 26, y + 20, 'tree-name', clip(one.model || 'a model', nameMost)));
    made.appendChild(label(x + boxW - 10, y + 20, 'tree-depth', 'd' + one.depth, 'end'));
    made.appendChild(label(x + 12, y + 42, 'tree-goal',
      clip(one.goal || 'no brief in the log', goalMost)));
    var said = (one.goal || 'no brief in the log')
      + '\n' + (one.model || 'a model')
      + ' \u00b7 depth ' + one.depth
      + ' \u00b7 ' + one.status
      + (one.kind ? ' \u00b7 ' + one.kind : '')
      + (one.stop_reason ? ' \u00b7 stopped: ' + one.stop_reason : '')
      + (one.turns != null ? ' \u00b7 ' + one.turns + ' turns' : '');
    if (cap && one.depth >= cap) {
      said += '\nthe depth cap: the code refuses to delegate from here';
    }
    made.appendChild(svg('title', null, said));
    return made;
  }

  function treeCard(root, title, note, cap, wide) {
    var card = boardCard(12, title, null, note);
    var placed = [];
    var leaves = layTree(root, 0, 0, placed);
    var depthMost = placed.reduce(function (most, one) {
      return Math.max(most, one.depth);
    }, 0);
    var boxW = treeBoxWide(wide, leaves);
    var w = PLOT_MARGIN * 2 + leaves * boxW + (leaves - 1) * TREE_X;
    var h = PLOT_MARGIN * 2 + (depthMost + 1) * (TREE_H + TREE_Y) - TREE_Y;
    var wrap = node('div', 'chart');
    var made = svgPlot(w, h, title);
    var at = {};
    placed.forEach(function (one) {
      at[one.node.id] = {
        x: treeLeft(one, boxW),
        y: PLOT_MARGIN + one.depth * (TREE_H + TREE_Y)
      };
    });
    treeEdges(root, at, made, boxW);
    placed.forEach(function (one) {
      made.appendChild(treeBox(one.node, at[one.node.id], cap, boxW));
    });
    wrap.appendChild(made);
    card.appendChild(wrap);
    return card;
  }

  // --- the graph ---

  // A node's layer is the longest chain of prerequisites under it, so a
  // node sits below everything that must finish before it rather than
  // beside it. A prerequisite the file names but does not declare has no
  // layer to wait for: it is unresolved, which the node's own card says.
  function graphLayers(graph) {
    var index = {};
    graph.nodes.forEach(function (one, at) { index[one.id] = at; });
    var layer = graph.nodes.map(function () { return 0; });
    for (var pass = 0; pass < graph.nodes.length; pass++) {
      var moved = false;
      graph.nodes.forEach(function (one, at) {
        var want = 0;
        one.prereqs.forEach(function (prereq) {
          var from = index[prereq];
          if (from != null) want = Math.max(want, layer[from] + 1);
        });
        if (want > layer[at]) { layer[at] = want; moved = true; }
      });
      if (!moved) break;
    }
    return layer;
  }

  function graphEdges(graph, at, into, boxW) {
    graph.edges.forEach(function (edge) {
      var a = at[graph.nodes[edge.from].id], b = at[graph.nodes[edge.to].id];
      if (!a || !b) return;
      var x1 = a.x + boxW, y1 = a.y + GRAPH_H / 2;
      var x2 = b.x, y2 = b.y + GRAPH_H / 2;
      var mid = x1 + (x2 - x1) / 2;
      into.appendChild(svg('path', {
        d: 'M' + x1 + ' ' + y1 + 'C' + mid + ' ' + y1 + ' ' + mid + ' ' + y2 + ' ' + x2 + ' ' + y2,
        class: 'graph-edge'
      }));
    });
  }

  // One graph node: its name, the state it stands in, and its score
  // against its own peak — the readiness bar the graph keeps.
  function graphBox(one, at, boxW) {
    var made = svg('g', { class: 'graph-node' });
    var tone = GRAPH_TONE[one.state] || 'idle';
    var x = at.x, y = at.y;
    made.appendChild(svg('rect', {
      x: x, y: y, width: boxW, height: GRAPH_H, rx: 8,
      class: 'graph-box', 'data-tone': tone
    }));
    made.appendChild(dot(x + 14, y + 17, tone));
    made.appendChild(label(x + 24, y + 21, 'graph-name', clip(one.id, Math.max(8, Math.floor((boxW - 40) / 6.9)))));
    made.appendChild(label(x + boxW - 10, y + 21, 'graph-state', one.state, 'end'));
    // The score as a bar against the peak: what is left of this node's
    // closure, drawn as the fraction of the most it has ever been.
    made.appendChild(svg('rect', {
      x: x + 14, y: y + 36, width: boxW - 28, height: 4, rx: 2,
      class: 'graph-track'
    }));
    var share = one.peak > 0 ? Math.min(1, one.score / one.peak) : 0;
    if (share > 0) {
      made.appendChild(svg('rect', {
        x: x + 14, y: y + 36, width: (boxW - 28) * share, height: 4, rx: 2,
        class: 'graph-fill', 'data-tone': tone
      }));
    }
    made.appendChild(label(x + 14, y + 56, 'graph-score',
      one.score + ' of ' + one.peak + ' left \u00b7 cost ' + one.cost));
    var said = one.id + ' \u00b7 ' + one.state
      + '\nscore ' + one.score + ' of a peak of ' + one.peak + ', cost ' + one.cost
      + (one.reason ? '\nheld: ' + one.reason : '');
    made.appendChild(svg('title', null, said));
    return made;
  }

  function graphCard(graph, wide) {
    var card = boardCard(12, 'the dependency graph', null,
      'Every node\u2019s state is named by the same function the terminal render uses, and an edge runs ' +
      'from a prerequisite to the node waiting on it.');
    var layer = graphLayers(graph);
    var widest = 0, deepest = 0;
    var at = {};
    var seen = {};
    graph.nodes.forEach(function (one, index) {
      var row = layer[index];
      seen[row] = (seen[row] || 0) + 1;
      widest = Math.max(widest, seen[row]);
      deepest = Math.max(deepest, row);
      at[one.id] = { row: row, at: seen[row] - 1 };
    });
    // A node of the graph says little — a name, a state, a score — so a
    // wide card would leave it mostly empty. The nodes widen to fill the
    // card up to a point and then stop, and the chart centres itself in
    // what is left rather than stretching its text to fill it.
    var usable = Math.max(320, wide) - PLOT_MARGIN * 2 - GRAPH_X * (widest - 1);
    var boxW = Math.max(GRAPH_MIN_W, Math.min(GRAPH_MAX_W, usable / widest));
    graph.nodes.forEach(function (one) {
      var where = at[one.id];
      where.x = PLOT_MARGIN + where.at * (boxW + GRAPH_X);
      where.y = PLOT_MARGIN + where.row * (GRAPH_H + GRAPH_Y);
    });
    var w = PLOT_MARGIN * 2 + widest * boxW + (widest - 1) * GRAPH_X;
    var h = PLOT_MARGIN * 2 + (deepest + 1) * (GRAPH_H + GRAPH_Y) - GRAPH_Y;
    var wrap = node('div', 'chart');
    var made = svgPlot(w, h, 'the declared dependency graph');
    graphEdges(graph, at, made, boxW);
    graph.nodes.forEach(function (one) {
      made.appendChild(graphBox(one, at[one.id], boxW));
    });
    wrap.appendChild(made);
    card.appendChild(wrap);
    return card;
  }

  // --- the unlock conditions ---

  // The three conditions the code requires, per node, each as its own
  // mark. This is the panel's whole answer to "why is nothing running":
  // all three are necessary, and the one that is not met is named rather
  // than summarised.
  function unlockRow(one) {
    var row = node('div', 'unlock');
    var head = node('div', 'unlock-head');
    var name = node('span', 'unlock-name', one.id);
    name.title = one.id;
    head.appendChild(name);
    var state = node('span', 'unlock-state', one.state);
    state.dataset.tone = GRAPH_TONE[one.state] || 'idle';
    head.appendChild(state);
    var score = node('span', 'unlock-score',
      one.score + ' of ' + one.peak + ' left');
    head.appendChild(score);
    row.appendChild(head);

    var marks = node('div', 'unlock-marks');
    [['score zero', one.unlock.score_zero],
      ['resource free', one.unlock.resource_free],
      ['deps verified', one.unlock.deps_verified]].forEach(function (pair) {
        var mark = node('span', 'unlock-mark');
        mark.dataset.met = pair[1] ? 'yes' : 'no';
        mark.appendChild(dotHtml(pair[1] ? 'ok' : 'down'));
        mark.appendChild(node('span', null, pair[0]));
        marks.appendChild(mark);
      });
    row.appendChild(marks);

    if (one.approach) row.appendChild(node('p', 'unlock-approach', one.approach));
    var said = [];
    if (one.unlock.waiting.length) said.push('waiting on ' + one.unlock.waiting.join(', '));
    if (one.unlock.failed.length) said.push('failed: ' + one.unlock.failed.join(', '));
    if (one.unlock.unresolved.length) said.push('undeclared prerequisites: ' + one.unlock.unresolved.join(', '));
    one.unlock.held.forEach(function (hold) {
      said.push(hold.path + ' is admitted by ' + hold.holder);
    });
    if (one.capabilities) said.push(one.capabilities);
    if (one.paths.length) said.push('paths: ' + one.paths.join(', '));
    if (one.delta) said.push('delta: ' + one.delta.state + ' \u00b7 ' + one.delta.reason);
    if (said.length) row.appendChild(node('p', 'unlock-said', said.join(' \u00b7 ')));
    return row;
  }

  // The dot the marks draw, in the face's own voice: a small span rather
  // than an SVG, because it sits in a line of text.
  function dotHtml(tone) {
    var made = node('span', 'unlock-dot');
    made.dataset.tone = tone;
    return made;
  }

  function unlockCard(graph) {
    var card = boardCard(7, 'what each node is waiting for', null,
      'All three conditions are necessary and none is a summary of the others.');
    var body = node('div', 'unlocks');
    graph.nodes.forEach(function (one) { body.appendChild(unlockRow(one)); });
    card.appendChild(body);
    return card;
  }

  function graphTiles(graph) {
    var card = boardCard(5, 'the graph\u2019s own aggregates', null, null);
    var tiles = node('div', 'tiles');
    tiles.appendChild(tile(graph.nodes.length, 'nodes'));
    tiles.appendChild(tile(graph.edges.length, 'edges'));
    tiles.appendChild(tile(graph.critical_path.length, 'critical path'));
    tiles.appendChild(tile(graph.revisions, 'revisions'));
    tiles.appendChild(tile(graph.diff_score, 'diff score'));
    tiles.appendChild(tile(graph.replan ? 'yes' : 'no', 'replan', graph.replan ? 'warn' : null));
    card.appendChild(tiles);
    if (graph.blocks_most_work) {
      var note = node('p', 'board-note',
        'blocking the most work: ' + graph.blocks_most_work
        + (graph.critical_path.length ? ' \u00b7 critical path ' + graph.critical_path.join(' \u2192 ') : ''));
      note.style.marginTop = '12px';
      card.appendChild(note);
    }
    return card;
  }

  function refusedCard(refused) {
    var card = boardCard(12, 'reads that did not land', 'the panel\u2019s own read');
    card.appendChild(node('p', 'board-empty', refused.join(' \u00b7 ')));
    return card;
  }

  function paintDelegation() {
    var read = face.delegation;
    var wide = cardWide(el.delegationBoard, 12);
    el.delegationBoard.textContent = '';
    if (!read) {
      var waiting = boardCard(12, 'delegation', null, null);
      waiting.appendChild(node('p', 'board-empty',
        'The delegation read has not landed yet. It reads the session logs and the graph file, ' +
        'both of which are this machine\u2019s own.'));
      el.delegationBoard.appendChild(waiting);
      return;
    }
    var tiles = node('div', 'tiles');
    tiles.appendChild(tile(read.depth_cap, 'depth cap'));
    tiles.appendChild(tile(read.live ? countNodes(read.live) : 0, 'in this run'));
    tiles.appendChild(tile(read.history.length, 'recent runs'));
    tiles.appendChild(tile(read.graph ? read.graph.nodes.length : 0, 'graph nodes'));
    var head = boardCard(12, 'delegation', null, null);
    head.appendChild(tiles);
    el.delegationBoard.appendChild(head);

    if (read.live) {
      el.delegationBoard.appendChild(treeCard(read.live,
        'the run in flight', null, read.depth_cap, wide));
    } else {
      var none = boardCard(12, 'the run in flight', null, null);
      none.appendChild(node('p', 'board-empty',
        'No readable session log for this run yet. A tree is rebuilt off the logs\u2019 own start ' +
        'records, so a run that has not written one has no tree to draw.'));
      el.delegationBoard.appendChild(none);
    }

    read.history.forEach(function (root) {
      el.delegationBoard.appendChild(treeCard(root,
        'run ' + clip(root.id, 22),
        (root.stop_reason ? 'stopped: ' + root.stop_reason : 'no end record')
        + (root.turns != null ? ' \u00b7 ' + root.turns + ' turns' : '')
        + (root.goal ? ' \u00b7 ' + root.goal : ''),
        read.depth_cap, wide));
    });

    if (read.graph && read.graph.nodes.length) {
      el.delegationBoard.appendChild(graphCard(read.graph, wide));
      el.delegationBoard.appendChild(unlockCard(read.graph));
      el.delegationBoard.appendChild(graphTiles(read.graph));
    } else {
      var empty = boardCard(12, 'the dependency graph', null, null);
      empty.appendChild(node('p', 'board-empty',
        'This home has declared no dependency graph. The graph is a file the plan\u2019s work ' +
        'writes; nothing here is inferred from what a run did.'));
      el.delegationBoard.appendChild(empty);
    }

    if (read.refused && read.refused.length) {
      el.delegationBoard.appendChild(refusedCard(read.refused));
    }
  }

  function countNodes(node0) {
    return 1 + (node0.children || []).reduce(function (sum, child) {
      return sum + countNodes(child);
    }, 0);
  }

  function readDelegation(force) {
    var now = Date.now();
    if (!force && face.delegation && now - face.delegationAt < 4000) return Promise.resolve();
    face.delegationAt = now;
    return fetch('/api/delegation', { headers: { accept: 'application/json' } })
      .then(function (reply) {
        if (reply.status === 401) throw new Error('token');
        return reply.json();
      })
      .then(function (doc) {
        face.delegation = doc;
        if (face.panel === 'delegation') paintDelegation();
      })
      .catch(function () { });
  }

  // ------------------------------------------------- the memory and skills

  // The memory panel. The corpus is one read and the search is the
  // panel's own — filtering a read in the browser is cheaper than a read
  // per keystroke, and the corpus is the machine's own to hold. The
  // day's hits come off the calibration stream's rows; this run's own
  // recall is already in the event log, so it is reduced off the log
  // rather than read a second time.
  function paintMemory() {
    var read = face.memory;
    el.memoryBoard.textContent = '';
    if (!read) {
      var waiting = boardCard(12, 'memory', null, null);
      waiting.appendChild(node('p', 'board-empty',
        'The corpus read has not landed yet. It reads the memories tree and the day\u2019s ' +
        'calibration stream, both of which are this machine\u2019s own.'));
      el.memoryBoard.appendChild(waiting);
      return;
    }
    var facts = read.facts || [];
    var live = facts.filter(function (fact) { return !fact.retired; });
    var hits = read.hits || [];
    var recalls = face.events.filter(function (event) { return event.type === 'memory'; });
    var tiles = node('div', 'tiles');
    tiles.appendChild(tile(live.length, 'live facts'));
    tiles.appendChild(tile(facts.filter(function (fact) { return fact.protected; }).length, 'protected'));
    tiles.appendChild(tile(facts.length - live.length, 'retired'));
    tiles.appendChild(tile(hits.length, 'hits ' + read.day));
    tiles.appendChild(tile(recalls.length, 'recalls this run', recalls.length ? 'live' : null));
    var head = boardCard(12, 'memory', null, 'the corpus at ' + read.root);
    head.appendChild(tiles);
    head.appendChild(memorySearch());
    el.memoryBoard.appendChild(head);

    var shown = live.filter(function (fact) { return matchesQuery(fact); });
    if (shown.length) {
      el.memoryBoard.appendChild(factGrid(shown));
    } else {
      var none = boardCard(12, 'the corpus', null, null);
      none.appendChild(node('p', 'board-empty',
        live.length
          ? 'No live fact matches \u201c' + face.query + '\u201d. The search reads the name, the ' +
            'sentence, the type, the shape tags and the body \u2014 every field the fact carries.'
          : 'This home holds no live facts. A fact is one file under the memories tree, and the ' +
            'agent writes them through the substrate\u2019s own writer.'));
      el.memoryBoard.appendChild(none);
    }

    el.memoryBoard.appendChild(recallCard(recalls, hits, read.day));

    var retired = facts.filter(function (fact) { return fact.retired; });
    if (retired.length) {
      var folded = boardCard(12, 'retired facts', null,
        'a supersession folded these away, or their valid-until passed. The corpus keeps them and recall skips them.');
      folded.appendChild(factGrid(retired));
      el.memoryBoard.appendChild(folded);
    }
    if (read.refused && read.refused.length) {
      el.memoryBoard.appendChild(refusedCard(read.refused));
    }
  }

  // The search box, and the query it holds. It is built once per paint
  // and keeps its own value, so typing in it repaints the grid without
  // touching the read.
  function memorySearch() {
    var wrap = node('div', 'panel-search');
    var box = node('input', 'panel-search-box');
    box.type = 'search';
    box.placeholder = 'search the corpus \u2014 name, sentence, type, shape, body';
    box.value = face.query || '';
    box.setAttribute('aria-label', 'search the corpus');
    box.addEventListener('input', function () {
      face.query = box.value;
      paintMemory();
      var again = el.memoryBoard.querySelector('.panel-search-box');
      if (again) { again.focus(); again.setSelectionRange(again.value.length, again.value.length); }
    });
    wrap.appendChild(box);
    return wrap;
  }

  // Whether one fact matches the panel's query: every field the fact
  // carries, lowercased, because a search that read only the name would
  // miss the fact whose body is the reason it was recalled.
  function matchesQuery(fact) {
    var asked = (face.query || '').trim().toLowerCase();
    if (!asked) return true;
    var haystack = [
      fact.name, fact.description, fact.fact_type, fact.tier, fact.basis,
      (fact.shape || []).join(' '), (fact.phrases || []).join(' '),
      (fact.missing || []).join(' '), fact.body, fact.valid_until
    ].join(' ').toLowerCase();
    return haystack.indexOf(asked) !== -1;
  }

  // The corpus as cards: one per fact, with its type and tier as badges
  // and its recall count beside the name. A card opens the fact in the
  // inspector, where its whole body is and its edit lives.
  function factGrid(facts) {
    var grid = node('div', 'fact-grid');
    facts.forEach(function (fact) {
      var card0 = node('article', 'fact-card');
      card0.dataset.tone = fact.protected ? 'protected' : fact.retired ? 'retired' : 'live';
      var name = node('h3', 'fact-name', fact.name);
      var recalls = node('span', 'fact-recalls',
        fact.recall_count + (fact.recall_count === 1 ? ' recall' : ' recalls'));
      card0.appendChild(name);
      card0.appendChild(recalls);
      var badges = node('div', 'fact-badges');
      badges.appendChild(badge(fact.fact_type, fact.protected ? 'accent' : null));
      badges.appendChild(badge(fact.tier, fact.tier === 'working' ? 'warn' : null));
      if (fact.retired) badges.appendChild(badge('retired', 'down'));
      card0.appendChild(badges);
      card0.appendChild(node('p', 'fact-said', fact.description));
      if (fact.shape && fact.shape.length) {
        card0.appendChild(node('p', 'fact-meta',
          'shape: ' + fact.shape.map(function (tag) { return clip(tag, 18); }).join(' \u00b7 ')));
      }
      card0.appendChild(node('p', 'fact-meta',
        'basis: ' + fact.basis + (fact.valid_until ? ' \u00b7 valid until ' + fact.valid_until : '')));
      card0.addEventListener('click', function () { inspectFact(fact); });
      grid.appendChild(card0);
    });
    return grid;
  }

  function badge(text, tone) {
    var el0 = node('span', 'badge', String(text));
    if (tone) el0.dataset.tone = tone;
    return el0;
  }

  // The recall card: this run's own recall, reduced off the log, beside
  // the day's recorded hits. The two are labelled for what they are,
  // because one is the run in front of the owner and the other is the
  // stream's own audit trail.
  function recallCard(recalls, hits, day) {
    var card0 = boardCard(12, 'recall', null, null);
    var body = node('div', 'board-body recall-body');
    var mine = node('section', 'recall-side');
    mine.appendChild(node('h3', null, 'this run'));
    if (recalls.length) {
      var list = node('ul', 'recall-list');
      recalls.forEach(function (event) {
        list.appendChild(node('li', null,
          (event.shape ? 'shape ' + event.shape + ' \u2014 ' : '') + event.facts + ' facts'));
      });
      mine.appendChild(list);
    } else {
      mine.appendChild(node('p', 'board-empty',
        'No recall has happened in this run yet. Each recall rides the event log as its own card, ' +
        'so the panel reads the same place the conversation does.'));
    }
    var theirs = node('section', 'recall-side');
    theirs.appendChild(node('h3', null, 'the stream, ' + day));
    if (hits.length) {
      var rows = node('div', 'rows');
      hits.forEach(function (hit) {
        var row0 = node('div', 'rrow');
        row0.appendChild(node('span', 'rrow-at', clock(hit.ts)));
        row0.appendChild(node('span', 'rrow-what',
          (hit.shape ? 'shape ' + hit.shape + ' \u00b7 ' : '') + hit.fact));
        var side = node('span', 'rrow-side');
        side.appendChild(node('span', 'rrow-outcome', hit.score.toFixed(2)));
        row0.appendChild(side);
        rows.appendChild(row0);
      });
      theirs.appendChild(rows);
    } else {
      theirs.appendChild(node('p', 'board-empty',
        'No recall hit is recorded for today. A hit is one calibration row per fact a recall surfaced.'));
    }
    body.appendChild(mine);
    body.appendChild(theirs);
    card0.appendChild(body);
    return card0;
  }

  // One fact in the inspector: its whole body, and the edit that rides
  // the substrate's own rewriter. The form carries the fields the
  // rewriter owns and nothing else — the name, the tier and the basis
  // are the corpus's own, not a surface's.
  function inspectFact(fact) {
    var form = node('form', 'fact-edit');
    form.appendChild(field('description', 'the fact as one sentence', fact.description, 'text'));
    form.appendChild(field('type', 'the kind the corpus files it under', fact.fact_type, 'text'));
    form.appendChild(field('shape', 'comma-separated structure tags', (fact.shape || []).join(', '), 'text'));
    form.appendChild(field('phrases', 'one per line', (fact.phrases || []).join('\n'), 'area'));
    form.appendChild(field('missing', 'one per line', (fact.missing || []).join('\n'), 'area'));
    form.appendChild(field('valid-until', 'YYYY-MM-DD, or empty', fact.valid_until, 'text'));
    form.appendChild(field('body', 'everything after the frontmatter fence', fact.body, 'area'));
    var save = node('button', 'fact-save', 'save the fact');
    save.type = 'submit';
    form.appendChild(save);
    var said = node('p', 'fact-said-note',
      'The save rides the substrate\u2019s own rewriter: the named frontmatter lines are re-rendered ' +
      'and every other byte survives. The name, the tier and the basis are not editable \u2014 ' +
      'renaming a fact is a supersession, which records why.');
    form.appendChild(said);
    form.addEventListener('submit', function (event) {
      event.preventDefault();
      save.disabled = true;
      save.textContent = 'rewriting\u2026';
      var lines = function (name) {
        return form.querySelector('[name="' + name + '"]').value
          .split('\n').map(function (line) { return line.trim(); })
          .filter(function (line) { return line.length > 0; });
      };
      post('/api/fact', {
        name: fact.name,
        description: form.querySelector('[name="description"]').value,
        type: form.querySelector('[name="type"]').value,
        shape: form.querySelector('[name="shape"]').value,
        phrases: lines('phrases'),
        missing: lines('missing'),
        'valid-until': form.querySelector('[name="valid-until"]').value,
        body: form.querySelector('[name="body"]').value
      }).then(function (reply) {
        if (reply.status === 200) {
          closeDrawer();
          readMemory(true).then(paintMemory);
          return;
        }
        save.disabled = false;
        save.textContent = 'save the fact';
        notice('the fact was not rewritten', reply.body.error || 'the substrate refused it');
      }).catch(function () {
        save.disabled = false;
        save.textContent = 'save the fact';
        notice('the fact was not rewritten', 'the server did not answer');
      });
    });
    openDrawer(fact.name, [
      ['the fact', node('p', 'derivation',
        'type ' + fact.fact_type + ' \u00b7 tier ' + fact.tier + ' \u00b7 basis ' + fact.basis +
        ' \u00b7 ' + fact.recall_count + ' recalls \u00b7 ' + fact.path)],
      ['edit in place', form]
    ]);
  }

  function field(name, label, value, kind) {
    var wrap = node('label', 'fact-field');
    wrap.appendChild(node('span', 'fact-field-label', label));
    var box = kind === 'area' ? document.createElement('textarea') : document.createElement('input');
    box.name = name;
    box.value = value == null ? '' : value;
    if (kind !== 'area') box.type = kind || 'text';
    wrap.appendChild(box);
    return wrap;
  }

  function readMemory(force) {
    var now = Date.now();
    if (!force && face.memory && now - face.memoryAt < 4000) return Promise.resolve();
    face.memoryAt = now;
    return fetch('/api/memory', { headers: { accept: 'application/json' } })
      .then(function (reply) {
        if (reply.status === 401) throw new Error('token');
        return reply.json();
      })
      .then(function (doc) {
        face.memory = doc;
        if (face.panel === 'memory') paintMemory();
      })
      .catch(function () { });
  }

  // The skills panel: the library with its loaded/skipped split, the
  // proposals awaiting a decision, and the curation candidates. The
  // approve rides improve's own apply, so its button says the gate is
  // running while it does — the gate is the real one, and a reply that
  // came back before it ran would be an approve of nothing.
  function paintSkills() {
    var read = face.skills;
    el.skillsBoard.textContent = '';
    if (!read) {
      var waiting = boardCard(12, 'skills', null, null);
      waiting.appendChild(node('p', 'board-empty',
        'The library read has not landed yet. It reads the canonical skills directory and the ' +
        'home\u2019s own proposals, both of which are this machine\u2019s.'));
      el.skillsBoard.appendChild(waiting);
      return;
    }
    var skills = read.skills || [];
    var skips = read.skips || [];
    var proposals = read.proposals || [];
    var tiles = node('div', 'tiles');
    tiles.appendChild(tile(skills.length, 'loaded'));
    tiles.appendChild(tile(skips.length, 'skipped', skips.length ? 'warn' : null));
    tiles.appendChild(tile(proposals.length, 'awaiting a decision', proposals.length ? 'live' : null));
    tiles.appendChild(tile(read.unused, 'never selected'));
    tiles.appendChild(tile(read.sessions, 'sessions scanned'));
    var head = boardCard(12, 'skills', null, 'the library at ' + (read.canonical || '\u2014'));
    head.appendChild(tiles);
    el.skillsBoard.appendChild(head);

    if (proposals.length) {
      el.skillsBoard.appendChild(proposalCards(proposals));
    } else {
      var none = boardCard(12, 'the decisions queue', null, null);
      none.appendChild(node('p', 'board-empty',
        'No proposal is awaiting a decision. A proposal is a drafted change under the home\u2019s own ' +
        'proposals directory, and an approve here rides the same apply the CLI rides.'));
      el.skillsBoard.appendChild(none);
    }

    if (skills.length) {
      var library = boardCard(12, 'the library', null, null);
      library.appendChild(skillGrid(skills));
      el.skillsBoard.appendChild(library);
    }
    if (skips.length) {
      el.skillsBoard.appendChild(skipCard(skips));
    }
    el.skillsBoard.appendChild(curationCard(read));
    if (read.refused && read.refused.length) {
      el.skillsBoard.appendChild(refusedCard(read.refused));
    }
  }

  // The proposals as cards, each with the approve that rides the apply.
  function proposalCards(proposals) {
    var card0 = boardCard(12, 'awaiting a decision', null,
      'an approve runs the four cargo steps the CLI runs, in the same order, and keeps the change ' +
      'only when every step is green. A skills write is home-side, so it creates no commit \u2014 ' +
      'the pre-image stash beside it is what makes it restorable by `fleet improve undo`.');
    var grid = node('div', 'fact-grid');
    proposals.forEach(function (proposal) {
      var item = node('article', 'fact-card');
      item.dataset.tone = 'live';
      item.appendChild(node('h3', 'fact-name', proposal.name));
      item.appendChild(node('span', 'fact-recalls', proposal.kind));
      var badges = node('div', 'fact-badges');
      badges.appendChild(badge(proposal.kind, 'accent'));
      item.appendChild(badges);
      item.appendChild(node('p', 'fact-said', proposal.description));
      item.appendChild(node('p', 'fact-meta', 'evidence: ' + clip(proposal.evidence, 120)));
      var approve = node('button', 'fact-save', 'approve');
      approve.type = 'button';
      approve.addEventListener('click', function (event) {
        event.stopPropagation();
        approve.disabled = true;
        approve.textContent = 'the gate is running\u2026';
        post('/api/skill', { name: proposal.name }).then(function (reply) {
          if (reply.status === 200) {
            approve.textContent = 'applied';
            notice('the proposal landed', reply.body.summary || proposal.name + ' applied');
            readSkills(true).then(paintSkills);
            return;
          }
          approve.disabled = false;
          approve.textContent = 'approve';
          notice('the approve was refused', reply.body.error || 'the apply refused it');
        }).catch(function () {
          approve.disabled = false;
          approve.textContent = 'approve';
          notice('the approve did not cross', 'the server did not answer');
        });
      });
      item.appendChild(approve);
      item.addEventListener('click', function () { inspectProposal(proposal); });
      grid.appendChild(item);
    });
    card0.appendChild(grid);
    return card0;
  }

  // The library as cards, each with its usage count: a skill the scan
  // never saw selected reads zero, which is the whole measure the
  // substrate reports.
  function skillGrid(skills) {
    var grid = node('div', 'fact-grid');
    skills.forEach(function (skill) {
      var item = node('article', 'fact-card');
      item.dataset.tone = skill.uses ? 'live' : 'retired';
      item.appendChild(node('h3', 'fact-name', skill.name));
      item.appendChild(node('span', 'fact-recalls',
        skill.uses + (skill.uses === 1 ? ' use' : ' uses')));
      item.appendChild(node('p', 'fact-said', skill.description));
      if (skill.modified) {
        item.appendChild(node('p', 'fact-meta', 'modified ' + clock(skill.modified)));
      }
      item.addEventListener('click', function () { inspectSkill(skill); });
      grid.appendChild(item);
    });
    return grid;
  }

  function skipCard(skips) {
    var card0 = boardCard(12, 'the library declined these', null,
      'a skip is the loader\u2019s own report, in its own words: a directory with no readable ' +
      'SKILL.md, a malformed one, or a proposal that reached the live tree.');
    var rows = node('div', 'rows');
    skips.forEach(function (skip) {
      var row0 = node('div', 'rrow');
      row0.appendChild(node('span', 'rrow-what', skip.name));
      var side = node('span', 'rrow-side');
      side.appendChild(node('span', 'rrow-outcome', clip(skip.reason, 90)));
      row0.appendChild(side);
      rows.appendChild(row0);
    });
    card0.appendChild(rows);
    return card0;
  }

  // The curation card: the merge candidates, scored by the substrate's
  // own rule at its own threshold, and the standing refusal that says
  // why a retire is not offered. Both come off the read, so the card and
  // the code cannot disagree.
  function curationCard(read) {
    var merges = read.merges || [];
    var card0 = boardCard(12, 'curation candidates', null, read.curation_note);
    if (!merges.length) {
      card0.appendChild(node('p', 'board-empty',
        'No two descriptions in the library read as near-identical. A candidate is a pair scoring ' +
        'at or above ' + read.threshold + ' token Jaccard \u2014 the same rule the substrate\u2019s ' +
        'own curation files a merge under.'));
      return card0;
    }
    var rows = node('div', 'rows');
    merges.forEach(function (merge) {
      var row0 = node('div', 'rrow');
      row0.appendChild(node('span', 'rrow-what',
        merge.survivor + ' would absorb ' + merge.absorbed));
      var side = node('span', 'rrow-side');
      side.appendChild(node('span', 'rrow-outcome', merge.similarity.toFixed(2)));
      row0.appendChild(side);
      rows.appendChild(row0);
    });
    card0.appendChild(rows);
    return card0;
  }

  function inspectSkill(skill) {
    openDrawer(skill.name, [
      ['the skill', node('p', 'derivation',
        skill.uses + ' uses across ' + (face.skills ? face.skills.sessions : 0) + ' scanned sessions' +
        ' \u00b7 ' + skill.path)],
      ['the body', node('pre', 'drawer-code', skill.body || '')],
      ['what the model sees', node('p', 'derivation',
        'the system prompt carries the name and the one sentence. The body enters context only when ' +
        'the skill is selected, and then it is already in the log as that call\u2019s result.')]
    ]);
  }

  function inspectProposal(proposal) {
    openDrawer(proposal.name, [
      ['the proposal', node('p', 'derivation',
        proposal.kind + ' \u00b7 ' + proposal.evidence + ' \u00b7 ' + proposal.path)],
      ['the proposed body', node('pre', 'drawer-code', proposal.body || '')],
      ['what an approve does', node('p', 'derivation',
        'the change is staged, the four cargo steps run in the repo, and the change is kept only ' +
        'when every step is green. A skills write is home-side, so it creates no commit; the ' +
        'pre-image stash beside it is what `fleet improve undo` restores.')]
    ]);
  }

  function readSkills(force) {
    var now = Date.now();
    if (!force && face.skills && now - face.skillsAt < 4000) return Promise.resolve();
    face.skillsAt = now;
    return fetch('/api/skills', { headers: { accept: 'application/json' } })
      .then(function (reply) {
        if (reply.status === 401) throw new Error('token');
        return reply.json();
      })
      .then(function (doc) {
        face.skills = doc;
        if (face.panel === 'skills') paintSkills();
      })
      .catch(function () { });
  }

  // ------------------------------------------------------------- actions

  function post(path, body) {
    return fetch(path, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(body)
    }).then(function (reply) {
      return reply.json().catch(function () { return {}; })
        .then(function (parsed) { return { status: reply.status, body: parsed }; });
    });
  }

  function send() {
    var task = el.task.value.trim();
    if (!task || face.running || face.seat) return;
    el.task.value = '';
    grow();
    var before = face.events.length;
    post('/api/run', { task: task }).then(function (reply) {
      if (reply.status === 200) return;
      // The run did not start. Sometimes the server has already written
      // why — the bridge's no-agent failure lands in the log as its own
      // event — and a note repeating it would be a second answer to one
      // question. So the log is re-read first, and the owner's words are
      // only added when the log has nothing new to say.
      return load().then(function () {
        if (face.events.length <= before) {
          notice('the run was refused', reply.body.error || 'the server did not take it');
        }
      });
    }).catch(function () {
      notice('the run was refused', 'the server did not answer');
    });
  }

  function answer(decision) {
    post('/api/approve', { decision: decision }).then(function (reply) {
      // The seat is already gone — the bridge clears it before it
      // replies — so a failure cannot be shown where it happened. A
      // `remember` the writer refused is the case that matters: the
      // call was allowed, the rule was not, and the owner must be able
      // to see the difference. A rule that did land needs no note; the
      // trust card is where the standing rules live, and it updates on
      // the server's own word.
      if (reply.body.error) notice('the answer crossed, but', reply.body.error);
    }).catch(function () {
      notice('the answer did not cross', 'the server did not answer; the call was refused');
    });
  }

  // ----------------------------------------------------------- inspector

  function openDrawer(title, sections) {
    el.drawerTitle.textContent = title;
    el.drawerBody.textContent = '';
    sections.forEach(function (section) {
      var head = node('h3', null, section[0]);
      el.drawerBody.appendChild(head);
      if (section[1] instanceof Node) {
        el.drawerBody.appendChild(section[1]);
      } else {
        var para = node('p', 'derivation');
        para.innerHTML = section[1];
        el.drawerBody.appendChild(para);
      }
    });
    el.drawer.hidden = false;
    el.scrim.hidden = false;
  }

  function closeDrawer() {
    el.drawer.hidden = true;
    el.scrim.hidden = true;
  }

  function inspect(event) {
    var raw = node('pre', 'raw', JSON.stringify(event, null, 2));
    var sections = [];
    if (event) {
      sections.push(['the event', raw]);
    } else {
      sections.push(['the log', node('p', 'derivation',
        face.events.length + ' events in order. The header and the rail are reduced from them.')]);
    }
    var pct = face.budget ? Math.min(100, Math.round((face.context / face.budget) * 100)) : 0;
    sections.push(['where the numbers come from',
      '<code>spend</code> is every <code>usage</code> event\u2019s prompt plus completion, summed \u2014 ' +
      tokens(face.spend) + ' tokens.<br>' +
      '<code>context</code> is the last <code>usage</code> event\u2019s prompt against the budget of ' +
      tokens(face.budget) + ' \u2014 ' + pct + '%.<br>' +
      '<code>route</code> is the last <code>routed</code> event, not a read of the config.<br>' +
      '<code>turns</code> is the last <code>turn</code> event.']);
    openDrawer(event ? (event.type || 'event') : 'inspector', sections);
  }

  // ------------------------------------------------------- the replaying

  // The snapshot is the whole log the server holds. The stream is the
  // live half of the same log, so the two are one derivation run twice:
  // a refresh replays, a stream appends, and neither has a number the
  // other does not.
  function replay(state) {
    face.session = state.session || '';
    face.model = state.model || '';
    face.budget = Number(state.budget) || 0;
    face.running = !!state.running;
    face.dropped = Number(state.dropped) || 0;
    face.rules = state.rules || [];
    face.seat = state.approval || null;
    face.events = [];
    face.rows = [];
    face.assistant = null;
    face.pending = [];
    face.delegations = {};
    face.lastTurn = 0;
    face.buffer = '';
    face.spend = 0;
    face.context = 0;
    face.turn = 0;
    face.route = null;
    face.stop = null;
    face.seatKey = '';

    el.log.textContent = '';
    face.stream = node('div', 'stream');
    el.log.appendChild(face.stream);
    el.log.hidden = false;
    el.empty.hidden = true;
    (state.events || []).forEach(apply);
    face.stick = true;
    settle();
    paint();
  }

  // An event recorded between the snapshot and the rebuild rides the
  // stream into a log the rebuild then wipes. The count is the tell: a
  // rebuild short of what the page already held has lost something, and
  // one more read closes the gap rather than leaving the face quietly
  // missing a turn. Three is where it stops asking.
  function load(pass) {
    pass = pass || 0;
    return fetch('/api/state', { headers: { accept: 'application/json' } })
      .then(function (reply) {
        if (reply.status === 401) throw new Error('token');
        return reply.json();
      })
      .then(function (state) {
        var held = face.events.length;
        replay(state);
        if (face.events.length < held && pass < 2) return load(pass + 1);
      });
  }

  // The stream is the live half of the same log. Its hello is the
  // moment the server has the connection, which is also the moment to
  // re-read the state: anything recorded between the replay and the
  // subscribe would otherwise be missed, and a reconnect starts clean
  // rather than appending to a log the server has moved past.
  function listen() {
    var source = new EventSource('/events?token=' + encodeURIComponent(token));
    source.onopen = function () {
      load().then(function () { paint(); }).catch(function () { });
    };
    source.onmessage = function (event) {
      var data = JSON.parse(event.data);
      if (data.type === 'hello') return;
      if (data.type === 'heartbeat') return;
      apply(data);
      settle();
    };
    source.onerror = function () {
      el.statePill.textContent = 'stream down';
      el.statePill.dataset.state = 'down';
    };
    return source;
  }

  // ------------------------------------------------------------- the boot

  if (!token) {
    el.empty.hidden = false;
    el.log.hidden = true;
    el.empty.querySelector('h1').textContent = 'This page was opened without its token.';
    el.empty.querySelector('p').textContent =
      'Use the URL fleet printed \u2014 it carries the launch token \u2014 then reload.';
    return;
  }
  // The token leaves the address bar, so a copied URL or a screenshot
  // carries no credential. The hash stays: it is the panel the owner
  // opened, and the plan's route table is keyed on it.
  window.history.replaceState(null, '', window.location.pathname + window.location.hash);

  el.offers.textContent = '';
  OFFERS.forEach(function (offer) {
    var button = node('button', 'offer', offer);
    button.type = 'button';
    button.addEventListener('click', function () {
      el.task.value = offer;
      grow();
      el.task.focus();
    });
    el.offers.appendChild(button);
  });

  el.composer.addEventListener('submit', function (event) {
    event.preventDefault();
    send();
  });

  el.task.addEventListener('input', grow);
  el.task.addEventListener('keydown', function (event) {
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault();
      send();
    }
  });

  function grow() {
    el.task.style.height = 'auto';
    el.task.style.height = Math.min(200, el.task.scrollHeight) + 'px';
  }

  el.drawerClose.addEventListener('click', closeDrawer);
  el.scrim.addEventListener('click', closeDrawer);
  el.inspectFace.addEventListener('click', function () { inspect(null); });
  document.addEventListener('keydown', function (event) {
    if (event.key === 'Escape') closeDrawer();
  });

  // Any card on the face opens the inspector on its own event.
  el.log.addEventListener('click', function (event) {
    var opened = event.target.closest ? event.target.closest('.card-row') : null;
    if (!opened) return;
    var made = face.rows.filter(function (candidate) { return candidate.el === opened; })[0];
    inspect(made ? made.event : null);
  });

  // The log follows the tail until the owner reads backwards. A face
  // that yanked them down on the next token would be fighting them, so
  // the scroll position decides, not the code.
  el.log.addEventListener('scroll', function () {
    face.stick = stick();
  });

  // ------------------------------------------------- the hash addresses

  function showRoute() {
    var which = (window.location.hash || '').replace(/^#\/?/, '');
    var known = ROUTES.filter(function (route) { return route[0] === which; })[0];
    var address = which === 'chat' || !known ? 'chat' : which;
    face.panel = address;
    markRail();
    if (address === 'chat') {
      el.panelChat.hidden = false;
      el.panelRouter.hidden = true;
      el.panelDelegation.hidden = true;
      el.panelMemory.hidden = true;
      el.panelSkills.hidden = true;
      el.panelLater.hidden = true;
      return;
    }
    if (address === 'router') {
      el.panelChat.hidden = true;
      el.panelLater.hidden = true;
      el.panelRouter.hidden = false;
      // The board's read, fetched on arrival and then on the panel's own
      // clock. The live decision is already in the log, so it paints now
      // and again with the stream.
      paintRouter();
      readRouter(true).then(paintRouter);
      return;
    }
    if (address === 'delegation') {
      el.panelChat.hidden = true;
      el.panelLater.hidden = true;
      el.panelRouter.hidden = true;
      el.panelMemory.hidden = true;
      el.panelSkills.hidden = true;
      el.panelDelegation.hidden = false;
      paintDelegation();
      readDelegation(true).then(paintDelegation);
      return;
    }
    if (address === 'memory') {
      el.panelChat.hidden = true;
      el.panelLater.hidden = true;
      el.panelRouter.hidden = true;
      el.panelDelegation.hidden = true;
      el.panelSkills.hidden = true;
      el.panelMemory.hidden = false;
      paintMemory();
      readMemory(true).then(paintMemory);
      return;
    }
    if (address === 'skills') {
      el.panelChat.hidden = true;
      el.panelLater.hidden = true;
      el.panelRouter.hidden = true;
      el.panelDelegation.hidden = true;
      el.panelMemory.hidden = true;
      el.panelSkills.hidden = false;
      paintSkills();
      readSkills(true).then(paintSkills);
      return;
    }
    var named = known || [which, which];
    el.laterTitle.textContent = named[1];
    el.laterNote.textContent = named[1] + ' is a later increment of the same plan. ' +
      'The conversation is live now, and every surface reads the one event log this face already holds.';
    el.panelChat.hidden = true;
    el.panelRouter.hidden = true;
    el.panelDelegation.hidden = true;
    el.panelMemory.hidden = true;
    el.panelSkills.hidden = true;
    el.panelLater.hidden = false;
  }

  // ------------------------------------------------- the rail and the palette

  // The rail: four destinations, one flat level beneath each. Built from
  // GROUPS, the same declaration the palette searches, so the two can
  // never disagree about where a surface lives.
  function buildRail() {
    el.rail.textContent = '';
    GROUPS.forEach(function (group) {
      var id = group[0], label = group[1], items = group[2];
      var head = node('span', 'rail-group', label);
      el.rail.appendChild(head);
      var button = node('button', 'rail-dest');
      button.type = 'button';
      button.dataset.group = id;
      button.appendChild(node('span', 'rail-mark', GROUP_MARKS[id] || label.slice(0, 1)));
      button.appendChild(node('span', 'rail-name', label));
      // The destination itself opens its first surface — the one an owner
      // opening the group almost always wants — so a group is one click
      // rather than two.
      button.addEventListener('click', function () {
        window.location.hash = '#/' + items[0][0];
      });
      el.rail.appendChild(button);
      var sub = node('div', 'rail-sub');
      items.forEach(function (item) {
        var address = item[0], name = item[1];
        var link = node('button', 'rail-link');
        link.type = 'button';
        link.dataset.address = address;
        link.textContent = name;
        link.addEventListener('click', function () {
          window.location.hash = '#/' + address;
        });
        sub.appendChild(link);
      });
      el.rail.appendChild(sub);
    });
    var open = node('button', 'rail-palette');
    open.type = 'button';
    open.appendChild(node('span', 'rail-mark', '\u2318'));
    open.appendChild(node('span', 'rail-name', 'jump to anything'));
    open.addEventListener('click', function () { openPalette(); });
    el.rail.appendChild(open);
  }

  // Which surfaces are live today, and which are a later increment — the
  // rail says so rather than pretending all ten are built.
  var BUILT = ['chat', 'router', 'delegation', 'memory', 'skills'];

  function markRail() {
    var which = (window.location.hash || '').replace(/^#\/?/, '');
    Array.prototype.forEach.call(el.rail.querySelectorAll('.rail-link'), function (link) {
      if (link.dataset.address === which) {
        link.setAttribute('aria-current', 'page');
      } else {
        link.removeAttribute('aria-current');
      }
    });
    Array.prototype.forEach.call(el.rail.querySelectorAll('.rail-dest'), function (button) {
      var here = WHERE[which] && WHERE[which][0] === button.dataset.group;
      if (here) {
        button.setAttribute('aria-current', 'page');
      } else {
        button.removeAttribute('aria-current');
      }
    });
  }

  // The rail's collapsed state, remembered across launches: deep in a run
  // the rail is chrome, and while navigating it is the map, so the one
  // switch worth offering is a mode rather than a fork of the structure.
  function railCollapsed() {
    try { return window.localStorage.getItem('fleet-rail') === 'icons'; } catch (error) { return false; }
  }

  function setRail(mode) {
    document.body.dataset.rail = mode;
    el.railToggle.setAttribute('aria-expanded', mode === 'icons' ? 'false' : 'true');
    el.railToggle.setAttribute('aria-label',
      mode === 'icons' ? 'expand the surfaces rail' : 'collapse the surfaces rail');
    try { window.localStorage.setItem('fleet-rail', mode); } catch (error) { }
  }

  // ------------------------------------------------------ the palette

  // The palette's rows: every surface, grouped by the same destinations
  // the rail shows. A result that matches nothing is dropped with its
  // group, so an empty group never shows a heading over nothing.
  function paletteRows(asked) {
    var query = (asked || '').trim().toLowerCase();
    var groups = [];
    GROUPS.forEach(function (group) {
      var hits = group[2].filter(function (item) {
        if (!query) return true;
        var hay = (item[1] + ' ' + item[0] + ' ' + group[1]).toLowerCase();
        return hay.indexOf(query) !== -1;
      });
      if (hits.length) groups.push([group, hits]);
    });
    return groups;
  }

  function paintPalette() {
    var asked = el.paletteInput.value;
    var groups = paletteRows(asked);
    el.paletteRows.textContent = '';
    var shown = 0;
    var first = null;
    groups.forEach(function (entry) {
      var group = entry[0], hits = entry[1];
      el.paletteRows.appendChild(node('span', 'palette-group', group[1]));
      hits.forEach(function (item) {
        var row = node('button', 'palette-row');
        row.type = 'button';
        row.setAttribute('role', 'option');
        row.dataset.address = item[0];
        row.appendChild(node('span', 'palette-row-mark', GROUP_MARKS[group[0]] || group[1].slice(0, 1)));
        row.appendChild(node('span', 'palette-row-name', item[1]));
        // Whether the surface is live is on the row, not hidden: a later
        // increment says so here rather than opening to a surprise.
        row.appendChild(node('span', 'palette-row-hint',
          BUILT.indexOf(item[0]) === -1 ? 'later' : 'open'));
        row.addEventListener('click', function () { choosePalette(item[0]); });
        if (!first) first = row;
        shown += 1;
        el.paletteRows.appendChild(row);
      });
    });
    if (!shown) {
      el.paletteRows.appendChild(node('p', 'palette-none',
        'No surface matches \u201c' + asked + '\u201d. The palette reaches the ten surfaces ' +
        'and their groups; a run is asked for in the composer.'));
    }
    el.paletteCount.textContent = shown ? shown + (shown === 1 ? ' result' : ' results') : '';
    if (first) {
      first.setAttribute('aria-selected', 'true');
      face.paletteAt = 0;
      face.paletteRows = Array.prototype.slice.call(el.paletteRows.querySelectorAll('.palette-row'));
    } else {
      face.paletteRows = [];
    }
  }

  function movePalette(step) {
    var rows = face.paletteRows || [];
    if (!rows.length) return;
    var at = face.paletteAt || 0;
    rows[at].removeAttribute('aria-selected');
    var next = (at + step + rows.length) % rows.length;
    rows[next].setAttribute('aria-selected', 'true');
    face.paletteAt = next;
    rows[next].scrollIntoView({ block: 'nearest' });
  }

  function choosePalette(address) {
    closePalette();
    window.location.hash = '#/' + address;
  }

  function openPalette() {
    el.palette.hidden = false;
    el.paletteScrim.hidden = false;
    el.paletteInput.value = '';
    paintPalette();
    el.paletteInput.focus();
  }

  function closePalette() {
    el.palette.hidden = true;
    el.paletteScrim.hidden = true;
    el.paletteInput.value = '';
    el.paletteInput.blur();
  }

  // The theme. One attribute on the document element is the whole
  // switch — the alias tier re-points under it and every component token
  // follows — so there is one thing to set and nothing else to keep in
  // step. Dark is the default; the choice is remembered.
  function themeNow() {
    return document.documentElement.dataset.theme === 'light' ? 'light' : 'dark';
  }

  function setTheme(mode) {
    document.documentElement.dataset.theme = mode;
    var light = mode === 'light';
    el.themeToggle.setAttribute('aria-label',
      light ? 'switch to the dark theme' : 'switch to the light theme');
    el.themeToggle.title = light ? 'switch to the dark theme' : 'switch to the light theme';
    el.themeToggle.dataset.state = light ? 'light' : 'dark';
    try { window.localStorage.setItem('fleet-theme', mode); } catch (error) { }
  }

  setTheme(themeNow());
  el.themeToggle.addEventListener('click', function () {
    setTheme(themeNow() === 'light' ? 'dark' : 'light');
  });

  buildRail();
  setRail(railCollapsed() ? 'icons' : 'full');
  el.railToggle.addEventListener('click', function () {
    setRail(railCollapsed() ? 'full' : 'icons');
  });
  el.paletteOpen.addEventListener('click', openPalette);
  el.paletteScrim.addEventListener('click', closePalette);
  el.paletteInput.addEventListener('input', paintPalette);
  el.paletteInput.addEventListener('keydown', function (event) {
    if (event.key === 'ArrowDown') { event.preventDefault(); movePalette(1); }
    else if (event.key === 'ArrowUp') { event.preventDefault(); movePalette(-1); }
    else if (event.key === 'Enter') {
      event.preventDefault();
      var rows = face.paletteRows || [];
      if (rows.length) choosePalette(rows[face.paletteAt || 0].dataset.address);
    }
  });
  // The palette's own key, and Esc from anywhere: the one keystroke that
  // reaches every surface, and the one that always comes home.
  window.addEventListener('keydown', function (event) {
    var typing = document.activeElement === el.paletteInput
      || document.activeElement === el.task;
    if ((event.metaKey || event.ctrlKey) && (event.key === 'k' || event.key === 'K')) {
      event.preventDefault();
      if (el.palette.hidden) openPalette(); else closePalette();
      return;
    }
    if (event.key !== 'Escape') return;
    if (!el.palette.hidden) { closePalette(); return; }
    var which = (window.location.hash || '').replace(/^#\/?/, '');
    if (which && which !== 'chat') window.location.hash = '#/chat';
  });

  window.addEventListener('hashchange', showRoute);
  showRoute();

  // The board's read on the panel's own clock, and only while the panel
  // is showing: a day's ledger moves without the log saying anything, and
  // a face nobody is looking at has nothing to refresh. The delegation
  // read rides the same clock for the same reason — a run delegates
  // while the owner watches the tree grow. The memory and skills reads
  // do too: the corpus changes when the agent writes a fact, and the
  // library changes when a proposal lands.
  window.setInterval(function () {
    if (face.panel === 'router') readRouter(false);
    if (face.panel === 'delegation') readDelegation(false);
    if (face.panel === 'memory') readMemory(false);
    if (face.panel === 'skills') readSkills(false);
  }, 5000);

  load().then(listen).catch(function (error) {
    if (error && error.message === 'token') {
      el.empty.hidden = false;
      el.empty.querySelector('h1').textContent = 'This page is not the one fleet printed.';
      el.empty.querySelector('p').textContent =
        'The launch token did not arrive with this request. Open the URL fleet printed.';
    }
  });
})();
