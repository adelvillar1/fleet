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

  // The composer's commands. Actions the page performs itself, which is
  // what separates them from the palette's surfaces: a slash command
  // changes the face or answers the seat, and everything else is a task
  // for the agent. One declaration, ranked two ways — lexically for the
  // instant answer and semantically against these descriptions when the
  // machine's embedding backend is up.
  var COMMANDS = [
    ['theme', ['paper', 'graphite', 'contrast', 'mode'], 'choose a theme \u2014 console, paper, graphite or contrast',
      function () { openThemes(); }],
    ['pane', ['files', 'code', 'preview', 'explore'], 'open or close the files and deliverables pane',
      function () { if (el.pane.hidden) openPane(); else closePane(); }],
    ['go', ['jump', 'open'], 'jump to a surface by name',
      function (rest) { if (rest) window.location.hash = '#/' + rest.trim().split(/\s+/)[0]; }],
    ['compact', ['fold', 'summarise'], 'fold the conversation now, without waiting for the budget',
      function () {
        // The flag is the browser's own: the agent reads its forced-fold
        // once, when the run starts, so arming it here does not fold
        // anything yet. Saying otherwise would be a command that claims
        // a result it does not produce.
        notice('the fold is not wired yet',
          'the agent reads its forced-fold flag once, when a run starts; arming it mid-run needs a shared flag the agent reads per turn');
      }],
    ['clear', ['new', 'reset'], 'clear the transcript and start a fresh session view',
      function () { window.location.hash = '#/chat'; load(true); }],
    ['approve', ['allow', 'yes'], 'answer the waiting approval: once, always, or deny',
      function (rest) { answer(rest.trim() || 'once'); }],
    ['help', ['commands', '?'], 'list every command and what it does',
      function () { openDrawer('commands', commandHelp()); }],
    ['review', ['diff', 'changes'], 'review this session’s file changes against a checkpoint',
      function () { openReview(); }]
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
    pane: document.getElementById('pane'),
    paneOpen: document.getElementById('pane-open'),
    commands: document.getElementById('commands'),
    paneClose: document.getElementById('pane-close'),
    paneTree: document.getElementById('pane-tree'),
    paneFile: document.getElementById('pane-file'),
    panePreview: document.getElementById('pane-preview-body'),
    tabFiles: document.getElementById('tab-files'),
    tabPreview: document.getElementById('tab-preview'),
    tabWeb: document.getElementById('tab-web'),
    paneWeb: document.getElementById('pane-web'),
    paneAddress: document.getElementById('pane-address'),
    paneAddressBox: document.getElementById('pane-address-box'),
    panePage: document.getElementById('pane-page'),
    paneFiles: document.getElementById('pane-files'),
    panePreviewPanel: document.getElementById('pane-preview'),
    themeToggle: document.getElementById('theme-toggle'),
    themes: document.getElementById('themes'),
    actions: document.getElementById('actions'),
    panelChat: document.getElementById('panel-chat'),
    panelRouter: document.getElementById('panel-router'),
    panelDelegation: document.getElementById('panel-delegation'),
    panelMemory: document.getElementById('panel-memory'),
    panelSkills: document.getElementById('panel-skills'),
    panelSessions: document.getElementById('panel-sessions'),
    panelCalibration: document.getElementById('panel-calibration'),
    panelTower: document.getElementById('panel-tower'),
    panelSecondBrain: document.getElementById('panel-second-brain'),
    panelSettings: document.getElementById('panel-settings'),
    panelLater: document.getElementById('panel-later'),
    laterTitle: document.getElementById('later-title'),
    laterNote: document.getElementById('later-note'),
    routerBoard: document.getElementById('router-board'),
    delegationBoard: document.getElementById('delegation-board'),
    memoryBoard: document.getElementById('memory-board'),
    skillsBoard: document.getElementById('skills-board'),
    sessionsBoard: document.getElementById('sessions-board'),
    calibrationBoard: document.getElementById('calibration-board'),
    towerBoard: document.getElementById('tower-board'),
    brainBoard: document.getElementById('brain-board'),
    settingsBoard: document.getElementById('settings-board'),
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
    plan: document.getElementById('plan'),
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
    // Plan mode: the composer's second door, the browser's own state.
    planMode: false,
    planned: false,
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
    // The sessions panel: the archive's read and its clock, and the one
    // replay the panel has open. The replay is one read per id, so
    // reopening a row is the cached document and not a second read.
    sessions: null,
    sessionsAt: 0,
    replay: null,
    replayId: null,
    // The calibration panel's read and its clock.
    calibration: null,
    calibrationAt: 0,
    // The tower console's and the second-brain panel's reads and clocks.
    tower: null,
    towerAt: 0,
    brain: null,
    brainAt: 0,
    // The settings surface's read and its clock.
    settings: null,
    settingsAt: 0,
    // The pane's web tab: the page in front and the address it came
    // from. One read per address, so re-reading is a refresh and not a
    // second fetch of the same page.
    page: null,
    pageAt: '',
    // The theme set's measured contrast, which is what the action card
    // offers a fix against.
    contrast: null,
    // The palette's rows and which of them is selected, so the arrow keys
    // move a cursor the same way the panel reads move theirs.
    paletteRows: [],
    paletteAt: 0,
    // The pane: which directory the tree is showing, which file is open,
    // and which of its two views is in front. The tree is one read per
    // directory and the file is one read per open, so neither polls. A
    // deck the run built rides here too, because the pane is where it
    // renders full-page.
    tree: null,
    treeAt: '',
    file: null,
    deck: null,
    paneTab: 'files'
  };

  // A short mark for a tool, so a card reads at a glance without an
  // icon set shipped beside the binary. The first letter is the honest
  // fallback: it is always the tool's own.
  var GLYPHS = {
    bash: '>', read: '<', write: 'w', edit: 'e', search: '?',
    recall: 'm', skill: 's', task: '*', delegate: '*', web: '@',
    deck: '\u25c7'
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
      case 'deck':
        deckRow(event);
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
      case 'checkpoint':
        checkpointRow(event);
        break;
      case 'rewound':
        rewoundRow(event);
        break;
      case 'plan':
        planRow(event);
        break;
      case 'finished':
      case 'failed':
        face.assistant = null;
        face.stop = event;
        endRow(event);
        // A plan-mode run's answer is a plan: the card it lands in is
        // the editable one, so the owner edits and builds.
        if (face.planned && event.type === 'finished') {
          face.planned = false;
          finishPlan();
        }
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
      // A face that has not run yet has no session: the log is created
      // on the first task (KI-022), so the row reads a dash until one
      // exists rather than naming an id that does not.
      ['session', face.session || '\u2014'],
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
    if (face.planMode) { planSend(); return; }
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

  // The deck, full-page in the pane. The fragment references the face's
  // own tokens, so it inherits the theme switch — and because the browser
  // is what renders it, a full-page view is also what makes a screenshot
  // one keystroke, which is the image the `vision` tool grades.
  function showDeck(event) {
    face.file = null;
    face.deck = event;
    showPaneTab('preview');
    openPane();
    paintPreview();
  }

  // The deck in the inspector: the same fragment, beside the run that
  // produced it.
  function inspectDeck(event) {
    openDrawer('a ' + event.project + ' of ' + event.nodes + ' nodes', [
      ['the deck', node('div', 'deck-frame', event.html || '')]
    ]);
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

  el.task.addEventListener('input', function () {
    grow();
    typeAhead();
  });
  el.task.addEventListener('keydown', function (event) {
    // The type-ahead owns the keys while it is open, so a command is one
    // keystroke from anywhere in the composer.
    if (face.commandOpen) {
      if (event.key === 'ArrowDown') { event.preventDefault(); moveCommand(1); return; }
      if (event.key === 'ArrowUp') { event.preventDefault(); moveCommand(-1); return; }
      if (event.key === 'Tab') {
        event.preventDefault();
        if (face.commands.length) {
          runCommand(face.commands[face.commandAt], el.task.value.replace(/^\/[^\s]*\s*/, ''));
        }
        return;
      }
      if (event.key === 'Escape') { event.preventDefault(); closeCommands(); return; }
    }
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault();
      // A slash line runs its command rather than reaching the agent.
      var line = el.task.value.trim();
      if (line.charAt(0) === '/') {
        var parts = line.slice(1).split(/\s+/);
        var name = (parts.shift() || '').toLowerCase();
        var rest = parts.join(' ');
        var at = -1;
        COMMANDS.forEach(function (command, index) {
          if (command[0] === name || command[1].indexOf(name) !== -1) at = index;
        });
        if (at !== -1) { runCommand(at, rest); return; }
      }
      send();
    }
  });

  // What the composer is showing, from what it holds. A leading slash
  // opens the type-ahead; anything else closes it, and a backspace past
  // the slash closes it too.
  function typeAhead() {
    var value = el.task.value;
    if (value.charAt(0) !== '/') { closeCommands(); return; }
    var asked = value.slice(1).split(/\s+/)[0].toLowerCase();
    if (!asked) { openCommands(''); return; }
    if (asked === face.commandAsk) return;
    face.commandAsk = asked;
    openCommands(asked);
  }

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
    var event = made ? made.event : null;
    // A deck's card opens the rendered deck rather than the raw event:
    // the fragment is the deliverable, and the raw JSON is the thing
    // nobody asked for.
    // A deck opens full-page rather than in the inspector: the browser
    // renders it, so the pane is the surface it was made for.
    if (event && event.type === 'deck') { showDeck(event); return; }
    inspect(event);
  });

  // The log follows the tail until the owner reads backwards. A face
  // that yanked them down on the next token would be fighting them, so
  // the scroll position decides, not the code.
  el.log.addEventListener('scroll', function () {
    face.stick = stick();
  });

  // ------------------------------------------------- the hash addresses

  // -------------------------------------------------------- the sessions panel

  // The archive read: the log heads as summaries, newest first, with
  // the session this face anchors named.
  function readSessions(force) {
    var now = Date.now();
    if (!force && face.sessions && now - face.sessionsAt < 4000) return Promise.resolve();
    face.sessionsAt = now;
    return fetch('/api/sessions', { headers: { accept: 'application/json' } })
      .then(function (reply) {
        if (reply.status === 401) throw new Error('token');
        return reply.json();
      })
      .then(function (doc) {
        face.sessions = doc;
        if (face.panel === 'sessions') paintSessions();
      })
      .catch(function () { });
  }

  // One session's replay. One read per id, so reopening a row is the
  // cached document and opening a different row is the only fetch.
  function readReplay(id, force) {
    if (!force && face.replayId === id && face.replay) return Promise.resolve();
    return fetch('/api/session?id=' + encodeURIComponent(id), {
      headers: { accept: 'application/json' }
    })
      .then(function (reply) {
        if (reply.status === 401) throw new Error('token');
        return reply.json();
      })
      .then(function (doc) {
        face.replayId = id;
        face.replay = doc;
        if (face.panel === 'sessions') paintSessions();
      })
      .catch(function () { });
  }

  function paintSessions() {
    var read = face.sessions;
    el.sessionsBoard.textContent = '';
    if (!read) {
      var waiting = boardCard(12, 'sessions', null, null);
      waiting.appendChild(node('p', 'board-empty',
        'The archive read has not landed yet. It reads the session logs under this home, ' +
        'which are this machine\u2019s own.'));
      el.sessionsBoard.appendChild(waiting);
      return;
    }
    if (read.refused && read.refused.length) {
      el.sessionsBoard.appendChild(refusedCard(read.refused));
      return;
    }
    var sessions = read.sessions || [];
    var children = sessions.filter(function (row) { return row.kind === 'subagent'; }).length;
    var compacted = sessions.filter(function (row) { return row.compacted; }).length;
    var open = sessions.filter(function (row) { return !row.ended; }).length;
    var tiles = node('div', 'tiles');
    tiles.appendChild(tile(sessions.length, 'logs in the archive'));
    tiles.appendChild(tile(open, 'without an end record', open ? 'warn' : null));
    tiles.appendChild(tile(children, 'delegated children'));
    tiles.appendChild(tile(compacted, 'compacted at least once', compacted ? 'warn' : null));
    var head = boardCard(12, 'sessions', read.read_at,
      'the archive under this home \u00b7 the row this face anchors is marked');
    head.appendChild(tiles);
    el.sessionsBoard.appendChild(head);

    if (sessions.length) {
      var list = boardCard(12, 'the archive', null, 'a row opens its replay beside the list');
      sessions.forEach(function (row) { list.appendChild(sessionRow(row)); });
      el.sessionsBoard.appendChild(list);
    } else {
      var none = boardCard(12, 'the archive', null, null);
      none.appendChild(node('p', 'board-empty',
        'This home holds no session logs yet. A run writes one log per session, whether it ' +
        'starts here, in the TUI, or headless.'));
      el.sessionsBoard.appendChild(none);
    }
    if (face.replayId) paintReplayInto(el.sessionsBoard, face.replay, read.current);
  }

  // One archive row: the id, the model, the markers the summary
  // carries, and a badge when it is the session this face anchors. A
  // click opens the replay; a double purpose needs no second button.
  function sessionRow(row) {
    var line = node('button', 'session-row');
    line.type = 'button';
    line.dataset.id = row.id;
    var name = node('span', 'session-row-id', row.id);
    if (row.current) name.appendChild(badge('this face', 'accent'));
    line.appendChild(name);
    line.appendChild(node('span', 'session-row-model', row.model || 'unreadable'));
    var marks = node('span', 'session-row-marks');
    if (row.kind) marks.appendChild(badge(row.kind, null));
    if (row.compacted) marks.appendChild(badge('compacted', 'warn'));
    marks.appendChild(badge(row.ended ? 'ended' : 'open', row.ended ? null : 'live'));
    line.appendChild(marks);
    line.appendChild(node('span', 'session-row-created', row.created || '\u2014'));
    line.addEventListener('click', function () { readReplay(row.id, true); });
    return line;
  }

  // The replay card: the head the start record pinned, the summary the
  // end record and the compaction marks carry, the resume that
  // re-anchors this face, and the cards themselves.
  function paintReplayInto(board, replay, current) {
    if (!replay) return;
    if (replay.refused && replay.refused.length) {
      board.appendChild(refusedCard(replay.refused));
      return;
    }
    var summary = replay.model + (replay.kind ? ' \u00b7 ' + replay.kind : '') +
      ' \u00b7 ' + (replay.ended
        ? 'ended: ' + replay.stop_reason + ' \u00b7 ' + replay.turns + ' turns'
        : 'no end record \u2014 the run was interrupted');
    var card = boardCard(12, 'replay \u00b7 ' + replay.id, replay.created, summary);
    var actions = node('div', 'replay-actions');
    var back = node('button', 'replay-button');
    back.type = 'button';
    back.textContent = 'close the replay';
    back.addEventListener('click', function () {
      face.replayId = null;
      face.replay = null;
      paintSessions();
    });
    actions.appendChild(back);
    var resume = node('button', 'replay-button replay-resume');
    resume.type = 'button';
    if (replay.id === current) {
      resume.disabled = true;
      resume.textContent = 'this face anchors this session';
    } else if (face.running) {
      resume.disabled = true;
      resume.textContent = 'a run is in flight';
    } else {
      resume.textContent = 'resume this session here';
      resume.addEventListener('click', function () { resumeSession(replay.id); });
    }
    actions.appendChild(resume);
    card.appendChild(actions);
    if (replay.omitted) {
      card.appendChild(node('p', 'board-empty',
        replay.omitted + ' earlier card' + (replay.omitted === 1 ? ' is' : 's are') +
        ' left out of this preview. The log holds them; the preview shows the newest ' +
        'and the resume reseeds from the whole log.'));
    }
    var lines = node('div', 'replay-lines');
    (replay.cards || []).forEach(function (entry) { lines.appendChild(replayLine(entry)); });
    if (!(replay.cards || []).length) {
      lines.appendChild(node('p', 'board-empty', 'This log carries no messages yet.'));
    }
    card.appendChild(lines);
    board.appendChild(card);
  }

  // One replayed record, as a line. The vocabulary is the live
  // transcript's, narrowed to what a log can reconstruct.
  function replayLine(entry) {
    var line = node('div', 'replay-line');
    line.dataset.kind = entry.type;
    if (entry.type === 'message') {
      line.dataset.role = entry.role;
      line.appendChild(node('span', 'replay-who', entry.role === 'owner' ? 'you' : 'fleet'));
      line.appendChild(node('span', 'replay-text', clip(entry.text || '', 400)));
    } else if (entry.type === 'tool_result') {
      line.appendChild(node('span', 'replay-who', 'tool \u00b7 ' + entry.tool));
      line.appendChild(node('span', 'replay-text', entry.preview || ''));
      line.appendChild(badge(entry.approved ? 'approved' : 'denied',
        entry.approved ? null : 'down'));
    } else if (entry.type === 'lane') {
      line.appendChild(node('span', 'replay-who', 'lane \u00b7 ' + entry.lane));
      line.appendChild(node('span', 'replay-text', entry.tools + ' tools joined the run'));
    } else if (entry.type === 'skills') {
      line.appendChild(node('span', 'replay-who', 'skills'));
      line.appendChild(node('span', 'replay-text', entry.loaded + ' declared on the run'));
    } else if (entry.type === 'memory') {
      line.appendChild(node('span', 'replay-who', 'memory'));
      line.appendChild(node('span', 'replay-text',
        entry.facts + ' facts rode the turn\u2019s context' +
        (entry.shape ? ' \u00b7 shape ' + entry.shape : ' \u00b7 unclassified')));
    } else if (entry.type === 'compacted') {
      line.appendChild(node('span', 'replay-who', 'compacted'));
      line.appendChild(node('span', 'replay-text',
        entry.turns + ' turns became one summary \u00b7 ' +
        entry.before + ' \u2192 ' + entry.after + ' tokens'));
    } else {
      line.appendChild(node('span', 'replay-who', entry.type));
    }
    return line;
  }

  // The resume: re-anchor this face to the replayed log. The reply says
  // whether an agent took the anchor, because a face without one still
  // resumes as a viewer, and saying nothing would leave the owner
  // guessing which of the two happened.
  function resumeSession(id) {
    post('/api/resume', { session: id }).then(function (reply) {
      if (reply.status !== 200) {
        notice('the resume was refused',
          (reply.body && reply.body.error) || 'the face said no without a reason');
        return;
      }
      face.sessions = null;
      face.replay = null;
      face.replayId = null;
      notice('resumed ' + id, reply.body.agent === false
        ? 'this face has no agent behind it; the conversation is here to read'
        : 'the next message continues that conversation');
      window.location.hash = '#/chat';
      load(true);
    }).catch(function () { });
  }

  // ---------------------------------------------------- the calibration panel

  function readCalibration(force) {
    var now = Date.now();
    if (!force && face.calibration && now - face.calibrationAt < 4000) return Promise.resolve();
    face.calibrationAt = now;
    return fetch('/api/calibration', { headers: { accept: 'application/json' } })
      .then(function (reply) {
        if (reply.status === 401) throw new Error('token');
        return reply.json();
      })
      .then(function (doc) {
        face.calibration = doc;
        if (face.panel === 'calibration') paintCalibration();
      })
      .catch(function () { });
  }

  function paintCalibration() {
    var read = face.calibration;
    el.calibrationBoard.textContent = '';
    if (!read) {
      var waiting = boardCard(12, 'calibration', null, null);
      waiting.appendChild(node('p', 'board-empty',
        'The stream read has not landed yet. It reads today\u2019s calibration rows, which ' +
        'this machine\u2019s own runs wrote.'));
      el.calibrationBoard.appendChild(waiting);
      return;
    }
    if (read.refused && read.refused.length) {
      el.calibrationBoard.appendChild(refusedCard(read.refused));
      return;
    }
    var cells = read.cells || [];
    var judged = cells.reduce(function (sum, cell) { return sum + cell.judged; }, 0);
    var fit = cells.filter(function (cell) { return cell.fit === 'fit'; }).length;
    var informing = cells.filter(function (cell) { return cell.informs_failover; }).length;
    var tiles = node('div', 'tiles');
    tiles.appendChild(tile(read.rows_read, 'rows, the day\u2019s last ' + read.cap));
    tiles.appendChild(tile(judged, 'judged rows'));
    tiles.appendChild(tile(cells.length, 'fit cells'));
    tiles.appendChild(tile(fit, 'fit', fit ? 'live' : null));
    tiles.appendChild(tile(informing, 'informing the failover'));
    tiles.appendChild(tile(read.gate && read.gate.informs_failover ? 'on' : 'off',
      'failover gate'));
    var head = boardCard(12, 'calibration', read.day,
      'the stream at ' + read.root + ' \u00b7 the gate is ' +
      (read.gate && read.gate.informs_failover ? 'open' : 'shut') +
      ' (' + read.gate.name + ')');
    head.appendChild(tiles);
    el.calibrationBoard.appendChild(head);

    if (cells.length) {
      var cellCard = boardCard(12, 'the judged quality, per provider and shape', null,
        'a cell is fit when its sample is big enough and wide enough \u2014 ' +
        read.fit_rule.rows + ' graded rows, a ' + read.fit_rule.span + ' confidence span');
      cells.forEach(function (cell) { cellCard.appendChild(calibrationCell(cell)); });
      el.calibrationBoard.appendChild(cellCard);
    } else {
      var none = boardCard(12, 'the judged quality', null, null);
      none.appendChild(node('p', 'board-empty',
        'No judged routes for ' + read.day + ' yet. `fleet judge` grades a day\u2019s route ' +
        'rows through the sys1 lane, and a cell appears when a judgment lands.'));
      el.calibrationBoard.appendChild(none);
    }

    var stream = read.stream || {};
    ['route', 'route_judged', 'turn', 'propose', 'eval_corpus', 'recall_hit', 'consolidate',
      'tool_screen', 'explain'].forEach(function (op) {
      var rows = stream[op];
      if (rows && rows.length) el.calibrationBoard.appendChild(streamCard(op, rows));
    });
  }

  // One fit cell as a card: the counts, the weighted rates, the fit
  // state with what it still needs, and the spend. The numbers are the
  // surface's own computation riding through — the panel re-derives
  // nothing.
  function calibrationCell(cell) {
    var card = node('article', 'cal-cell');
    card.dataset.fit = cell.fit;
    var title = node('h3', 'cal-cell-title',
      cell.provider + (cell.workload ? ' \u00b7 ' + cell.workload : ' \u00b7 no shape judged'));
    title.appendChild(badge(cell.fit, cell.fit === 'fit' ? 'live' : 'warn'));
    card.appendChild(title);
    var counts = node('p', 'cal-cell-counts',
      cell.judged + ' judged \u00b7 ' + cell.right + ' right \u00b7 ' + cell.wrong + ' wrong \u00b7 ' +
      cell.insufficient + ' insufficient' +
      (cell.waived ? ' \u00b7 ' + cell.waived + ' waived' : '') +
      (cell.overridden ? ' \u00b7 ' + cell.overridden + ' overridden' : ''));
    card.appendChild(counts);
    var rates = node('p', 'cal-cell-rates',
      'weighted wrong ' + fmtRate(cell.weighted_wrong_rate) + ' \u00b7 insufficient ' +
      fmtRate(cell.weighted_insufficient_rate) + ' \u00b7 span ' +
      fmtRate(cell.confidence_span) +
      (cell.zero_confidence ? ' \u00b7 ' + cell.zero_confidence + ' rows without confidence' : ''));
    card.appendChild(rates);
    var wants = [];
    if (cell.fit !== 'fit') {
      if (cell.needs_rows) wants.push(cell.needs_rows + ' more graded row' +
        (cell.needs_rows === 1 ? '' : 's'));
      if (cell.needs_span > 0) wants.push('a wider confidence span');
    }
    if (cell.fit === 'fit' && !cell.informs_failover) {
      wants.push('fit, and short of the wrong-rate floor that moves the failover');
    }
    card.appendChild(node('p', 'cal-cell-wants',
      wants.length ? wants.join(' \u00b7 ') : 'this cell informs the failover'));
    var spend = node('p', 'cal-cell-spend',
      'spend ' + cell.spend_usd.toFixed(4) +
      (cell.cost_per_right === null || cell.cost_per_right === undefined
        ? ''
        : ' \u00b7 ' + cell.cost_per_right.toFixed(4) + ' per right'));
    card.appendChild(spend);
    return card;
  }

  function fmtRate(rate) {
    return (rate * 100).toFixed(1) + '%';
  }

  // One op's rows as a card, newest last. The rows ride as the reader
  // parsed them; the panel formats, it does not re-decide.
  function streamCard(op, rows) {
    var names = {
      route: 'routes', route_judged: 'judgments', turn: 'turns',
      propose: 'proposals', eval_corpus: 'eval runs', recall_hit: 'recall hits',
      consolidate: 'consolidations', tool_screen: 'tool screens', explain: 'explains'
    };
    var card = boardCard(12, names[op] || op, rows.length + ' row' + (rows.length === 1 ? '' : 's'),
      null);
    var recent = rows.slice(-20);
    if (rows.length > recent.length) {
      card.appendChild(node('p', 'board-empty',
        'showing the last ' + recent.length + ' of ' + rows.length));
    }
    recent.forEach(function (row) { card.appendChild(streamLine(op, row)); });
    return card;
  }

  function streamLine(op, row) {
    var line = node('div', 'stream-line');
    line.appendChild(node('span', 'stream-ts', (row.ts || '').slice(11, 19) || '\u2014'));
    var text = {
      route: function () {
        return (row.provider || '?') + ' \u2192 ' + (row.model || '?') +
          (row.workload ? ' \u00b7 ' + row.workload : '') +
          (row.how ? ' \u00b7 ' + row.how +
            (row.judged_from ? ' (' + row.judged_from + ')' : '') : '');
      },
      route_judged: function () {
        return (row.workload || '?') + ' \u00b7 ' + (row.verdict || '?') +
          ' \u00b7 confidence ' + fmtRate(row.confidence) +
          (row.disposition ? ' \u00b7 ' + row.disposition : '');
      },
      turn: function () {
        return (row.outcome || '?') +
          (row.stop_reason ? ' \u00b7 ' + row.stop_reason : '') +
          ' \u00b7 ' + row.turns + ' turn' + (row.turns === 1 ? '' : 's') +
          (row.reason ? ' \u00b7 ' + clip(row.reason, 80) : '');
      },
      propose: function () {
        return (row.proposal || '?') + ' \u00b7 ' + (row.target || '?') +
          (row.target_name ? ' \u2192 ' + row.target_name : '') +
          ' \u00b7 ' + (row.verdict || '?') +
          (row.gate && row.gate.length
            ? ' \u00b7 gate ' + row.gate.filter(function (step) { return step.ok; }).length +
              '/' + row.gate.length
            : '');
      },
      eval_corpus: function () {
        return row.passed + '/' + (row.passed + row.failed) + ' scenarios \u00b7 score ' +
          fmtRate(row.score);
      },
      recall_hit: function () {
        return (row.fact || '?') + (row.shape ? ' \u00b7 ' + row.shape : '') +
          ' \u00b7 ' + fmtRate(row.score);
      },
      consolidate: function () {
        return (row.action || '?') + (row.reason ? ' \u00b7 ' + clip(row.reason, 80) : '');
      },
      tool_screen: function () {
        return (row.tool || '?') + ' \u00b7 ' + (row.outcome || '?');
      },
      explain: function () {
        return (row.explained_op || '?') + ' \u00b7 ' + (row.selector || '?') +
          (row.eval_only ? ' \u00b7 eval-only' : '');
      }
    }[op];
    line.appendChild(node('span', 'stream-text', text ? text() : op));
    return line;
  }

  // ------------------------------------------------------ the tower console

  // The console is the one dense surface the plan names: the KPI strip,
  // the hero two-series chart, the harness donut, the ranked projects,
  // the claims-vs-evidence rows, and the feed. Every number traces to
  // the board's own read — the git facts and the second-brain's
  // aggregates — and a read that did not land keeps the last good
  // numbers with its reason beside them.
  function readTower(force) {
    var now = Date.now();
    if (!force && face.tower && now - face.towerAt < 4000) return Promise.resolve();
    face.towerAt = now;
    return fetch('/api/tower', { headers: { accept: 'application/json' } })
      .then(function (reply) {
        if (reply.status === 401) throw new Error('token');
        return reply.json();
      })
      .then(function (doc) {
        // A read that did not land is a banner over the last good
        // numbers, never a blank console: the reasons ride forward on
        // the document that holds them.
        var empty = !(doc.projects || []).length && !doc.kpis && !doc.feed;
        if (empty && face.tower) {
          face.tower = Object.assign({}, face.tower, { refused: doc.refused || [] });
        } else {
          face.tower = doc;
        }
        if (face.panel === 'tower') paintTower();
      })
      .catch(function () { });
  }

  function paintTower() {
    var read = face.tower;
    el.towerBoard.textContent = '';
    if (!read) {
      var waiting = boardCard(12, 'tower', null, null);
      waiting.appendChild(node('p', 'board-empty',
        'The tower read has not landed yet. It reads the projects root\u2019s git facts and ' +
        'the second-brain\u2019s derived state, both of which are this machine\u2019s own.'));
      el.towerBoard.appendChild(waiting);
      return;
    }
    if (read.refused && read.refused.length) {
      el.towerBoard.appendChild(refusedCard(read.refused));
    }
    var projects = read.projects || [];
    var kpis = read.kpis || {};
    var feed = read.feed || [];
    var conflicts = read.conflicts || [];
    var trust = read.trust || [];
    var coverage = read.coverage;
    var daily = read.report_daily || [];
    var timeline = read.timeline || [];
    var harness = read.harness || [];
    var brain = read.brain_projects || [];

    // The KPI strip: five big numbers, each a read's own count.
    var tiles = node('div', 'tiles');
    tiles.appendChild(tile(projects.length, 'projects tracked'));
    tiles.appendChild(tile(kpis.doing || 0, 'open tasks', kpis.doing ? 'warn' : null));
    tiles.appendChild(tile(kpis.open_conflicts || 0, 'open conflicts',
      kpis.open_conflicts ? 'down' : null));
    tiles.appendChild(tile(reportsToday(feed), 'reports today'));
    tiles.appendChild(tile(projects.filter(function (row) { return row.dirty > 0; }).length,
      'dirty worktrees'));
    var head = boardCard(12, 'tower', read.read_at,
      'the projects root at ' + read.root + ' \u00b7 the derived state\u2019s last build ' +
      (read.brain_at || '\u2014'));
    head.appendChild(tiles);
    el.towerBoard.appendChild(head);

    // The hero: reports filed per day against commits shipped per day.
    var hero = boardCard(12, 'activity', null,
      'reports filed per day against the commits the shipped timeline names, over the last ' +
      'fourteen days. The feed is as fresh as the derived state\u2019s last build and says so.');
    hero.appendChild(twoSeries(daily, timeline));
    el.towerBoard.appendChild(hero);

    // The harness mix, beside the coverage line.
    var mix = boardCard(5, 'who files the reports', null, 'each harness\u2019s share of the feed');
    if (harness.length) {
      mix.appendChild(donut(harness.map(function (row) {
        return { model: row.harness, calls: row.total };
      })));
    } else {
      mix.appendChild(node('p', 'board-empty', 'The derived state names no harness mix yet.'));
    }
    el.towerBoard.appendChild(mix);
    var cover = boardCard(7, 'what the reports reach', null,
      coverage
        ? 'of ' + (coverage.with_reports + coverage.commit_only) + ' projects, ' +
          coverage.with_reports + ' filed reports in the last ' + coverage.window_days +
          ' days and ' + coverage.commit_only + ' have commits only'
        : 'the derived state names no coverage yet');
    cover.appendChild(node('p', 'board-empty',
      coverage ? '' : 'the coverage line names itself when the next build lands'));
    el.towerBoard.appendChild(cover);

    // The ranked projects: trailing-30-day commits, bar, share, facts.
    var ranked = projects.slice().sort(function (left, right) {
      return right.commits_30d - left.commits_30d;
    });
    var top = ranked.reduce(function (max, row) { return Math.max(max, row.commits_30d); }, 0);
    var rankCard = boardCard(12, 'the ranked projects', null,
      'trailing thirty days of commits, the share of the whole, and the git facts as read. ' +
      'The board\u2019s own read is dirtiest-first; this list ranks by the thirty-day count.');
    ranked.forEach(function (row) {
      var health = brain.filter(function (entry) { return entry.name === row.name; })[0];
      rankCard.appendChild(projectRow(row, health, top));
    });
    el.towerBoard.appendChild(rankCard);

    // The claims against the evidence, and the precision each harness
    // earned.
    var claims = boardCard(12, 'claims against the evidence', kpis.open_conflicts + ' open',
      'a claim the evidence rejected, with the reality that rejected it. The trust lines are ' +
      'each harness\u2019s own precision.');
    if (conflicts.length) {
      conflicts.forEach(function (row) {
        var line = node('div', 'stream-line');
        line.appendChild(node('span', 'stream-ts', row.task));
        line.appendChild(node('span', 'stream-text',
          row.claim + '  \u2014  ' + row.reality));
        claims.appendChild(line);
      });
    } else {
      claims.appendChild(node('p', 'board-empty', 'No claim conflicts with its evidence right now.'));
    }
    if (trust.length) {
      trust.forEach(function (row) {
        var line = node('div', 'stream-line');
        line.appendChild(node('span', 'stream-ts', row.harness));
        line.appendChild(node('span', 'stream-text',
          row.evidenced + ' of ' + row.claimed + ' claims carried evidence \u00b7 precision ' +
          fmtRate(row.precision)));
        claims.appendChild(line);
      });
    }
    el.towerBoard.appendChild(claims);

    // The feed, newest first, with its own staleness named.
    var feedCard = boardCard(12, 'the report feed', (read.reports_at || '\u2014') + ' built',
      'every filed report, newest first. The feed is as fresh as the derived state\u2019s last ' +
      'build and says so rather than pretending to be live.');
    if (feed.length) {
      feed.forEach(function (row) {
        var line = node('div', 'stream-line');
        line.appendChild(node('span', 'stream-ts', (row.ts || '').slice(5, 16)));
        line.appendChild(node('span', 'stream-text',
          row.kind + ' \u00b7 ' + row.project + ' \u00b7 ' + row.summary +
          (row.commit ? ' \u00b7 ' + row.commit.slice(0, 12) : ' \u00b7 no commit filed')));
        feedCard.appendChild(line);
      });
    } else {
      feedCard.appendChild(node('p', 'board-empty',
        'No reports are in the feed. The ledger at ~/.second-brain is where they are filed.'));
    }
    el.towerBoard.appendChild(feedCard);

    // The status bar: the scan's honesty about what it left unread.
    var status = boardCard(12, 'the scan', null,
      (read.beyond_bound || read.unread
        ? read.beyond_bound + ' directories were beyond the scan\u2019s bound and ' +
          read.unread + ' were left unread by its budget'
        : 'the scan read every directory it found'));
    if ((read.skipped || []).length) {
      read.skipped.forEach(function (row) {
        var line = node('div', 'stream-line');
        line.appendChild(node('span', 'stream-ts', row.name));
        line.appendChild(node('span', 'stream-text', row.reason));
        status.appendChild(line);
      });
    }
    el.towerBoard.appendChild(status);
  }

  // Reports the feed carries for today, UTC — the KPI the tower's own
  // ledger counts, derived from the feed rather than reported twice.
  function reportsToday(feed) {
    var today = new Date().toISOString().slice(0, 10);
    return feed.filter(function (row) { return (row.ts || '').slice(0, 10) === today; }).length;
  }

  // The hero: two bars per day, reports filed against commits shipped.
  // Hand-rolled SVG in the board's own chart language; the days are the
  // union of both series' dates, so a day one series misses reads zero.
  function twoSeries(daily, timeline) {
    var reports = {};
    var commits = {};
    daily.forEach(function (row) { reports[row.day] = (reports[row.day] || 0) + row.count; });
    timeline.forEach(function (row) {
      var day = (row.date || '').slice(0, 10);
      if (day) commits[day] = (commits[day] || 0) + 1;
    });
    var days = Object.keys(reports).concat(Object.keys(commits)).sort();
    var last = days.slice(-14);
    var max = 0;
    last.forEach(function (day) {
      max = Math.max(max, reports[day] || 0, commits[day] || 0);
    });
    if (!last.length || max === 0) {
      return node('p', 'board-empty',
        'No activity on either series yet. The chart appears when the first report or ' +
        'commit lands in a day the derived state names.');
    }
    var plot = svgPlot(Math.max(240, last.length * 30), 120, 'reports filed against commits shipped');
    var base = 104;
    var scale = max ? 88 / max : 0;
    last.forEach(function (day, index) {
      var x = 14 + index * 30;
      var reports_h = Math.round((reports[day] || 0) * scale);
      var commits_h = Math.round((commits[day] || 0) * scale);
      if (reports_h > 0) {
        plot.appendChild(svg('rect', {
          x: x, y: base - reports_h, width: 9, height: reports_h,
          fill: 'var(--dsw-alias-accent)', rx: 2
        }));
      }
      if (commits_h > 0) {
        plot.appendChild(svg('rect', {
          x: x + 10, y: base - commits_h, width: 9, height: commits_h,
          fill: 'var(--dsw-alias-ink-tertiary)', rx: 2
        }));
      }
      var label = svg('text', { x: x + 9, y: base + 11, 'text-anchor': 'middle',
        fill: 'var(--dsw-alias-ink-tertiary)', 'font-size': 8 });
      label.textContent = day.slice(5);
      plot.appendChild(label);
    });
    plot.appendChild(svg('line', {
      x1: 14, y1: base, x2: 14 + last.length * 30, y2: base,
      stroke: 'var(--dsw-alias-ink-tertiary)', 'stroke-width': 0.5
    }));
    var wrap = node('div', 'hero-wrap');
    wrap.appendChild(plot);
    var legend = node('ul', 'legend');
    [['reports filed', 'var(--dsw-alias-accent)'], ['commits shipped', 'var(--dsw-alias-ink-tertiary)']]
      .forEach(function (entry) {
        var item = node('li');
        var swatch = node('span', 'swatch');
        swatch.style.background = entry[1];
        item.appendChild(swatch);
        item.appendChild(node('span', 'legend-name', entry[0]));
        legend.appendChild(item);
      });
    wrap.appendChild(legend);
    return wrap;
  }

  // One ranked row: the bar and the share, then the facts as read. A
  // `null` fact stays a dash — git could not answer, and a zero nobody
  // measured would be a lie the bar graph tells.
  function projectRow(row, health, top) {
    var line = node('div', 'rank-row');
    var dot = node('span', 'health-dot');
    var lifecycle = health && health.lifecycle ? health.lifecycle : '';
    dot.dataset.health = lifecycle === 'active' ? 'ok'
      : lifecycle === 'dormant' ? 'warn' : lifecycle === 'archived' ? 'down' : 'unknown';
    line.appendChild(dot);
    line.appendChild(node('span', 'rank-name', row.name));
    var share = top ? Math.round((row.commits_30d / top) * 100) : 0;
    var track = node('span', 'rank-track');
    var fill = node('span', 'rank-fill');
    fill.style.width = share + '%';
    track.appendChild(fill);
    line.appendChild(track);
    line.appendChild(node('span', 'rank-count', String(row.commits_30d)));
    var facts = node('span', 'rank-facts');
    facts.appendChild(node('span', 'rank-fact', (row.branch || '\u2014') +
      ' @ ' + (row.head || '\u2014')));
    if (row.dirty) facts.appendChild(badge(row.dirty + ' dirty', 'warn'));
    if (row.ahead !== null && row.ahead !== undefined) {
      facts.appendChild(badge('\u2191' + row.ahead + ' \u2193' + row.behind, null));
    }
    line.appendChild(facts);
    if (health && health.plans && health.plans.length) {
      var plans = node('span', 'rank-plans');
      health.plans.forEach(function (plan) {
        plans.appendChild(badge(plan.status + ' ' + plan.count, plan.status === 'done' ? 'ok' : null));
      });
      line.appendChild(plans);
    }
    return line;
  }

  // -------------------------------------------------- the second-brain panel

  function readBrain(force) {
    var now = Date.now();
    if (!force && face.brain && now - face.brainAt < 4000) return Promise.resolve();
    face.brainAt = now;
    return fetch('/api/second-brain', { headers: { accept: 'application/json' } })
      .then(function (reply) {
        if (reply.status === 401) throw new Error('token');
        return reply.json();
      })
      .then(function (doc) {
        // The same rule the console keeps: a read that did not land
        // leaves the last good sections standing with the reasons
        // beside them.
        var empty = !doc.gates && !doc.harness && !doc.loops && !doc.maturity;
        if (empty && face.brain) {
          face.brain = Object.assign({}, face.brain, { refused: doc.refused || [] });
        } else {
          face.brain = doc;
        }
        if (face.panel === 'second-brain') paintBrain();
      })
      .catch(function () { });
  }

  function paintBrain() {
    var read = face.brain;
    el.brainBoard.textContent = '';
    if (!read) {
      var waiting = boardCard(12, 'second brain', null, null);
      waiting.appendChild(node('p', 'board-empty',
        'The derived-state read has not landed yet. It reads the six state files under the ' +
        'second-brain home, which is this machine\u2019s own.'));
      el.brainBoard.appendChild(waiting);
      return;
    }
    if (read.refused && read.refused.length) {
      el.brainBoard.appendChild(refusedCard(read.refused));
    }
    var head = boardCard(12, 'second brain', read.read_at, 'the derived state at ' + read.home);
    var tiles = node('div', 'tiles');
    var reports = read.reports || {};
    tiles.appendChild(tile(reports.doing_total || 0, 'open tasks'));
    tiles.appendChild(tile(reports.pending_verification || 0, 'awaiting verification',
      reports.pending_verification ? 'warn' : null));
    tiles.appendChild(tile(reports.open_conflicts || 0, 'conflicts',
      reports.open_conflicts ? 'down' : null));
    tiles.appendChild(tile(reports.suppressed || 0, 'suppressed rows'));
    head.appendChild(tiles);
    el.brainBoard.appendChild(head);

    // The earned-autonomy gates.
    var gates = read.gates;
    if (gates) {
      var board = boardCard(12, 'the gates', gates.generated,
        'verdicts and dispositions per gate, with the graded progress and the override rate. ' +
        'A gate that is fit has earned its autonomy; the override rate is the human seam.');
      (gates.board || []).forEach(function (row) {
        var line = node('div', 'gate-row');
        line.appendChild(node('span', 'gate-name', row.gate));
        line.appendChild(badge(row.graded_progress, null));
        line.appendChild(node('span', 'gate-counts',
          row.verdicts + ' verdicts \u00b7 ' + row.dispositions + ' dispositions'));
        line.appendChild(badge(row.fit_ready ? 'fit' : 'not yet fit', row.fit_ready ? 'ok' : 'warn'));
        line.appendChild(node('span', 'gate-rate', 'override ' + fmtRate(row.override_rate)));
        board.appendChild(line);
      });
      if ((gates.inbox || []).length) {
        (gates.inbox || []).forEach(function (row) {
          var line = node('div', 'stream-line');
          line.appendChild(node('span', 'stream-ts', row.ts.slice(5, 16)));
          line.appendChild(node('span', 'stream-text',
            row.op + ' \u00b7 ' + row.verdict + ' \u00b7 ' + row.command));
          board.appendChild(line);
        });
      }
      el.brainBoard.appendChild(board);
    }

    // The quality loops.
    var loops = read.loops;
    if (loops) {
      var loopCard = boardCard(12, 'the quality loops', loops.derived_at, null);
      (loops.loops || []).forEach(function (row) {
        var line = node('div', 'stream-line');
        line.appendChild(node('span', 'stream-ts', row.name));
        line.appendChild(node('span', 'stream-text',
          row.verdict + ' \u00b7 ' + row.iterations + ' iterations' +
          (row.pending_review ? ' \u00b7 ' + row.pending_review + ' pending review' : '')));
        loopCard.appendChild(line);
      });
      el.brainBoard.appendChild(loopCard);
    }

    // The harness runs.
    var harness = read.harness;
    if (harness) {
      var harnessCard = boardCard(12, 'the harness runs', harness.generated, null);
      (harness.loops || []).forEach(function (row) {
        var line = node('div', 'stream-line');
        line.appendChild(node('span', 'stream-ts', row.name));
        line.appendChild(node('span', 'stream-text',
          row.runs + ' runs \u00b7 ' + row.converged + ' converged \u00b7 last ' + row.last_status));
        harnessCard.appendChild(line);
      });
      (harness.recent || []).slice(0, 8).forEach(function (row) {
        var line = node('div', 'stream-line');
        line.appendChild(node('span', 'stream-ts', row.ts.slice(5, 16)));
        line.appendChild(node('span', 'stream-text', row.kind + ' \u00b7 ' + row.label +
          ' \u00b7 ' + row.outcome));
        harnessCard.appendChild(line);
      });
      el.brainBoard.appendChild(harnessCard);
    }

    // The maturity week grid.
    var maturity = read.maturity;
    if (maturity) {
      var weekCard = boardCard(12, 'the maturity signals', maturity.generated,
        'one row per signal, one cell per week. A darker cell is a week with more of it.');
      weekCard.appendChild(weekGrid(maturity.series || [], maturity.weeks || []));
      (maturity.milestones || []).forEach(function (row) {
        var line = node('div', 'stream-line');
        line.appendChild(node('span', 'stream-ts', row.when));
        line.appendChild(node('span', 'stream-text', row.what));
        weekCard.appendChild(line);
      });
      el.brainBoard.appendChild(weekCard);
    }
  }

  // The week grid: one row per series, one cell per week, the cell's
  // intensity the week's value against the row's own maximum.
  function weekGrid(series, weeks) {
    var wrap = node('div', 'week-grid');
    series.forEach(function (row) {
      var line = node('div', 'week-row');
      line.appendChild(node('span', 'week-name', row.name));
      var cells = node('span', 'week-cells');
      var max = row.points.reduce(function (m, v) { return Math.max(m, v); }, 0);
      row.points.forEach(function (value, index) {
        var cell = node('span', 'week-cell');
        cell.dataset.level = max && value ? String(Math.ceil((value / max) * 4)) : '0';
        cell.title = (weeks[index] || 'week ' + (index + 1)) + ': ' + value;
        cells.appendChild(cell);
      });
      line.appendChild(cells);
      wrap.appendChild(line);
    });
    return wrap;
  }

  // ------------------------------------------------------- the settings surface

  // The whole config as one document and every write through the writer
  // the surface owns. The panel never holds a shadow copy of the
  // home's state: a write posts, the reply says what landed, and the
  // read that follows is the home's own answer.
  function readSettings(force) {
    var now = Date.now();
    if (!force && face.settings && now - face.settingsAt < 4000) return Promise.resolve();
    face.settingsAt = now;
    return fetch('/api/settings', { headers: { accept: 'application/json' } })
      .then(function (reply) {
        if (reply.status === 401) throw new Error('token');
        return reply.json();
      })
      .then(function (doc) {
        face.settings = doc;
        if (face.panel === 'settings') paintSettings();
      })
      .catch(function () { });
  }

  // One write, then the read again: the panel paints what the writer
  // confirmed, and a refusal names itself the way the writer did.
  function writeSettings(op, fields, onDone) {
    var body = Object.assign({ op: op }, fields || {});
    post('/api/settings', body).then(function (reply) {
      if (reply.status !== 200) {
        notice('the write was refused',
          (reply.body && reply.body.error) || 'the surface said no without a reason');
        return;
      }
      return readSettings(true).then(onDone);
    }).catch(function () { });
  }

  function paintSettings() {
    var read = face.settings;
    el.settingsBoard.textContent = '';
    if (!read) {
      var waiting = boardCard(12, 'settings', null, null);
      waiting.appendChild(node('p', 'board-empty',
        'The settings read has not landed yet. It reads this home\u2019s own config, routing, ' +
        'and env files.'));
      el.settingsBoard.appendChild(waiting);
      return;
    }
    if (read.refused && read.refused.length) {
      el.settingsBoard.appendChild(refusedCard(read.refused));
    }
    var head = boardCard(12, 'settings', read.read_at,
      'every knob this home carries, the two lists and the rules, the cron jobs, and the ' +
      'identity slot. Every write rides the writer the CLI rides, so the two faces cannot ' +
      'disagree about what the file holds.');
    var tiles = node('div', 'tiles');
    tiles.appendChild(tile(read.scalars.length, 'knobs'));
    tiles.appendChild(tile(read.lists.filter(function (e) { return e.kind === 'provider'; }).length,
      'provider overrides'));
    tiles.appendChild(tile(read.lists.filter(function (e) { return e.kind === 'workload'; }).length,
      'workload preferences'));
    tiles.appendChild(tile(read.rules.length, 'remembered rules',
      read.rules.length ? 'live' : null));
    tiles.appendChild(tile(read.cron.length, 'cron jobs',
      read.cron.filter(function (j) { return j.enabled; }).length + ' enabled'));
    head.appendChild(tiles);
    el.settingsBoard.appendChild(head);

    // The knobs, each editable in place: the value box saves through
    // the one writer, and the source badge says which file answers.
    var knobs = boardCard(12, 'the knobs', null,
      'the value in the box is what the home holds. A knob sourced from the environment says ' +
      'so \u2014 a file write would not take effect until the env var is unset.');
    (read.scalars || []).forEach(function (knob) {
      knobs.appendChild(knobRow(knob));
    });
    el.settingsBoard.appendChild(knobs);

    // The two list surfaces and the rules, as the third.
    var providers = (read.lists || []).filter(function (e) { return e.kind === 'provider'; });
    var providerCard = boardCard(12, 'the providers', null,
      'config.json\u2019s provider overrides. Removing one that reverts to the compiled-in ' +
      'roster is the ordinary edit; removing one the file added takes the provider out of ' +
      'every walk that names it, and the card says which is which.');
    if (providers.length) {
      providers.forEach(function (entry) { providerCard.appendChild(listEditor(entry, 'provider')); });
    } else {
      providerCard.appendChild(node('p', 'board-empty',
        'No provider overrides. The compiled-in roster carries every candidate.'));
    }
    providerCard.appendChild(addProviderForm());
    el.settingsBoard.appendChild(providerCard);

    var workloads = (read.lists || []).filter(function (e) { return e.kind === 'workload'; });
    var workloadCard = boardCard(12, 'the workloads', null,
      'routing.json\u2019s preference lists. The candidates are the walk order, and an empty ' +
      'list is refused \u2014 a workload nothing can serve walks into its default.');
    if (workloads.length) {
      workloads.forEach(function (entry) { workloadCard.appendChild(listEditor(entry, 'workload')); });
    } else {
      workloadCard.appendChild(node('p', 'board-empty',
        'No workload preferences. The compiled-in table carries every chain.'));
    }
    workloadCard.appendChild(addWorkloadForm());
    el.settingsBoard.appendChild(workloadCard);

    var ruleCard = boardCard(12, 'the remembered rules', null,
      'the third list, and the one that makes prompts thin out over time. The always-ask and ' +
      'destructive classes are refused by name \u2014 approval for those happens in the ' +
      'current turn, and a rule cannot waive it.');
    if (read.rules.length) {
      read.rules.forEach(function (rule) {
        var row = node('div', 'list-row');
        row.appendChild(node('span', 'list-row-key', rule.label));
        var forget = node('button', 'replay-button');
        forget.type = 'button';
        forget.textContent = 'forget';
        forget.addEventListener('click', function () {
          writeSettings('rule-remove', { tool: rule.tool, program: rule.program || null });
        });
        row.appendChild(forget);
        ruleCard.appendChild(row);
      });
    } else {
      ruleCard.appendChild(node('p', 'board-empty', 'No rule is remembered yet.'));
    }
    ruleCard.appendChild(addRuleForm());
    el.settingsBoard.appendChild(ruleCard);

    // The cron jobs: the surface they have never had.
    var cronCard = boardCard(12, 'the cron jobs', null,
      'the scheduled runs. A fire is an ordinary headless run in its own session log, and the ' +
      'task is the instruction it is given.');
    if (read.cron.length) {
      read.cron.forEach(function (job) { cronCard.appendChild(cronRow(job)); });
    } else {
      cronCard.appendChild(node('p', 'board-empty',
        'No jobs are scheduled. The five-field form is minute hour day-of-month month day-of-week.'));
    }
    cronCard.appendChild(addCronForm());
    el.settingsBoard.appendChild(cronCard);

    // The identity slot and the env names.
    var soulCard = boardCard(12, 'the identity slot', read.soul.present ? 'SOUL.md is set' : 'absent',
      'the file the model-visible base is read from per run. Absent, the compiled-in voice ' +
      'stands. Writing one replaces the base; the skills and memory frames still append.');
    var soul = node('textarea', 'soul-box');
    soul.value = read.soul.text || '';
    soul.setAttribute('aria-label', 'the SOUL.md identity slot');
    soulCard.appendChild(soul);
    var saveSoul = node('button', 'replay-button replay-resume');
    saveSoul.type = 'button';
    saveSoul.textContent = 'write SOUL.md';
    saveSoul.addEventListener('click', function () {
      writeSettings('soul', { text: soul.value });
    });
    soulCard.appendChild(saveSoul);
    el.settingsBoard.appendChild(soulCard);

    var envCard = boardCard(12, 'the env-var counterparts', null,
      'by name only: whether each resolves right now, and from where. A value never rides ' +
      'this document \u2014 the doctor\u2019s masking rule.');
    (read.env || []).forEach(function (entry) {
      var row = node('div', 'list-row');
      row.appendChild(node('span', 'list-row-key', entry.name));
      row.appendChild(badge(entry.present ? 'resolves' : 'unset', entry.present ? 'ok' : 'warn'));
      envCard.appendChild(row);
    });
    el.settingsBoard.appendChild(envCard);
  }

  // One knob: its key, its source, and the box that saves. The source
  // badge is the point — a knob sourced from the environment would not
  // take a file write until the var is unset, and saying so is the
  // difference between an edit that works and one that silently does not.
  function knobRow(knob) {
    var row = node('div', 'knob-row');
    var head = node('div', 'knob-head');
    head.appendChild(node('span', 'knob-key', knob.key));
    head.appendChild(badge(knob.file, null));
    head.appendChild(badge(knob.source, knob.source === 'env' ? 'warn' : null));
    row.appendChild(head);
    row.appendChild(node('p', 'knob-summary', knob.summary));
    var editor = node('div', 'knob-editor');
    var box = node('input', 'knob-box');
    box.type = 'text';
    box.value = knob.value === 'unset' ? '' : knob.value;
    box.setAttribute('aria-label', knob.key);
    var save = node('button', 'replay-button');
    save.type = 'button';
    save.textContent = 'save';
    save.addEventListener('click', function () {
      writeSettings('set', { key: knob.key, value: box.value });
    });
    editor.appendChild(box);
    editor.appendChild(save);
    row.appendChild(editor);
    return row;
  }

  // One list entry with its editor: the seed the writer parses, a save
  // that rides it, and a remove that names the class of what it removes.
  function listEditor(entry, kind) {
    var row = node('div', 'list-editor');
    var head = node('div', 'list-editor-head');
    head.appendChild(node('span', 'list-row-key', entry.key));
    head.appendChild(badge(entry.reverts ? 'reverts to the compiled-in default' : 'added by the file',
      entry.reverts ? null : 'warn'));
    row.appendChild(head);
    row.appendChild(node('p', 'knob-summary', entry.detail));
    var box = node('textarea', 'list-seed');
    box.value = entry.seed || '';
    box.setAttribute('aria-label', entry.key);
    row.appendChild(box);
    var actions = node('div', 'replay-actions');
    var save = node('button', 'replay-button');
    save.type = 'button';
    save.textContent = 'save';
    save.addEventListener('click', function () {
      if (kind === 'provider') {
        writeSettings('provider', { entry: box.value });
      } else {
        var candidates = box.value.split(',').map(function (one) { return one.trim(); })
          .filter(function (one) { return one.length > 0; });
        writeSettings('workload', { workload: entry.key, candidates: candidates });
      }
    });
    actions.appendChild(save);
    var remove = node('button', 'replay-button');
    remove.type = 'button';
    remove.textContent = entry.reverts ? 'remove the override' : 'remove \u2014 takes it out of every walk';
    remove.addEventListener('click', function () {
      if (!entry.reverts && !window.confirm(
        '`' + entry.key + '` was added by the file, not the compiled-in roster. Removing it ' +
        'takes the ' + (kind === 'provider' ? 'provider' : 'workload') + ' out of every walk ' +
        'that names it. Do it anyway?')) {
        return;
      }
      writeSettings(kind === 'provider' ? 'provider-remove' : 'workload-remove',
        kind === 'provider' ? { id: entry.key, force: !entry.reverts }
                            : { workload: entry.key, force: !entry.reverts });
    });
    actions.appendChild(remove);
    row.appendChild(actions);
    return row;
  }

  function addProviderForm() {
    var form = node('div', 'list-editor');
    var head = node('div', 'list-editor-head');
    head.appendChild(node('span', 'list-row-key', 'add a provider override'));
    form.appendChild(head);
    var box = node('textarea', 'list-seed');
    box.value = '{"id":"my-provider","base_url":"https://example.invalid/v1","key_env":"MY_KEY"}';
    box.setAttribute('aria-label', 'a new provider entry');
    form.appendChild(box);
    var add = node('button', 'replay-button replay-resume');
    add.type = 'button';
    add.textContent = 'add';
    add.addEventListener('click', function () {
      writeSettings('provider', { entry: box.value });
    });
    form.appendChild(add);
    return form;
  }

  function addWorkloadForm() {
    var form = node('div', 'list-editor');
    var head = node('div', 'list-editor-head');
    head.appendChild(node('span', 'list-row-key', 'add a workload preference'));
    form.appendChild(head);
    var name = node('input', 'knob-box');
    name.type = 'text';
    name.value = 'my-workload';
    name.setAttribute('aria-label', 'the workload name');
    var box = node('input', 'knob-box');
    box.type = 'text';
    box.value = 'provider/model, other-provider/model';
    box.setAttribute('aria-label', 'the workload candidates');
    form.appendChild(name);
    form.appendChild(box);
    var add = node('button', 'replay-button replay-resume');
    add.type = 'button';
    add.textContent = 'add';
    add.addEventListener('click', function () {
      writeSettings('workload', {
        workload: name.value,
        candidates: box.value.split(',').map(function (one) { return one.trim(); })
          .filter(function (one) { return one.length > 0; })
      });
    });
    form.appendChild(add);
    return form;
  }

  function addRuleForm() {
    var form = node('div', 'list-editor');
    var head = node('div', 'list-editor-head');
    head.appendChild(node('span', 'list-row-key', 'remember a rule'));
    form.appendChild(head);
    var tool = node('input', 'knob-box');
    tool.type = 'text';
    tool.value = 'bash';
    tool.setAttribute('aria-label', 'the tool the rule covers');
    var program = node('input', 'knob-box');
    program.type = 'text';
    program.value = 'cargo';
    program.setAttribute('aria-label', 'the leading program, for the shell only');
    form.appendChild(tool);
    form.appendChild(program);
    var add = node('button', 'replay-button replay-resume');
    add.type = 'button';
    add.textContent = 'remember';
    add.addEventListener('click', function () {
      writeSettings('rule', {
        tool: tool.value,
        program: program.value.trim() || null
      });
    });
    form.appendChild(add);
    return form;
  }

  function cronRow(job) {
    var row = node('div', 'list-editor');
    var head = node('div', 'list-editor-head');
    head.appendChild(node('span', 'list-row-key', job.name));
    head.appendChild(badge(job.enabled ? 'enabled' : 'disabled', job.enabled ? 'ok' : 'warn'));
    if (job.auto_approve) head.appendChild(badge('auto-approve', 'warn'));
    row.appendChild(head);
    row.appendChild(node('p', 'knob-summary',
      job.schedule + ' \u00b7 ' + job.task +
      (job.last_run ? ' \u00b7 last ran ' + job.last_run +
        (job.last_outcome ? ' (' + job.last_outcome + ')' : '') : ' \u00b7 never ran')));
    var actions = node('div', 'replay-actions');
    var toggle = node('button', 'replay-button');
    toggle.type = 'button';
    toggle.textContent = job.enabled ? 'disable' : 'enable';
    toggle.addEventListener('click', function () {
      writeSettings('cron-enabled', { name: job.name, enabled: !job.enabled });
    });
    actions.appendChild(toggle);
    var remove = node('button', 'replay-button');
    remove.type = 'button';
    remove.textContent = 'remove';
    remove.addEventListener('click', function () {
      if (window.confirm('remove the job `' + job.name + '`?')) {
        writeSettings('cron-remove', { name: job.name });
      }
    });
    actions.appendChild(remove);
    row.appendChild(actions);
    return row;
  }

  function addCronForm() {
    var form = node('div', 'list-editor');
    var head = node('div', 'list-editor-head');
    head.appendChild(node('span', 'list-row-key', 'schedule a job'));
    form.appendChild(head);
    var fields = [
      ['name', 'a job name', 'daily-triage'],
      ['schedule', 'minute hour day-of-month month day-of-week', '0 9 * * *'],
      ['task', 'the instruction it is given', 'triage the KI ledger']
    ];
    var boxes = {};
    fields.forEach(function (field) {
      var box = node('input', 'knob-box');
      box.type = 'text';
      box.value = field[2];
      box.setAttribute('aria-label', field[1]);
      boxes[field[0]] = box;
      form.appendChild(box);
    });
    var add = node('button', 'replay-button replay-resume');
    add.type = 'button';
    add.textContent = 'schedule';
    add.addEventListener('click', function () {
      writeSettings('cron', {
        name: boxes.name.value,
        schedule: boxes.schedule.value,
        task: boxes.task.value,
        auto_approve: false
      });
    });
    form.appendChild(add);
    return form;
  }

  // -------------------------------------------------------- the trust trio

  // Plan mode's toggle: the composer's second door. The state is the
  // browser's own — the server has no session-scoped mode, because a
  // mode that outlived the message that asked for it would be a mode
  // nobody chose for the next task.
  el.plan.addEventListener('click', function () {
    face.planMode = !face.planMode;
    el.plan.setAttribute('aria-pressed', face.planMode ? 'true' : 'false');
    el.hint.textContent = face.planMode
      ? 'Plan mode: the run plans and writes nothing \u00b7 Enter plans'
      : 'Enter sends \u00b7 Shift+Enter is a newline';
  });

  // The plan door: the same task, the plan arm. A plan-mode run's final
  // message records as a plan, and the card it lands in is editable —
  // Build posts the edited text as an ordinary run.
  function planSend() {
    var task = el.task.value.trim();
    if (!task || face.running || face.seat) return;
    el.task.value = '';
    grow();
    face.planned = true;
    post('/api/plan', { task: task }).then(function (reply) {
      if (reply.status === 200) return;
      face.planned = false;
      notice('the plan did not start',
        (reply.body && reply.body.error) || 'the face said no without a reason');
    }).catch(function () { face.planned = false; });
  }

  // Enter in plan mode plans; the Send button still runs.
  el.task.addEventListener('keydown', function (event) {
    if (event.key === 'Enter' && !event.shiftKey && face.planMode) {
      event.preventDefault();
      planSend();
    }
  });

  // A turn's file state, checkpointed: the card names the turn and
  // offers the rewind. The store's own counts ride along, because a
  // checkpoint of nothing is a different fact from a checkpoint of the
  // whole tree.
  function checkpointRow(event) {
    var el0 = card('checkpoint', '\u25c7', 'checkpoint \u00b7 turn ' + event.turn,
      event.files + ' files \u00b7 ' + event.bytes + ' bytes held for a rewind',
      null, null);
    var rewind = node('button', 'replay-button');
    rewind.type = 'button';
    rewind.textContent = 'rewind to here';
    rewind.addEventListener('click', function () { rewindTo(event.turn); });
    el0.querySelector('.note').textContent = '';
    el0.querySelector('.note').hidden = false;
    el0.querySelector('.note').appendChild(rewind);
    return row(el0, event);
  }

  // The rewind: the tree restored, the log cut, the face reseeded. The
  // reply names what it did, and the conversation that follows is the
  // one the next task seeds from.
  function rewindTo(turn) {
    if (!window.confirm('rewind the working tree to turn ' + turn +
      '\u2019s checkpoint? The files the run changed after it go back, and the ' +
      'conversation continues from that turn.')) {
      return;
    }
    post('/api/rewind', { turn: turn }).then(function (reply) {
      if (reply.status !== 200) {
        notice('the rewind was refused',
          (reply.body && reply.body.error) || 'the face said no without a reason');
        return;
      }
      notice('rewound to turn ' + turn,
        reply.body.restored + ' entries restored \u00b7 ' + reply.body.removed +
        ' files the checkpoint never saw removed');
      window.location.hash = '#/chat';
      load(true);
    }).catch(function () { });
  }

  // The rewind record, replayed: what the log says happened.
  function rewoundRow(event) {
    var el0 = card('rewound', '\u21a9', 'rewound to turn ' + event.turn,
      event.restored + ' entries restored \u00b7 ' + event.removed + ' removed',
      null, null);
    return row(el0, event);
  }

  // A plan-mode run's answer: the plan itself, editable, with Build.
  // The card is the log's own record, so a refresh replays the same
  // plan and the same buttons.
  function planRow(event) {
    var el0 = card('plan', '\u25b8', 'the plan', 'plan mode wrote nothing \u00b7 edit it, then build',
      null, null);
    var box = node('textarea', 'soul-box');
    box.value = event.text || '';
    box.setAttribute('aria-label', 'the plan');
    var build = node('button', 'replay-button replay-resume');
    build.type = 'button';
    build.textContent = 'build this plan';
    build.addEventListener('click', function () {
      var edited = box.value.trim();
      if (!edited || face.running || face.seat) return;
      face.planned = false;
      post('/api/run', { task: edited }).then(function (reply) {
        if (reply.status === 200) return;
        notice('the build did not start',
          (reply.body && reply.body.error) || 'the face said no without a reason');
      }).catch(function () { });
    });
    var note = el0.querySelector('.note');
    note.hidden = false;
    note.appendChild(box);
    note.appendChild(build);
    return row(el0, event);
  }

  // The live half of a plan-mode run: the finished event's assistant
  // message is the plan, and the card it lands in is the editable one.
  function finishPlan() {
    var said = '';
    for (var i = face.events.length - 1; i >= 0; i -= 1) {
      var event = face.events[i];
      if (event.type === 'message' && event.role === 'assistant' && event.text) {
        said = event.text;
        break;
      }
    }
    planRow({ type: 'plan', text: said });
  }

  // The diff review: the session's changes against a checkpoint. The
  // turn is the newest the store holds, and each hunk's reject is the
  // reverse of exactly that hunk.
  function openReview() {
    face.panel === 'chat' ? null : (window.location.hash = '#/chat');
    fetch('/api/checkpoints', { headers: { accept: 'application/json' } })
      .then(function (reply) { return reply.json(); })
      .then(function (doc) {
        var snapshots = doc.snapshots || [];
        if (!snapshots.length) {
          notice('nothing to review',
            'this session holds no checkpoint yet \u2014 a run checkpoints its file state ' +
            'per turn, and the review measures the tree against one');
          return;
        }
        var turn = snapshots[0].turn;
        return fetch('/api/diff?turn=' + turn, { headers: { accept: 'application/json' } })
          .then(function (reply) { return reply.json(); })
          .then(function (diff) { paintDiff(diff, turn); });
      })
      .catch(function () { });
  }

  function paintDiff(diff, turn) {
    var refused = diff.refused;
    if (refused && refused.length) {
      notice('the review did not land', refused.join(' \u00b7 '));
      return;
    }
    var files = diff.files || [];
    if (!files.length) {
      notice('no changes against turn ' + turn,
        'the working tree matches the checkpoint exactly');
      return;
    }
    var el0 = card('diff', '\u00b1', 'the changes against turn ' + turn,
      files.length + ' file' + (files.length === 1 ? '' : 's') +
      ' \u00b7 a reject puts that hunk back', null, null);
    var note = el0.querySelector('.note');
    note.hidden = false;
    files.forEach(function (file) {
      note.appendChild(diffFile(file, turn));
    });
    face.stream.appendChild(el0);
    settle();
  }

  function diffFile(file, turn) {
    var wrap = node('div', 'diff-file');
    var head = node('div', 'diff-file-head');
    head.appendChild(node('span', 'diff-path', file.path));
    head.appendChild(badge(file.kind, file.kind === 'added' ? 'ok'
      : file.kind === 'removed' ? 'down' : null));
    if (file.kind !== 'modified' || file.too_large) {
      var whole = node('button', 'replay-button');
      whole.type = 'button';
      whole.textContent = file.too_large ? 'too large to diff \u00b7 reject the whole file'
        : file.kind === 'added' ? 'remove the file' : 'restore the file';
      whole.addEventListener('click', function () {
        rejectHunk(turn, file.path, null);
      });
      head.appendChild(whole);
    }
    wrap.appendChild(head);
    (file.hunks || []).forEach(function (hunk, index) {
      var block = node('div', 'diff-hunk');
      var old_ = node('pre', 'diff-old', hunk.old.join('\n'));
      var new_ = node('pre', 'diff-new', hunk.new.join('\n'));
      block.appendChild(old_);
      block.appendChild(new_);
      var reject = node('button', 'replay-button');
      reject.type = 'button';
      reject.textContent = 'reject this hunk';
      reject.addEventListener('click', function () { rejectHunk(turn, file.path, index); });
      block.appendChild(reject);
      wrap.appendChild(block);
    });
    return wrap;
  }

  function rejectHunk(turn, path, hunk) {
    var body = hunk === null ? { turn: turn, path: path } : { turn: turn, path: path, hunk: hunk };
    post('/api/diff/reject', body).then(function (reply) {
      if (reply.status !== 200) {
        notice('the reject was refused',
          (reply.body && reply.body.error) || 'the face said no without a reason');
        return;
      }
      notice(hunk === null ? 'rejected the whole file' : 'rejected hunk ' + hunk,
        path + ' \u00b7 the checkpoint\u2019s lines are back');
      openReview();
    }).catch(function () { });
  }

  // The keyed panels, by address. One registry, so showing one panel
  // can never forget to hide another — the failure mode the explicit
  // hide-lists were one new surface away from.
  var PANEL_EL = {
    chat: el.panelChat,
    router: el.panelRouter,
    delegation: el.panelDelegation,
    memory: el.panelMemory,
    skills: el.panelSkills,
    sessions: el.panelSessions,
    calibration: el.panelCalibration,
    tower: el.panelTower,
    'second-brain': el.panelSecondBrain,
    settings: el.panelSettings
  };

  function showRoute() {
    var which = (window.location.hash || '').replace(/^#\/?/, '');
    var known = ROUTES.filter(function (route) { return route[0] === which; })[0];
    var address = which === 'chat' || !known ? 'chat' : which;
    face.panel = address;
    markRail();
    Object.keys(PANEL_EL).forEach(function (key) {
      PANEL_EL[key].hidden = key !== address;
    });
    el.panelLater.hidden = Boolean(PANEL_EL[address]);
    if (!PANEL_EL[address]) {
      var named = known || [which, which];
      el.laterTitle.textContent = named[1];
      el.laterNote.textContent = named[1] + ' is a later increment of the same plan. ' +
        'The conversation is live now, and every surface reads the one event log this face already holds.';
      return;
    }
    // The conversation is live from the log; every board paints now and
    // again when its own read lands.
    var arrive = {
      router: function () { paintRouter(); readRouter(true).then(paintRouter); },
      delegation: function () { paintDelegation(); readDelegation(true).then(paintDelegation); },
      memory: function () { paintMemory(); readMemory(true).then(paintMemory); },
      skills: function () { paintSkills(); readSkills(true).then(paintSkills); },
      sessions: function () { paintSessions(); readSessions(true).then(paintSessions); },
      calibration: function () { paintCalibration(); readCalibration(true).then(paintCalibration); },
      tower: function () { paintTower(); readTower(true).then(paintTower); },
      'second-brain': function () { paintBrain(); readBrain(true).then(paintBrain); },
      settings: function () { paintSettings(); readSettings(true).then(paintSettings); }
    };
    if (arrive[address]) arrive[address]();
  }

  // ------------------------------------------------------------ the pane

  // The pane reads the working directory through two routes the bridge
  // contains: the tree is one directory's entries, the file is one
  // file's text. Both are behind the launch token like every other
  // route, and the bridge refuses a credential file by name and a path
  // that escapes the root — the browser can ask for a file now, which is
  // exactly where a credential would leak.

  function paneGet(path) {
    return fetch(path, { headers: { accept: 'application/json' } })
      .then(function (reply) {
        if (reply.status === 401) throw new Error('token');
        return reply.json();
      });
  }

  // The tree, for one directory. A click on a directory reads it; a click
  // on a file opens it in the viewer and the preview.
  function readTree(asked, force) {
    var now = Date.now();
    if (!force && face.tree && face.treeAt === asked && now - face.treeAt2 < 4000) {
      return Promise.resolve();
    }
    face.treeAt = asked;
    face.treeAt2 = now;
    return paneGet('/api/tree?path=' + encodeURIComponent(asked))
      .then(function (doc) {
        face.tree = doc;
        if (face.paneTab === 'files') paintTree();
      })
      .catch(function () { });
  }

  function paintTree() {
    var read = face.tree;
    el.paneTree.textContent = '';
    if (!read) {
      el.paneTree.appendChild(node('p', 'pane-empty', 'The tree read has not landed yet.'));
      return;
    }
    if (read.refused && read.refused.length) {
      el.paneTree.appendChild(node('p', 'pane-empty', read.refused.join(' \u00b7 ')));
      return;
    }
    // The breadcrumb: where in the tree this read stands, and a way up.
    var crumb = node('div', 'pane-crumb');
    var up = node('button', 'pane-crumb-up');
    up.type = 'button';
    up.textContent = read.path ? '\u2026 / ' + clip(read.path, 26) : '/';
    up.addEventListener('click', function () {
      var parts = String(read.path || '').split('/');
      parts.pop();
      readTree(parts.join('/'), true);
    });
    crumb.appendChild(up);
    el.paneTree.appendChild(crumb);

    (read.entries || []).forEach(function (entry) {
      var row = node('button', 'pane-row');
      row.type = 'button';
      row.dataset.kind = entry.kind;
      row.appendChild(node('span', 'pane-row-mark', entry.kind === 'dir' ? '\u2338' : '\u00b7'));
      row.appendChild(node('span', 'pane-row-name', entry.name));
      if (entry.kind === 'dir') {
        row.addEventListener('click', function () {
          var at = read.path ? read.path + '/' + entry.name : entry.name;
          readTree(at, true);
        });
      } else {
        row.appendChild(node('span', 'pane-row-size', bytes(entry.size)));
        row.addEventListener('click', function () {
          var at = read.path ? read.path + '/' + entry.name : entry.name;
          openFile(at);
        });
      }
      el.paneTree.appendChild(row);
    });
    if (read.truncated) {
      el.paneTree.appendChild(node('p', 'pane-note',
        'this directory has more entries than one read carries'));
    }
  }

  // One file, in the viewer and in the preview. The viewer shows the
  // actual code with its line numbers; the preview renders it when it is
  // a deliverable — markdown today, and text for everything else.
  function openFile(asked) {
    return paneGet('/api/file?path=' + encodeURIComponent(asked))
      .then(function (doc) {
        face.file = doc;
        paintFile();
        if (face.paneTab === 'preview') paintPreview();
      })
      .catch(function () { });
  }

  function paintFile() {
    var read = face.file;
    el.paneFile.textContent = '';
    if (!read) return;
    if (read.refused && read.refused.length) {
      el.paneFile.appendChild(node('p', 'pane-empty', read.refused.join(' \u00b7 ')));
      return;
    }
    var head = node('div', 'pane-file-head');
    head.appendChild(node('span', 'pane-file-name', read.name));
    head.appendChild(node('span', 'pane-file-meta',
      read.language + ' \u00b7 ' + bytes(read.bytes) + (read.truncated ? ' \u00b7 cut' : '')));
    el.paneFile.appendChild(head);
    var lines = String(read.text || '').split('\n');
    var code = node('div', 'pane-code');
    lines.forEach(function (line, index) {
      var row = node('div', 'pane-line');
      row.appendChild(node('span', 'pane-line-no', String(index + 1)));
      row.appendChild(node('span', 'pane-line-text', line.length ? line : ' '));
      code.appendChild(row);
    });
    el.paneFile.appendChild(code);
  }

  // ------------------------------------------------------- the web tab

  // One page, read. The address is the cache key: re-reading the same
  // address is a refresh, and a new address is a new read. A refused
  // read names its reason rather than showing a blank pane, because a
  // blank pane would read as "this site is empty".
  function readPage(asked, force) {
    var url = (asked || '').trim();
    if (!url) return Promise.resolve();
    if (!force && face.page && face.pageAt === url) return Promise.resolve();
    face.pageAt = url;
    return paneGet('/api/web?url=' + encodeURIComponent(url))
      .then(function (doc) {
        face.page = doc;
        if (face.paneTab === 'web') paintPage();
      })
      .catch(function () { });
  }

  function paintPage() {
    var read = face.page;
    el.panePage.textContent = '';
    if (!read) {
      el.panePage.appendChild(node('p', 'pane-empty',
        'Read an address to see it here. The page is fetched and reduced to its ' +
        'prose — no script from it runs, and a private or loopback address is refused.'));
      return;
    }
    if (read.refused && read.refused.length) {
      el.panePage.appendChild(node('p', 'pane-empty', read.refused.join(' · ')));
      return;
    }
    if (read.title) {
      el.panePage.appendChild(node('h2', 'page-title', read.title));
    }
    var where = node('p', 'page-where', read.url);
    el.panePage.appendChild(where);
    (read.blocks || []).forEach(function (block) {
      var level = Math.min(6, Math.max(3, Number(block.level) || 3));
      if (block.kind === 'heading') {
        el.panePage.appendChild(node('h' + level, 'md-h', block.text));
      } else if (block.kind === 'item') {
        el.panePage.appendChild(node('p', 'page-item', '· ' + block.text));
      } else if (block.kind === 'code') {
        el.panePage.appendChild(node('pre', 'md-code', block.text));
      } else if (block.kind === 'quote') {
        el.panePage.appendChild(node('blockquote', 'page-quote', block.text));
      } else {
        el.panePage.appendChild(node('p', 'md-p', block.text));
      }
    });
    if (read.links && read.links.length) {
      var box = node('div', 'page-links');
      box.appendChild(node('h3', null, read.links.length + ' links on this page'));
      read.links.forEach(function (link) {
        var row = node('button', 'page-link');
        row.type = 'button';
        row.appendChild(node('span', 'page-link-label', link.label));
        row.appendChild(node('span', 'page-link-href', link.href));
        // A link is read the way a typed address is, relative to the
        // page it came from.
        row.addEventListener('click', function () {
          var target = link.href;
          if (target.indexOf('://') === -1) {
            var base = read.url.split('/').slice(0, 3).join('/');
            target = target.charAt(0) === '/' ? base + target : base + '/' + target;
          }
          el.paneAddressBox.value = target;
          readPage(target, true);
        });
        box.appendChild(row);
      });
      el.panePage.appendChild(box);
    }
  }

  // The preview: markdown rendered, everything else shown as text with
  // its own line breaks. A half-written markdown renderer is worse than
  // none, so the subset here is the one this repository's own documents
  // actually use — headings, paragraphs, lists, code fences, emphasis,
  // links and inline code.
  function paintPreview() {
    var read = face.file;
    el.panePreview.textContent = '';
    // A deck the run built renders ahead of any file: it is the
    // deliverable, and the pane is where it is readable.
    if (face.deck && face.deck.html) {
      var head = node('div', 'pane-file-head');
      head.appendChild(node('span', 'pane-file-name', 'a ' + face.deck.project));
      head.appendChild(node('span', 'pane-file-meta',
        face.deck.nodes + ' node' + (face.deck.nodes === 1 ? '' : 's')));
      el.panePreview.appendChild(head);
      el.panePreview.appendChild(node('div', 'deck-frame', face.deck.html));
      return;
    }
    if (!read) {
      el.panePreview.appendChild(node('p', 'pane-empty',
        'Open a file from the tree to preview it here.'));
      return;
    }
    if (read.refused && read.refused.length) {
      el.panePreview.appendChild(node('p', 'pane-empty', read.refused.join(' \u00b7 ')));
      return;
    }
    if (read.language !== 'markdown') {
      var plain = node('pre', 'pane-plain', read.text || '');
      el.panePreview.appendChild(plain);
      return;
    }
    el.panePreview.appendChild(markdown(String(read.text || '')));
  }

  function markdown(text) {
    var out = node('div', 'md');
    var fence = null;
    var list = null;
    var para = [];
    var flushPara = function () {
      if (!para.length) return;
      var p = node('p', 'md-p');
      p.appendChild(inline(para.join(' ')));
      out.appendChild(p);
      para = [];
    };
    var flushList = function () {
      if (!list) return;
      out.appendChild(list);
      list = null;
    };
    text.split('\n').forEach(function (raw) {
      var line = raw.replace(/\s+$/, '');
      var fenceAt = line.match(/^```(\w*)$/);
      if (fenceAt) {
        if (fence) {
          out.appendChild(node('pre', 'md-code', fence.join('\n')));
          fence = null;
        } else {
          flushPara(); flushList();
          fence = [];
        }
        return;
      }
      if (fence) { fence.push(raw); return; }
      if (!line.trim()) { flushPara(); flushList(); return; }
      var heading = line.match(/^(#{1,4})\s+(.*)$/);
      if (heading) {
        flushPara(); flushList();
        var level = Math.min(4, heading[1].length + 2);
        var h = node('h' + level, 'md-h');
        h.appendChild(inline(heading[2]));
        out.appendChild(h);
        return;
      }
      var item = line.match(/^\s*[-*]\s+(.*)$/);
      if (item) {
        flushPara();
        if (!list) list = node('ul', 'md-ul');
        var li = node('li', 'md-li');
        li.appendChild(inline(item[1]));
        list.appendChild(li);
        return;
      }
      flushList();
      para.push(line.trim());
    });
    if (fence) out.appendChild(node('pre', 'md-code', fence.join('\n')));
    flushPara();
    flushList();
    return out;
  }

  // Inline markdown: code, bold, italic, links. Text is built as nodes
  // rather than innerHTML, so a document can never inject markup.
  function inline(text) {
    var span = node('span');
    var rest = String(text);
    var pattern = /(`[^`]+`)|(\*\*[^*]+\*\*)|(\[[^\]]+\]\([^)]+\))/g;
    var at = 0;
    var match;
    while ((match = pattern.exec(rest)) !== null) {
      if (match.index > at) span.appendChild(document.createTextNode(rest.slice(at, match.index)));
      if (match[1]) {
        span.appendChild(node('code', 'md-inline-code', match[1].slice(1, -1)));
      } else if (match[2]) {
        span.appendChild(node('strong', 'md-strong', match[2].slice(2, -2)));
      } else if (match[3]) {
        var link = match[3].match(/^\[([^\]]+)\]\(([^)]+)\)$/);
        var a = node('a', 'md-link', link[1]);
        a.href = link[2];
        span.appendChild(a);
      }
      at = match.index + match[0].length;
    }
    if (at < rest.length) span.appendChild(document.createTextNode(rest.slice(at)));
    return span;
  }

  function bytes(n) {
    n = Number(n) || 0;
    if (n < 1024) return n + ' B';
    if (n < 1024 * 1024) return (n / 1024).toFixed(n < 10240 ? 1 : 0) + ' kB';
    return (n / 1024 / 1024).toFixed(1) + ' MB';
  }

  // ------------------------------------------------- the type-ahead

  // The lexical ranking, which paints in the same frame the key lands.
  // An owner who knows the command types its first letters and expects
  // it now, so this pass cannot wait on anything — not the network, not
  // the embedding backend, not a debounce.
  function rankLexical(asked) {
    var query = asked.toLowerCase();
    var scored = COMMANDS.map(function (command, index) {
      var name = command[0], aliases = command[1], said = command[2];
      var score = 0;
      if (name === query) score = 100;
      else if (name.indexOf(query) === 0) score = 80 - name.length;
      else if (aliases.some(function (alias) { return alias.indexOf(query) === 0; })) score = 60;
      else if (aliases.indexOf(query) !== -1) score = 55;
      else if (said.toLowerCase().indexOf(query) !== -1) score = 30;
      else if (subsequence(query, name)) score = 15;
      return { index: index, score: score };
    });
    return scored
      .filter(function (entry) { return entry.score > 0; })
      .sort(function (left, right) { return right.score - left.score; })
      .map(function (entry) { return entry.index; });
  }

  // Whether `query`'s characters appear in `text` in order, so "thm"
  // finds "theme" the way a fuzzy finder does.
  function subsequence(query, text) {
    var at = 0;
    for (var i = 0; i < text.length && at < query.length; i += 1) {
      if (text[i] === query[at]) at += 1;
    }
    return at === query.length;
  }

  // The semantic pass, debounced and non-blocking. The candidates are
  // this page's own command descriptions; the substrate ranks them
  // against what the owner typed, which is what lets "make it dark"
  // find /theme. A machine with no embedding backend answers with zero
  // scores and the lexical ranking stands — the honest degradation, and
  // not an error the owner has to know about.
  function rankSemantic(intent) {
    if (intent.length < 3) return Promise.resolve();
    return post('/api/rank', {
      asked: intent,
      candidates: COMMANDS.map(function (command) {
        return command[0] + ' ' + command[1].join(' ') + ' ' + command[2];
      })
    }).then(function (reply) {
      if (reply.status !== 200 || !reply.body.ranked) return;
      var best = reply.body.ranked.filter(function (entry) { return entry.score > 0.25; });
      if (!best.length) return;
      var order = best.map(function (entry) { return entry.index; });
      var lexical = face.commands;
      // The semantic order leads, and anything it did not rank keeps its
      // lexical place behind it — so the refinement never loses a match
      // the lexical pass found.
      var merged = order.concat(lexical.filter(function (index) {
        return order.indexOf(index) === -1;
      }));
      face.commands = merged;
      // The lexical pass may have found nothing — an owner describing
      // what they want rarely types a command name — so the semantic
      // answer opens the popover rather than refining a closed one.
      face.commandOpen = true;
      face.commandAt = 0;
      paintCommands();
    }).catch(function () { });
  }

  function paintCommands() {
    el.commands.textContent = '';
    if (!face.commands.length) {
      el.commands.hidden = true;
      return;
    }
    el.commands.hidden = false;
    face.commands.forEach(function (index, at) {
      var command = COMMANDS[index];
      var row = node('button', 'command');
      row.type = 'button';
      row.setAttribute('role', 'option');
      row.dataset.index = String(index);
      if (at === face.commandAt) row.setAttribute('aria-selected', 'true');
      row.appendChild(node('span', 'command-name', '/' + command[0]));
      row.appendChild(node('span', 'command-said', command[2]));
      row.addEventListener('click', function () { runCommand(index); });
      el.commands.appendChild(row);
    });
  }

  function openCommands(asked) {
    face.commandOpen = true;
    face.commands = rankLexical(asked);
    face.commandAt = 0;
    paintCommands();
    window.clearTimeout(face.commandTimer);
    face.commandTimer = window.setTimeout(function () {
      rankSemantic(el.task.value.slice(1).trim());
    }, 180);
  }

  function closeCommands() {
    face.commandOpen = false;
    face.commands = [];
    window.clearTimeout(face.commandTimer);
    el.commands.hidden = true;
  }

  function moveCommand(step) {
    if (!face.commands.length) return;
    face.commandAt = (face.commandAt + step + face.commands.length) % face.commands.length;
    paintCommands();
  }

  // Run one command: the popover closes, the composer clears, and the
  // command's own action takes the rest of the line as its argument.
  function runCommand(index, rest) {
    var command = COMMANDS[index];
    closeCommands();
    el.task.value = '';
    grow();
    command[3](rest || '');
  }

  // Every command, as the help drawer's sections.
  function commandHelp() {
    return COMMANDS.map(function (command) {
      return ['/' + command[0], command[2] + (command[1].length ? ' \u2014 also ' + command[1].join(', ') : '')];
    });
  }

  // The pane's open and closed state, and which of its two views is in
  // front. The pane is layout, not an overlay: it takes its own column
  // beside the stage so a preview and the run are readable together.
  function openPane() {
    el.pane.hidden = false;
    document.body.dataset.pane = 'open';
    if (!face.tree) readTree('', true);
  }

  function closePane() {
    el.pane.hidden = true;
    document.body.dataset.pane = 'closed';
  }

  function showPaneTab(which) {
    face.paneTab = which;
    var files = which === 'files';
    var preview = which === 'preview';
    var web = which === 'web';
    el.tabFiles.classList.toggle('active', files);
    el.tabPreview.classList.toggle('active', preview);
    el.tabWeb.classList.toggle('active', web);
    el.tabFiles.setAttribute('aria-selected', files ? 'true' : 'false');
    el.tabPreview.setAttribute('aria-selected', preview ? 'true' : 'false');
    el.tabWeb.setAttribute('aria-selected', web ? 'true' : 'false');
    el.paneFiles.hidden = !files;
    el.panePreviewPanel.hidden = !preview;
    el.paneWeb.hidden = !web;
    if (files) paintTree();
    else if (preview) paintPreview();
    else paintPage();
  }

  el.paneAddress.addEventListener('submit', function (event) {
    event.preventDefault();
    readPage(el.paneAddressBox.value, true);
  });

  // ------------------------------------------------- the rail and the palette

  // The rail: four destinations, one flat level beneath each. Built from
  // GROUPS, the same declaration the palette searches, so the two can
  // never disagree about where a surface lives.
  function buildRail() {
    el.rail.textContent = '';
    GROUPS.forEach(function (group) {
      var id = group[0], label = group[1], items = group[2];
      var button = node('button', 'surfaces-dest');
      button.type = 'button';
      button.dataset.group = id;
      button.appendChild(node('span', 'surfaces-mark', GROUP_MARKS[id] || label.slice(0, 1)));
      button.appendChild(node('span', 'surfaces-name', label));
      // The destination itself opens its first surface — the one an owner
      // opening the group almost always wants — so a group is one click
      // rather than two.
      button.addEventListener('click', function () {
        window.location.hash = '#/' + items[0][0];
      });
      el.rail.appendChild(button);
    });
  }

  // Which surfaces are live today, and which are a later increment — the
  // rail says so rather than pretending all ten are built.
  var BUILT = ['chat', 'router', 'delegation', 'memory', 'skills', 'sessions', 'calibration',
    'tower', 'second-brain', 'settings'];

  function markRail() {
    var which = (window.location.hash || '').replace(/^#\/?/, '');
    Array.prototype.forEach.call(el.rail.querySelectorAll('.surfaces-dest'), function (button) {
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

  // The theme set. Each one is the alias tier re-pointed, so a theme is
  // one block of custom properties and nothing else moves — the component
  // tokens read the aliases, which is the whole reason the chain has
  // three tiers. Every theme's text tokens clear 4.5:1 against all three
  // of its surfaces; the contrast theme is the accessibility one and
  // trades the quiet look for the maximum separation.
  var THEMES = [
    ['console', 'console', 'warm dark \u00b7 the default, a console left open beside the work'],
    ['paper', 'paper', 'warm light \u00b7 the same structure over a warm paper ground'],
    ['graphite', 'graphite', 'cool dark \u00b7 for an owner who finds the warm dark too brown'],
    ['contrast', 'contrast', 'high contrast \u00b7 pure ground, near-black ink, the accessibility theme']
  ];

  // The theme. One attribute on the document element is the whole
  // switch — the alias tier re-points under it and every component token
  // follows — so there is one thing to set and nothing else to keep in
  // step. Console is the default; the choice is remembered.
  function themeNow() {
    var now = document.documentElement.dataset.theme;
    return THEMES.some(function (theme) { return theme[0] === now; }) ? now : 'console';
  }

  function setTheme(mode) {
    document.documentElement.dataset.theme = mode;
    el.themeToggle.dataset.theme = mode;
    var named = THEMES.filter(function (theme) { return theme[0] === mode; })[0];
    el.themeToggle.setAttribute('aria-label', 'choose a theme \u2014 now ' + (named ? named[1] : mode));
    el.themeToggle.title = 'choose a theme \u2014 now ' + (named ? named[1] : mode);
    try { window.localStorage.setItem('fleet-theme', mode); } catch (error) { }
  }

  setTheme(themeNow());
  // The theme measurement, read once so the picker's ratios and the
  // action card carry the substrate's own numbers rather than a swatch.
  readContrast(true);
  // The toggle opens the picker rather than flipping: with four themes a
  // blind cycle is a guess, and a one-click choice is not. The picker is
  // the same shape the commands use, so one interaction pattern serves
  // both.
  el.themeToggle.addEventListener('click', function () { openThemes(); });

  function openThemes() {
    el.themes.textContent = '';
    THEMES.forEach(function (theme) {
      var row = node('button', 'theme-row');
      row.type = 'button';
      row.dataset.theme = theme[0];
      row.appendChild(node('span', 'theme-row-name', theme[1]));
      row.appendChild(node('span', 'theme-row-said', theme[2]));
      if (themeNow() === theme[0]) row.setAttribute('aria-selected', 'true');
      row.addEventListener('click', function () {
        setTheme(theme[0]);
        closeThemes();
        readContrast();
      });
      // The measured ratio rides the row, so the choice is made against
      // a number rather than a swatch.
      var report = (face.contrast && face.contrast.themes || [])
        .filter(function (entry) { return entry.theme === theme[0]; })[0];
      if (report) {
        var mark = node('span', 'theme-row-ratio',
          report.worst.toFixed(2) + ':1' + (report.holds ? '' : ' \u00b7 under the floor'));
        if (!report.holds) mark.dataset.tone = 'down';
        row.appendChild(mark);
      }
      el.themes.appendChild(row);
    });
    el.themes.hidden = false;
  }

  // The theme measurement, read once per launch and again whenever the
  // theme changes. It is the substrate's own computation, so the card and
  // the guard cannot disagree.
  function readContrast(force) {
    if (!force && face.contrast) return Promise.resolve();
    return paneGet('/api/contrast')
      .then(function (doc) {
        face.contrast = doc;
        paintContrast();
      })
      .catch(function () { });
  }

  // The action card. It appears only when the current theme does not
  // clear the floor, it names the measurement, and its one button is the
  // fix already computed — the readable theme, not a promise to find
  // one. That is the whole pattern: the interface calculates the flaw
  // and offers the correction as one click, before the owner hits it.
  function paintContrast() {
    var doc = face.contrast;
    if (!el.actions) return;
    el.actions.textContent = '';
    var here = doc && (doc.themes || []).filter(function (entry) {
      return entry.theme === themeNow();
    })[0];
    if (!here || here.holds) { el.actions.hidden = true; return; }
    var best = (doc.themes || []).filter(function (entry) { return entry.holds; })
      .sort(function (left, right) { return right.worst - left.worst; })[0];
    var card0 = node('div', 'action');
    card0.dataset.kind = 'contrast';
    card0.appendChild(node('span', 'action-mark', '\u25d0'));
    var body = node('div', 'action-body');
    body.appendChild(node('p', 'action-title', 'this theme falls below the readable floor'));
    body.appendChild(node('p', 'action-said',
      'its worst text is ' + here.worst.toFixed(2) + ':1 against the ' + doc.floor.toFixed(1) +
      ':1 floor \u2014 ' + here.failing.map(function (entry) {
        return entry.token + ' at ' + entry.ratio.toFixed(2) + ':1';
      }).join(', ')));
    if (best) {
      var fix = node('button', 'action-do');
      fix.type = 'button';
      fix.textContent = 'switch to ' + best.theme + ' (' + best.worst.toFixed(2) + ':1)';
      fix.addEventListener('click', function () {
        setTheme(best.theme);
        readContrast(true);
      });
      body.appendChild(fix);
    }
    card0.appendChild(body);
    el.actions.appendChild(card0);
    el.actions.hidden = false;
  }

  function closeThemes() { el.themes.hidden = true; }

  // The pane's controls. Its shortcut is cmd+backslash — the editor
  // convention for a side pane — and Esc from a pane with nothing open
  // comes home rather than closing it, so one key always means one thing.
  el.paneOpen.addEventListener('click', openPane);
  el.paneClose.addEventListener('click', closePane);
  el.tabFiles.addEventListener('click', function () { showPaneTab('files'); });
  el.tabPreview.addEventListener('click', function () { showPaneTab('preview'); });
  el.tabWeb.addEventListener('click', function () { showPaneTab('web'); });

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
    if ((event.metaKey || event.ctrlKey) && event.key === '\\') {
      event.preventDefault();
      if (el.pane.hidden) openPane(); else closePane();
      return;
    }
    if (event.key !== 'Escape') return;
    if (!el.palette.hidden) { closePalette(); return; }
    if (!el.pane.hidden && !face.file) { closePane(); return; }
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
