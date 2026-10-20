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
// The face has no credential: the boundary is the machine, and the
// server checks that a request's Host and Origin name it. The address
// alone opens the face, and a reload is the same address again.
(function () {
  'use strict';

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
    ['second-brain', 'second brain'],
    ['bots', 'bots']
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
      ['bots', 'bots'],
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
  // The marks. The rail's places and the empty state's offers use the
  // same 24-unit icon set, drawn here rather than shipped as a second
  // asset: a stroke icon is geometry, and geometry belongs with the code
  // that places it. GROUP_MARKS remains the letter fallback the palette
  // can use when a surface has no icon of its own.
  var GROUP_MARKS = { work: 'w', intelligence: 'i', knowledge: 'k', system: 's' };

  var ICONS = {
    chat: [['path', { d: 'M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z' }]],
    sessions: [
      ['circle', { cx: 12, cy: 12, r: 8.5 }],
      ['path', { d: 'M12 7.5V12l3 1.8' }]
    ],
    router: [
      ['path', { d: 'M4 7h5a4 4 0 0 1 4 4v2a4 4 0 0 0 4 4h3' }],
      ['circle', { cx: 4, cy: 7, r: 1.6 }],
      ['circle', { cx: 20, cy: 17, r: 1.6 }]
    ],
    delegation: [
      ['circle', { cx: 6, cy: 6, r: 2 }],
      ['circle', { cx: 18, cy: 6, r: 2 }],
      ['circle', { cx: 12, cy: 18, r: 2 }],
      ['path', { d: 'M7.4 7.3 10.8 16.2' }],
      ['path', { d: 'M16.6 7.3 13.2 16.2' }],
      ['path', { d: 'M8 6h8' }]
    ],
    memory: [
      ['path', { d: 'M4 19.5A2.5 2.5 0 0 1 6.5 17H20' }],
      ['path', { d: 'M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z' }]
    ],
    skills: [
      ['path', { d: 'M12 2 2 7l10 5 10-5-10-5z' }],
      ['path', { d: 'M2 12l10 5 10-5' }],
      ['path', { d: 'M2 17l10 5 10-5' }]
    ],
    'second-brain': [
      ['rect', { x: 3.5, y: 3.5, width: 7, height: 7, rx: 1.5 }],
      ['rect', { x: 13.5, y: 3.5, width: 7, height: 7, rx: 1.5 }],
      ['rect', { x: 3.5, y: 13.5, width: 7, height: 7, rx: 1.5 }],
      ['rect', { x: 13.5, y: 13.5, width: 7, height: 7, rx: 1.5 }]
    ],
    bots: [
      ['rect', { x: 4, y: 8, width: 16, height: 11, rx: 2.5 }],
      ['path', { d: 'M12 8V4.5' }],
      ['circle', { cx: 12, cy: 3.4, r: 1.3 }],
      ['path', { d: 'M9 13v1.5' }],
      ['path', { d: 'M15 13v1.5' }],
      ['path', { d: 'M2.5 12.5v3' }],
      ['path', { d: 'M21.5 12.5v3' }]
    ],
    settings: [
      ['path', { d: 'M4 7h10' }],
      ['path', { d: 'M18 7h2' }],
      ['circle', { cx: 16, cy: 7, r: 2 }],
      ['path', { d: 'M4 17h4' }],
      ['path', { d: 'M12 17h8' }],
      ['circle', { cx: 10, cy: 17, r: 2 }]
    ],
    calibration: [['path', { d: 'M3 12h4l2.5-7 4 14 2.5-7h5' }]],
    tower: [
      ['path', { d: 'M5 20V10' }],
      ['path', { d: 'M12 20V4' }],
      ['path', { d: 'M19 20v-7' }],
      ['path', { d: 'M3 20h18' }]
    ],
    plan: [
      ['path', { d: 'M9 4h6v3H9z' }],
      ['path', { d: 'M9 4H7a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V6a2 2 0 0 0-2-2h-2' }]
    ],
    repo: [
      ['path', { d: 'M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z' }]
    ],
    dot: [['circle', { cx: 12, cy: 12, r: 6 }]]
  };

  // Which group an address belongs to, and its label in that group —
  // derived once so the rail, the palette and the panel all agree.
  var WHERE = {};
  GROUPS.forEach(function (group) {
    group[2].forEach(function (item) { WHERE[item[0]] = [group[0], item[1]]; });
  });

  var el = {
    rail: document.getElementById('rail'),
    railTree: document.getElementById('rail-tree'),
    railToggle: document.getElementById('rail-toggle'),
    railToggleName: document.getElementById('rail-toggle-name'),
    railJump: document.getElementById('rail-jump'),
    railMenu: document.getElementById('rail-menu'),
    railScrim: document.getElementById('rail-scrim'),
    railWhere: document.getElementById('rail-where'),
    asideToggle: document.getElementById('aside-toggle'),
    dock: document.getElementById('dock'),
    dockObject: document.getElementById('dock-object'),
    dockObjectTitle: document.getElementById('dock-object-title'),
    dockObjectBody: document.getElementById('dock-object-body'),
    dockObjectClose: document.getElementById('dock-object-close'),
    faceCard: document.getElementById('face-card'),
    trustCard: document.getElementById('trust-card'),
    processCard: document.getElementById('process-card'),
    processState: document.getElementById('process-state'),
    processBody: document.getElementById('process-body'),
    processControls: document.getElementById('process-controls'),
    processStop: document.getElementById('process-stop'),
    processSteerForm: document.getElementById('process-steer-form'),
    processSteer: document.getElementById('process-steer'),
    processStrip: document.getElementById('process-strip'),
    processPill: document.getElementById('process-pill'),
    crumbGroup: document.getElementById('crumb-group'),
    crumbPage: document.getElementById('crumb-page'),
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
    panelBots: document.getElementById('panel-bots'),
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
    botsBoard: document.getElementById('bots-board'),
    settingsBoard: document.getElementById('settings-board'),
    routePill: document.getElementById('route-pill'),
    statePill: document.getElementById('state-pill'),
    spend: document.getElementById('spend'),
    context: document.getElementById('context'),
    contextMeter: document.getElementById('context-meter'),
    contextFill: document.getElementById('context-fill'),
    log: document.getElementById('log'),
    empty: document.getElementById('empty'),
    emptyEyebrow: document.getElementById('empty-eyebrow'),
    emptyTitle: document.getElementById('empty-title'),
    emptyLede: document.getElementById('empty-lede'),
    emptyMeta: document.getElementById('empty-meta'),
    offers: document.getElementById('offers'),
    recent: document.getElementById('recent'),
    homeStats: document.getElementById('home-stats'),
    homeFacts: document.getElementById('home-facts'),
    homeSkills: document.getElementById('home-skills'),
    homeRuns: document.getElementById('home-runs'),
    homeKnown: document.getElementById('home-known'),
    next: document.getElementById('next'),
    nextEyebrow: document.getElementById('next-eyebrow'),
    nextGoal: document.getElementById('next-goal'),
    nextGo: document.getElementById('next-go'),
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
    workspace: '',
    // The welcome's one read and its clock. The panels keep their own
    // reads; this document is the bounded summary that fills the home.
    home: null,
    homeAt: 0,
    // The selected object, shared by every panel so a repaint does not
    // lose the row the owner chose. The address mirrors it.
    focus: null,
    // A panel's own filters, held in memory for this visit only.
    sessionsFilter: 'all',
    memoryFilter: 'live',
    skillsFilter: 'all',
    calibrationFilter: 'all',
    towerFilter: 'all',
    towerDay: '',
    routerHour: null,
    routerModel: '',
    // The visit stamp, read before it is overwritten, so the home can
    // say what happened while the owner was away.
    lastVisit: '',
    // The nav mode the owner had before the dock auto-folded it at the
    // mid breakpoint. Restored when the dock closes or the screen widens.
    railBeforeDock: null,
    // The live process: the run's own stage, turn, tool, and counts,
    // reduced from the same event log the transcript reads.
    process: null,
    // The verify step's gate, read from the state document: the same
    // failover knob the route module owns.
    gate: null,
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
    // Related documents by kind:id, held for a minute: a board that
    // repaints on its clock must not re-ask the index on every tick.
    related: {},
    // The browser's own counters and the knob that switches their
    // effect off. One exposure is recorded per id per visit, so a
    // repaint is not a second showing.
    usage: null,
    offerShown: {},
    relatedShown: {},
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
    // Bumped whenever the type-ahead closes, whether by a command run or
    // by the owner's hand: an in-flight semantic reply from before the
    // bump is a reply to a question the owner stopped asking, and it must
    // not reopen the popover.
    commandEpoch: 0,
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
    // The action cards: every source that measures a flaw and pre-bakes
    // a correction pushes one here, and the strip above the composer
    // renders them all — the deck's layout faults, the theme's contrast,
    // whatever measures next. One shape, one place, regardless of which
    // surface the work is in.
    actionCards: [],
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
    {
      id: 'plan',
      icon: 'plan',
      title: 'Read the plan',
      said: 'Find the next increment and what it still owes.',
      prompt: 'read the plan and tell me what the next increment owes'
    },
    {
      id: 'review',
      icon: 'sessions',
      title: 'Review today\u2019s changes',
      said: 'See every file the run touched, hunk by hunk, before it lands.',
      prompt: '/review'
    },
    {
      id: 'memory',
      icon: 'memory',
      title: 'Ask what fleet knows',
      said: 'Search the corpus for the decisions you already made.',
      prompt: 'what do the memories say about the fleet router'
    },
    {
      id: 'router',
      icon: 'router',
      title: 'Walk the router',
      said: 'Send a small task and see the chain it chose, and why.',
      prompt: 'walk the router\u2019s chain for a small task and report what it did'
    }
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

  // One stroke icon, from the declaration above. A surface's mark is its
  // own shape, not a letter in a box: the rail and the palette should
  // read at a glance without inventing a second alphabet.
  function icon(name, className) {
    var made = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
    made.setAttribute('viewBox', '0 0 24 24');
    made.setAttribute('fill', 'none');
    made.setAttribute('stroke', 'currentColor');
    made.setAttribute('stroke-width', '1.6');
    made.setAttribute('stroke-linecap', 'round');
    made.setAttribute('stroke-linejoin', 'round');
    made.setAttribute('aria-hidden', 'true');
    if (className) made.setAttribute('class', className);
    (ICONS[name] || ICONS.dot).forEach(function (part) {
      var shape = document.createElementNS('http://www.w3.org/2000/svg', part[0]);
      Object.keys(part[1]).forEach(function (key) { shape.setAttribute(key, part[1][key]); });
      made.appendChild(shape);
    });
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
    // The core reports an owner's stop through `failed`; the reason is
    // the only thing that separates it from a failure, and calling the
    // owner's own decision a failure would be the wrong card.
    var stopped = event.type === 'failed' && /stopped by the owner/i.test(event.reason || '');
    var failed = event.type === 'failed' && !stopped;
    var el0 = card(failed ? 'failed' : 'finished', stopped ? '\u25a0' : failed ? '!' : '\u2713',
      stopped ? 'stopped by you' : failed ? 'failed' : 'finished', event.reason, null, null);
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
        if (event.role === 'owner') {
          face.running = true;
          face.stop = null;
          ownerRow(event);
          break;
        }
        break;
      case 'token':
        if (!face.assistant) face.assistant = assistantRow();
        face.buffer += event.text;
        schedule();
        break;
      case 'turn':
        face.running = true;
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
        face.running = false;
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
        face.running = true;
        face.seat = event;
        break;
      case 'seat_cleared':
        face.seat = null;
        break;
      // The resolution of a seat this face did not answer: the seat is
      // gone, and the page says who took it rather than leaving a button
      // standing for a decision already made. A seat this face did answer
      // arrives as `seat_cleared`, so the two are told apart.
      case 'answered':
        if (face.seat && (!event.session || event.session === face.session)) {
          face.seat = null;
          face.answeredElsewhere = event.by || 'another surface';
          // The seat is gone and the owner should know whose decision it
          // was: a button that vanishes without a word reads as a bug.
          notice('the seat was answered elsewhere',
            (event.by || 'another surface') + ' answered it ' +
            (event.decision === 'deny' ? 'no' : 'yes') + '.');
        }
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
    advanceProcess(event);
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
      // An owner's stop reaches the page as a failure whose reason names
      // the owner. It is not a failure, and the header must not say so.
      var stopped = face.stop && face.stop.type === 'failed' &&
        /stopped by the owner/i.test(face.stop.reason || '');
      el.statePill.textContent = stopped ? 'stopped'
        : face.stop ? (face.stop.type === 'failed' ? 'failed' : 'idle') : 'idle';
      el.statePill.dataset.state = face.stop && face.stop.type === 'failed' && !stopped
        ? 'down' : 'idle';
    }

    paintFacts();
    paintRules();
    paintSeat();
    paintComposer();
    paintProcess();
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
      ['state', face.seat ? 'a seat is standing' : face.running ? 'running'
        : face.stop ? (/stopped by the owner/i.test(face.stop.reason || '') ? 'stopped' : face.stop.type)
          : 'idle']
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

  // ------------------------------------------------------------ the process

  // A run is a loop, and the owner is a visual person: route, think, act,
  // verify, with the current step, the turn, the tool, and the elapsed
  // time. The state is reduced from the same event log the transcript
  // reads, so the card cannot claim a step the run did not take.
  var RUN_STEPS = [
    ['route', 'Route', 'where the work goes'],
    ['think', 'Think', 'the model\u2019s turn'],
    ['act', 'Act', 'tools, memory, lanes'],
    ['verify', 'Verify', 'the answer and the gate']
  ];
  var PLAN_STEPS = [
    ['plan', 'Plan', 'the run writes nothing'],
    ['review', 'Review', 'read and edit the plan'],
    ['build', 'Build', 'post the edited plan']
  ];

  function resetProcess(mode, replayed) {
    face.process = {
      mode: mode || 'run',
      stage: mode === 'plan' ? 'plan' : 'route',
      active: true,
      // A replay cannot know when the run began, only when this page
      // read it. The card says nothing rather than counting from the
      // moment the owner opened it.
      replayed: !!replayed,
      started: Date.now(),
      ended: 0,
      turn: 0,
      tool: '',
      calls: 0,
      provider: '',
      model: '',
      facts: 0,
      delegations: 0,
      lanes: 0,
      checkpoint: 0,
      waiting: false,
      stop: '',
      stopping: false,
      branches: {},
      branchOrder: [],
      gate: face.gate || null,
      last: '',
      at: 0
    };
  }

  function ensureProcess() {
    if (!face.process) resetProcess(face.planMode ? 'plan' : 'run');
    return face.process;
  }

  function advanceProcess(event) {
    if (event.type === 'message' && event.role === 'owner') {
      resetProcess(face.planMode ? 'plan' : 'run');
      face.process.last = 'you sent the task';
      face.process.at = Date.now();
      return;
    }
    if (!face.process) {
      // A replayed session can begin at any point. The first run-shaped
      // event creates the card; a cron or a skills boot event does not.
      if (['turn', 'routed', 'tool_call', 'tool_result', 'seat', 'memory',
        'lane', 'delegation', 'control', 'plan', 'finished', 'failed'].indexOf(event.type) === -1) {
        return;
      }
      resetProcess(face.planMode ? 'plan' : 'run', true);
    }
    var p = face.process;
    p.at = Date.now();
    switch (event.type) {
      case 'routed':
        p.provider = event.provider || '';
        p.model = event.model || '';
        if (!p.turn) p.stage = 'route';
        p.last = 'routed to ' + (p.provider || 'a provider') +
          (p.model ? ' \u00b7 ' + p.model : '');
        break;
      case 'turn':
        p.stage = 'think';
        p.turn = Number(event.turn) || p.turn;
        p.last = 'turn ' + p.turn;
        break;
      case 'tool_call':
        p.stage = 'act';
        p.tool = event.tool || '';
        p.calls += 1;
        p.last = p.tool + (event.summary ? ' \u00b7 ' + clip(event.summary, 60) : '');
        break;
      case 'tool_result':
        p.stage = 'act';
        p.waiting = false;
        p.last = (event.tool || 'tool') + ' \u00b7 ' +
          (event.approved ? 'done' : 'refused');
        break;
      case 'seat':
        p.stage = 'act';
        p.waiting = true;
        p.last = 'waiting for your approval';
        break;
      case 'seat_cleared':
        p.waiting = false;
        p.last = 'your answer crossed';
        break;
      case 'memory':
        p.stage = 'act';
        p.facts = Number(event.facts) || 0;
        p.last = 'memory \u00b7 ' + p.facts + ' facts' +
          (event.shape ? ' \u00b7 ' + event.shape : '');
        break;
      case 'lane':
        p.stage = 'act';
        p.lanes += 1;
        p.last = 'lane ' + (event.lane || '') + ' \u00b7 ' + (event.tools || 0) + ' tools';
        break;
      case 'delegation':
        p.stage = 'act';
        // One branch per child session, rewritten by each stage. The
        // order is the order the children were spawned, so the card
        // reads like the tree it came from rather than like its last
        // report.
        if (event.session) {
          if (!p.branches[event.session]) p.branchOrder.push(event.session);
          p.branches[event.session] = {
            session: event.session,
            goal: event.goal || '',
            stage: event.stage || 'spawned',
            calls: event.detail && event.detail.calls ? Number(event.detail.calls) : 0,
            tool: event.detail && event.detail.tool ? event.detail.tool : '',
            outcome: event.detail && event.detail.outcome ? event.detail.outcome : '',
            turns: event.detail && event.detail.turns ? Number(event.detail.turns) : 0,
            at: Date.now()
          };
        }
        p.delegations = p.branchOrder.length;
        p.last = 'delegation \u00b7 ' + (event.stage || 'spawned') +
          (event.detail && event.detail.tool ? ' \u00b7 ' + event.detail.tool : '');
        break;
      case 'control':
        // The owner's own instruction, recorded by the bridge when the
        // control file was written. The run reads it at its next model
        // call, so the card says "stopping" until the run ends rather
        // than pretending the stop was instant.
        p.stage = 'act';
        if (event.kind === 'stop') {
          p.stopping = true;
          p.last = 'stopping \u00b7 the run ends at its next step';
        } else {
          p.last = 'steer sent \u00b7 ' + clip(event.steer || '', 50);
        }
        break;
      case 'checkpoint':
        p.checkpoint = Number(event.turn) || p.checkpoint;
        p.last = 'checkpoint \u00b7 turn ' + p.checkpoint;
        break;
      case 'compacted':
        p.last = 'compacted \u00b7 ' + (event.turns || 0) + ' turns folded';
        break;
      case 'plan':
        p.mode = 'plan';
        p.stage = 'review';
        p.active = false;
        p.ended = Date.now();
        p.stop = 'plan';
        p.stopping = false;
        p.last = 'the plan is ready to edit';
        break;
      case 'finished':
        if (p.mode === 'plan' && p.stage === 'review') {
          p.last = 'the plan is ready to edit';
          break;
        }
        p.stage = 'verify';
        p.active = false;
        p.ended = Date.now();
        p.stop = 'finished';
        p.stopping = false;
        if (event.turns) p.turn = Number(event.turns) || p.turn;
        p.last = 'finished \u00b7 ' + (event.stop_reason || 'the answer landed');
        break;
      case 'failed':
        p.stage = 'verify';
        p.active = false;
        p.ended = Date.now();
        // A stop by the owner is a different end from a failure: the
        // core reports both through `failed`, and only the reason says
        // which. The card must not call the owner's own decision a
        // failure.
        var failedReason = event.reason || event.stop_reason || 'the run stopped';
        p.stop = /stopped by the owner/i.test(failedReason) ? 'stopped' : 'failed';
        p.stopping = false;
        p.last = (p.stop === 'stopped' ? 'stopped by you' : 'failed') + ' \u00b7 ' + failedReason;
        break;
      default:
        break;
    }
  }

  function elapsedText(start, end) {
    var seconds = Math.max(0, Math.floor(((end || Date.now()) - start) / 1000));
    return Math.floor(seconds / 60) + ':' + String(seconds % 60).padStart(2, '0');
  }

  function processSteps(p) {
    var steps = p.mode === 'plan' ? PLAN_STEPS : RUN_STEPS;
    var order = steps.map(function (step) { return step[0]; });
    var at = order.indexOf(p.stage);
    if (at < 0) at = 0;
    var wrap = node('div', 'process-steps');
    steps.forEach(function (step, index) {
      var state = 'pending';
      if (!p.active && p.mode === 'plan' && p.stage === 'review') {
        state = index < at ? 'done' : index === at ? 'current' : 'pending';
      } else if (!p.active) {
        state = index <= at ? 'done' : 'pending';
        if (p.stop === 'failed' && index === at) state = 'down';
      } else {
        state = index < at ? 'done' : index === at ? 'current' : 'pending';
      }
      var line = node('div', 'process-step');
      line.dataset.state = state;
      line.appendChild(node('span', 'process-dot'));
      var body = node('span', null);
      body.appendChild(node('span', 'process-label', step[1]));
      body.appendChild(node('span', 'process-note', processNote(step[0], p)));
      line.appendChild(body);
      wrap.appendChild(line);
    });
    return wrap;
  }

  function processNote(key, p) {
    switch (key) {
      case 'route':
        return p.provider ? p.provider + (p.model ? ' \u00b7 ' + p.model : '') : 'waiting for the decision';
      case 'think':
        return p.turn ? 'turn ' + p.turn : 'the model\u2019s turn';
      case 'act':
        if (p.waiting) return 'waiting for your approval';
        if (p.tool) return p.tool + (p.calls > 1 ? ' \u00b7 ' + p.calls + ' calls' : '');
        if (p.calls) return p.calls + ' calls';
        return p.active ? 'no tool yet' : 'no tool this run';
      case 'verify':
        if (p.stop === 'failed') return 'the run stopped';
        if (p.stop === 'stopped') return 'stopped by you';
        // The gate is the route module's own failover knob, read from
        // the state document: "on" means the calibration may move the
        // preference, and the step says so instead of describing a
        // check the run did not make.
        return p.gate
          ? (p.gate.informs_failover
            ? 'gate on \u00b7 ' + (p.gate.name || 'sys1')
            : 'gate shut')
          : 'the answer and the gate';
      case 'plan':
        return 'the run writes nothing';
      case 'review':
        return p.stage === 'review' ? 'ready to edit' : 'read and edit the plan';
      case 'build':
        return 'post the edited plan';
      default:
        return '';
    }
  }

  function paintProcess() {
    if (!el.processBody) return;
    var p = face.process;
    var state = !p ? 'idle'
      : p.active ? 'running'
        : p.stop === 'failed' ? 'failed'
          : p.stop === 'stopped' ? 'stopped'
            : p.mode === 'plan' && p.stage === 'review' ? 'review' : 'done';
    el.processState.textContent = state;
    el.processState.dataset.state = state === 'review' ? 'running' : state;
    if (el.processPill) {
      el.processPill.hidden = !p || (!p.active && !(p.mode === 'plan' && p.stage === 'review'));
      if (!el.processPill.hidden) {
        el.processPill.textContent = p.turn ? 'turn ' + p.turn + ' \u00b7 ' + p.stage
          : p.mode === 'plan' ? 'plan mode' : p.stage;
        el.processPill.dataset.state = state === 'review' ? 'running' : state;
      }
    }
    el.processBody.textContent = '';
    if (!p) {
      el.processBody.appendChild(node('p', 'process-idle',
        'No run is in flight. The loop starts when you ask for something: route, think, act, verify.'));
      var start = node('button', 'replay-button replay-resume', 'start with Plan');
      start.type = 'button';
      start.addEventListener('click', function () {
        el.plan.click();
        el.task.focus();
      });
      el.processBody.appendChild(start);
      paintProcessControls(null);
      paintProcessStrip(null);
      return;
    }
    var head = node('div', 'process-head-row');
    head.appendChild(node('span', 'process-state',
      p.mode === 'plan' ? 'plan mode' : p.active ? 'in flight'
        : p.stop === 'failed' ? 'stopped'
          : p.stop === 'stopped' ? 'stopped by you' : 'done'));
    head.lastChild.dataset.state = state === 'review' ? 'running' : state;
    if (!p.replayed) {
      head.appendChild(node('span', 'process-elapsed', elapsedText(p.started, p.ended)));
    }
    el.processBody.appendChild(head);
    el.processBody.appendChild(processSteps(p));
    var branches = processBranches(p);
    if (branches) el.processBody.appendChild(branches);
    var meta = node('div', 'process-meta');
    if (p.turn) meta.appendChild(node('span', null, 'turn ' + p.turn));
    if (p.calls) meta.appendChild(node('span', null, p.calls + ' calls'));
    if (p.facts) meta.appendChild(node('span', null, p.facts + ' facts'));
    if (p.delegations) meta.appendChild(node('span', null, p.delegations + ' delegations'));
    if (p.lanes) meta.appendChild(node('span', null, p.lanes + ' lanes'));
    if (p.checkpoint) meta.appendChild(node('span', null, 'checkpoint ' + p.checkpoint));
    if (meta.children.length) el.processBody.appendChild(meta);
    if (p.last) {
      var last = node('p', 'process-last');
      last.appendChild(node('b', null, 'last'));
      last.appendChild(document.createTextNode(' \u00b7 ' + p.last));
      el.processBody.appendChild(last);
    }
    // A finished run offers the gate's next step: a review task lands in
    // the composer, and the owner still sends it. The UI never speaks
    // for the owner, and a batch judge with spend semantics is exactly a
    // task the owner should see before it runs.
    if (!p.active && p.stop !== 'failed' && p.gate && p.gate.informs_failover) {
      var review = node('button', 'replay-button process-review', 'ask sys1 for a review');
      review.type = 'button';
      review.dataset.act = 'action';
      review.title = 'fill the composer with a review task; nothing runs until you send it';
      review.addEventListener('click', function () {
        usageRecord('actions', 'process-review', 'uses');
        ask('Ask sys1 to review the last run: grade the evidence for what it claims, ' +
          'name the weakest step, and say what would change the verdict.');
      });
      el.processBody.appendChild(review);
    }
    paintProcessControls(p);
    paintProcessStrip(p);
  }

  // The live delegation branches, capped at the three most recent: the
  // full tree is the delegation panel, and the process card's job is
  // to show that the work fanned out and where each child stands.
  function processBranches(p) {
    var ids = p.branchOrder || [];
    if (!ids.length) return null;
    var wrap = node('div', 'process-branches');
    var head = node('div', 'process-branches-head');
    head.appendChild(node('span', null, 'branches'));
    head.appendChild(node('span', 'process-branches-count', String(ids.length)));
    wrap.appendChild(head);
    var shown = ids.slice(-3);
    shown.forEach(function (id) {
      var info = p.branches[id];
      if (!info) return;
      var line = node('button', 'process-branch');
      line.type = 'button';
      line.dataset.act = 'inspect';
      line.title = info.goal || id;
      line.dataset.state = info.stage === 'done'
        ? (info.outcome === 'completed' ? 'done' : 'down')
        : info.stage === 'working' ? 'current' : 'pending';
      line.appendChild(node('span', 'process-branch-dot'));
      line.appendChild(node('span', 'process-branch-id', shortSession(id)));
      line.appendChild(node('span', 'process-branch-note', branchNote(info)));
      line.addEventListener('click', function () { openBranchSession(id); });
      wrap.appendChild(line);
    });
    if (ids.length > shown.length) {
      var rest = node('button', 'process-branch-more',
        '+' + (ids.length - shown.length) + ' in the delegation panel');
      rest.type = 'button';
      rest.addEventListener('click', function () { window.location.hash = '#/delegation'; });
      wrap.appendChild(rest);
    }
    return wrap;
  }

  function shortSession(id) {
    var text = String(id || '');
    var hash = /Z-(.+)$/.exec(text);
    return hash ? hash[1].slice(0, 8) : text.slice(0, 8);
  }

  function branchNote(info) {
    if (info.stage === 'working') {
      return (info.calls || 0) + ' calls' + (info.tool ? ' \u00b7 ' + info.tool : '');
    }
    if (info.stage === 'done') {
      return (info.outcome || 'done') + (info.turns ? ' \u00b7 ' + info.turns + ' turns' : '');
    }
    return clip(info.goal || 'spawned', 42);
  }

  // A branch opens the child's own replay in the dock, over the live
  // conversation rather than instead of it: the owner keeps watching
  // the run and still reads the child's log.
  function openBranchSession(id) {
    fetch('/api/session?id=' + encodeURIComponent(id), {
      headers: { accept: 'application/json' }
    })
      .then(function (reply) { return reply.json(); })
      .then(function (doc) {
        if (doc && doc.refused && doc.refused.length) {
          notice('the branch could not be read', doc.refused[0]);
          return;
        }
        showSessionObject(doc, face.session);
      })
      .catch(function () {
        notice('the branch could not be read', 'the server did not answer');
      });
  }

  // The two controls are the only part of the process card the paint
  // loop never rebuilds, so a half-typed steer survives every event.
  function paintProcessControls(p) {
    if (!el.processControls) return;
    var show = !!p && p.active;
    el.processControls.hidden = !show;
    if (!show) {
      // A steer typed for a run that has ended is not a draft for the
      // next one: the next run has its own task and its own control.
      el.processSteer.value = '';
      return;
    }
    el.processStop.disabled = !!p.stopping;
    el.processStop.textContent = p.stopping ? 'stopping\u2026' : 'stop';
    el.processStop.dataset.state = p.stopping ? 'stopping' : '';
    el.processSteer.disabled = !!p.stopping;
  }

  function paintProcessStrip(p) {
    if (!el.processStrip) return;
    var open = document.body.dataset.aside !== 'closed';
    var narrow = window.innerWidth <= 1180;
    var show = !!p && (p.active || (p.mode === 'plan' && p.stage === 'review')) && (!open || narrow);
    el.processStrip.hidden = !show;
    if (!show) return;
    el.processStrip.textContent = '';
    var main = node('button', 'process-strip-main');
    main.type = 'button';
    main.title = 'open the run\u2019s process';
    var dot = node('span', 'process-strip-dot');
    dot.dataset.state = p.active ? 'running' : p.stop === 'failed' ? 'failed' : 'done';
    main.appendChild(dot);
    main.appendChild(node('span', 'process-strip-label',
      p.turn ? 'turn ' + p.turn : p.mode === 'plan' ? 'plan mode' : 'the run'));
    main.appendChild(node('span', 'process-strip-note',
      p.tool || (p.stage === 'review' ? 'the plan is ready'
        : p.last && p.last !== 'turn ' + p.turn ? p.last : '')));
    var dots = node('span', 'process-strip-dots');
    var steps = p.mode === 'plan' ? PLAN_STEPS : RUN_STEPS;
    var order = steps.map(function (step) { return step[0]; });
    var at = Math.max(0, order.indexOf(p.stage));
    steps.forEach(function (step, index) {
      var one = node('span');
      one.dataset.state = !p.active
        ? (p.stop === 'failed' && index === at ? 'down' : index <= at ? 'done' : 'pending')
        : index < at ? 'done' : index === at ? 'current' : 'pending';
      dots.appendChild(one);
    });
    main.appendChild(dots);
    main.addEventListener('click', openProcess);
    el.processStrip.appendChild(main);
  }

  function openProcess() {
    if (window.innerWidth > 1180) {
      setAside(true);
      return;
    }
    openProcessDrawer();
  }

  function openProcessDrawer() {
    var p = face.process;
    if (!p) {
      openDrawer('the run', [['process', node('p', 'derivation',
        'No run is in flight. Ask fleet for something and the loop lands here.')]]);
      return;
    }
    var sections = [
      ['where we are', processSteps(p)],
      ['the last step', node('p', 'derivation', p.last || 'no step yet')],
      ['the counts', node('p', 'derivation',
        'turn ' + p.turn + ' \u00b7 ' + p.calls + ' calls \u00b7 ' + p.facts +
        ' facts \u00b7 ' + p.delegations + ' delegations \u00b7 ' + p.lanes + ' lanes')]
    ];
    // Below the dock's breakpoint the drawer is the process card, so it
    // carries the same two controls. They post the same control file;
    // the strip above the composer shows where the run then stands.
    if (p.active) {
      var form = node('form', 'process-steer');
      var input = node('input');
      input.type = 'text';
      input.placeholder = 'steer the run\u2026';
      input.autocomplete = 'off';
      input.setAttribute('aria-label', 'steer this run');
      var send = node('button', 'ghost', 'steer');
      send.type = 'submit';
      form.appendChild(input);
      form.appendChild(send);
      form.addEventListener('submit', function (event) {
        event.preventDefault();
        var text = input.value.trim();
        if (!text) return;
        send.disabled = true;
        post('/api/steer', { text: text }).then(function (reply) {
          send.disabled = false;
          if (reply.status === 200 && face.process === p) {
            p.last = 'steer sent \u00b7 ' + clip(text, 50);
            openProcessDrawer();
            return;
          }
          notice('the steer was refused',
            (reply.body && reply.body.error) || 'the server did not take it');
        }).catch(function () {
          send.disabled = false;
          notice('the steer was refused', 'the server did not answer');
        });
      });
      var stop = node('button', 'process-stop', p.stopping ? 'stopping\u2026' : 'stop');
      stop.type = 'button';
      stop.addEventListener('click', function () {
        if (stop.disabled) return;
        stop.disabled = true;
        post('/api/stop', {}).then(function (reply) {
          if (reply.status === 200 && face.process === p) {
            p.stopping = true;
            p.last = 'stopping \u00b7 the run ends at its next step';
            openProcessDrawer();
            return;
          }
          stop.disabled = false;
          notice('the stop was refused',
            (reply.body && reply.body.error) || 'the server did not take it');
        }).catch(function () {
          stop.disabled = false;
          notice('the stop was refused', 'the server did not answer');
        });
      });
      var controls = node('div', 'process-controls');
      controls.appendChild(form);
      controls.appendChild(stop);
      sections.push(['stop or steer', controls]);
    }
    openDrawer('the run', sections);
  }

  function setAside(open) {
    document.body.dataset.aside = open ? 'open' : 'closed';
    // At the mid breakpoint the dock only has room as a column if the
    // nav folds. Opening it folds the nav and remembers what was there;
    // the fold is not written to storage because it is not the owner's
    // choice.
    if (open && window.innerWidth > 1180 && window.innerWidth < 1600 &&
        document.body.dataset.rail !== 'icons' && !face.railBeforeDock) {
      face.railBeforeDock = document.body.dataset.rail || 'full';
      applyRail('icons', false);
    }
    if (!open && face.railBeforeDock) {
      var restore = face.railBeforeDock;
      face.railBeforeDock = null;
      applyRail(restore, false);
    }
    if (el.asideToggle) {
      el.asideToggle.setAttribute('aria-pressed', open ? 'true' : 'false');
      el.asideToggle.setAttribute('aria-label',
        open ? 'undock the run\u2019s card' : 'dock the run\u2019s card');
      el.asideToggle.title = open ? 'undock the run\u2019s card' : 'dock the run\u2019s card';
    }
    try { window.localStorage.setItem('fleet-aside', open ? 'open' : 'closed'); } catch (error) { }
    syncDock();
  }

  // ------------------------------------------------------------- the home

  // The empty state is the face's welcome, so it speaks in the owner's own
  // time and names the place they are standing in. The words are the only
  // thing that changes; the structure stays still between visits.
  function greeting() {
    var hour = new Date().getHours();
    if (hour < 5) return 'working late';
    if (hour < 12) return 'good morning';
    if (hour < 18) return 'good afternoon';
    return 'good evening';
  }

  function workspaceName() {
    var path = String(face.workspace || '');
    if (!path) return 'this machine';
    var parts = path.split('/').filter(function (part) { return part.length; });
    return parts.length ? parts[parts.length - 1] : path;
  }

  function paintEmpty() {
    if (!el.emptyEyebrow) return;
    el.emptyEyebrow.textContent = greeting() + ' \u00b7 ' + workspaceName();
    if (el.emptyMeta) {
      el.emptyMeta.textContent = face.workspace
        ? 'Local to ' + face.workspace + ' \u00b7 nothing leaves this machine.'
        : 'Nothing leaves this machine. Every turn is a card you can open.';
    }
    if (el.railWhere) el.railWhere.textContent = workspaceName();
  }

  // The last few runs, one click from the welcome. A console that opens
  // blank after a night of scheduled work is a console that forgot the
  // owner was there.
  function paintRecent() {
    paintHome();
    var rows = (face.home && face.home.runs && face.home.runs.recent)
      || ((face.sessions && face.sessions.sessions) || []).slice(0, 3);
    if (!rows.length) { el.recent.hidden = true; return; }
    el.recent.hidden = false;
    el.recent.textContent = '';
    var head = node('div', 'recent-head');
    head.appendChild(node('h2', 'recent-title', 'pick up where you left off'));
    var all = node('button', 'recent-all', 'open the archive');
    all.type = 'button';
    all.addEventListener('click', function () { window.location.hash = '#/sessions'; });
    head.appendChild(all);
    el.recent.appendChild(head);
    rows.forEach(function (row) {
      var line = node('button', 'recent-row');
      line.type = 'button';
      var dot = node('span', 'recent-dot');
      dot.dataset.state = row.ended ? 'done' : 'live';
      line.appendChild(dot);
      var id = node('span', 'recent-id',
        (row.current ? 'this face \u00b7 ' : '') + sessionLabel(row.id));
      id.title = row.id;
      line.appendChild(id);
      var when = String(row.created || '');
      line.appendChild(node('span', 'recent-when',
        when ? when.slice(5, 16).replace('T', ' ') : '\u2014'));
      line.addEventListener('click', function () {
        el.recent.hidden = true;
        window.location.hash = '#/sessions';
        readReplay(row.id, true);
      });
      el.recent.appendChild(line);
    });
  }

  // The welcome's three readings: the same memory, skills and sessions
  // documents the panels read, reduced to what fits in a glance. They
  // answer "what does this harness know about me and my work" without a
  // second store and without inventing a number.
  function paintHome() {
    if (!el.homeStats) return;
    var h = face.home || {};
    var facts = h.memory ? h.memory.live
      : face.memory
        ? (face.memory.facts || []).filter(function (fact) { return !fact.retired; }).length
        : null;
    var skills = h.skills ? h.skills.count
      : face.skills ? (face.skills.skills || []).length : null;
    var runs = h.runs ? h.runs.count
      : face.sessions ? (face.sessions.sessions || []).length : null;
    if (facts === null && skills === null && runs === null) {
      el.homeStats.hidden = true;
      return;
    }
    el.homeStats.hidden = false;
    el.homeFacts.textContent = facts === null ? '\u2014' : String(facts);
    el.homeSkills.textContent = skills === null ? '\u2014' : String(skills);
    el.homeRuns.textContent = runs === null ? '\u2014' : String(runs);
    if (!el.homeStats.dataset.wired) {
      el.homeStats.dataset.wired = '1';
      Array.prototype.forEach.call(el.homeStats.querySelectorAll('[data-go]'), function (button) {
        button.addEventListener('click', function () {
          window.location.hash = button.dataset.go;
        });
      });
    }

    // The part a count cannot say: which facts are carrying the most
    // weight in this corpus. The home read names them; a panel read is
    // the fallback when the owner arrived before the home landed.
    if (el.homeKnown) {
      var named = [];
      if (h.memory && (h.memory.top || []).length) {
        named = h.memory.top.map(function (fact) { return { name: fact.name, recalls: fact.recalls }; });
      } else {
        named = (face.memory ? face.memory.facts || [] : [])
          .filter(function (fact) { return !fact.retired; })
          .sort(function (left, right) { return right.recall_count - left.recall_count; })
          .slice(0, 2);
      }
      el.homeKnown.hidden = !named.length;
      if (named.length) {
        el.homeKnown.textContent = '';
        el.homeKnown.appendChild(document.createTextNode('Most recalled: '));
        named.forEach(function (fact, at) {
          if (at) el.homeKnown.appendChild(document.createTextNode(' \u00b7 '));
          el.homeKnown.appendChild(node('b', null, fact.name));
        });
        el.homeKnown.onclick = function () { window.location.hash = '#/memory'; };
      }
    }

    // The method's live edge: the graph's next ready node, offered as the
    // first step of the day. The home read names it, so the welcome and
    // the graph cannot point at different steps.
    if (el.next) {
      var pick = h.next || null;
      if (!pick && face.delegation && face.delegation.graph) {
        var nodes = face.delegation.graph.nodes || [];
        var ready = nodes.filter(function (one) { return one.state === 'ready'; })
          .sort(function (left, right) { return left.score - right.score; });
        var running = nodes.filter(function (one) { return one.state === 'running'; });
        pick = running[0] || ready[0] || null;
      }
      el.next.hidden = !pick;
      if (pick) {
        el.nextEyebrow.textContent = pick.state === 'running' ? 'running now' : 'next from the graph';
        el.nextGoal.textContent = pick.id + (pick.approach ? ' \u00b7 ' + pick.approach : '');
        el.nextGo.onclick = function () {
          ask('work the next ready graph node: ' + pick.id +
            (pick.approach ? '. ' + pick.approach : ''));
        };
      }
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

  // The surface announces itself once, above the cards: an eyebrow that
  // names the group, a title in the display voice, one plain sentence
  // about what the panel reads, and the read's own stamp on the right.
  // It is deliberately not a card — ten surfaces should not all open with
  // the same framed heading.
  function pageHead(eyebrow, title, note, stamp, action) {
    var head = node('header', 'page-head');
    var main = node('div', 'page-main');
    main.appendChild(node('p', 'page-eyebrow', eyebrow));
    main.appendChild(node('h1', 'page-title', title));
    if (note) main.appendChild(node('p', 'page-note', note));
    head.appendChild(main);
    if (stamp || action) {
      var side = node('div', 'page-side');
      if (stamp) side.appendChild(node('span', 'page-stamp', stamp));
      if (action) {
        var button = node('button', 'page-action', action.label);
        button.type = 'button';
        button.dataset.act = 'action';
        button.addEventListener('click', function () {
          usageRecord('actions', 'page:' + action.label, 'uses');
          action.run();
        });
        side.appendChild(button);
      }
      head.appendChild(side);
    }
    return head;
  }

  // The KPI strip is the `.tiles` node the panel already built; the
  // class is kept so the markup stays one shape whether it is a page's
  // first row or a card's interior.
  function statStrip(tiles) {
    if (tiles && !tiles.classList.contains('stats-strip')) tiles.classList.add('stats-strip');
    return tiles;
  }

  function tile(value, label, tone, run) {
    var el0 = run ? node('button', 'tile') : node('div', 'tile');
    el0.dataset.act = run ? 'filter' : 'inspect';
    if (run) {
      el0.type = 'button';
      el0.title = label + ': open the list it counts';
      el0.addEventListener('click', run);
    } else {
      el0.tabIndex = 0;
      el0.setAttribute('role', 'button');
      el0.title = label + ': what this counts and where it comes from';
      el0.addEventListener('click', function () { metricDrawer(value, label); });
      el0.addEventListener('keydown', function (event) {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault();
          metricDrawer(value, label);
        }
      });
    }
    if (tone) el0.dataset.tone = tone;
    el0.appendChild(node('div', 'tile-value', String(value)));
    el0.appendChild(node('div', 'tile-label', label));
    return el0;
  }

  // A count is an object too. Its drawer says what it counts and takes the
  // owner to the list it belongs to, so no number is a dead end.
  function metricDrawer(value, label) {
    var go = node('button', 'replay-button replay-resume');
    go.type = 'button';
    go.textContent = 'open the list below';
    go.addEventListener('click', function () {
      closeDrawer();
      var card = document.querySelector('.stage:not([hidden]) .board-card');
      if (card) card.scrollIntoView({ behavior: 'smooth', block: 'start' });
    });
    var actions = node('div', 'replay-actions');
    actions.appendChild(go);
    openDrawer('metric \u00b7 ' + label, [
      ['what it counts', node('p', 'derivation', String(value) + ' ' + label)],
      ['where it comes from', node('p', 'derivation',
        'the read that feeds this panel. The number is the read\u2019s own; the panel does not ' +
        're-derive it. A count with no list is a count the code did not carry.')],
      ['what you can do', actions]
    ]);
  }

  // A panel's filter chips. One shape for memory, skills, calibration,
  // and the tower, so a filter reads the same wherever it is: the value
  // it sets, the word it shows, and its count. `current` is the live
  // value; the chosen chip is the only one pressed.
  function filterBar(items, current, onPick) {
    var bar = node('div', 'filter-bar');
    items.forEach(function (item) {
      var label = item.label + (item.count == null ? '' : ' ' + item.count);
      var chip = node('button', 'filter-chip', label);
      chip.type = 'button';
      chip.dataset.act = 'filter';
      chip.setAttribute('aria-pressed', current === item.value ? 'true' : 'false');
      chip.addEventListener('click', function () { onPick(item.value); });
      bar.appendChild(chip);
    });
    return bar;
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
  function dayChart(hours, wide, onHour) {
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
          rx: 2, class: 'chart-col', 'data-act': 'filter-hour' }));
      }
      if (bad > 0) {
        plot.appendChild(svg('rect', { x: x, y: base - full, width: width, height: Math.max(1, bad),
          rx: 2, class: 'chart-col', 'data-tone': 'down', 'data-act': 'filter-hour' }));
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
      if (onHour) {
        var hit = svg('rect', {
          x: x - 2, y: top - 8, width: width + 4, height: base - top + 20,
          fill: 'transparent', class: 'chart-hit', 'data-act': 'filter-hour',
          role: 'button', tabindex: '0'
        });
        var title = svg('title');
        title.textContent = String(column.hour).padStart(2, '0') + ':00 \u00b7 ' +
          column.calls + ' calls \u00b7 ' + column.errors + ' failed';
        hit.appendChild(title);
        hit.addEventListener('click', function () { onHour(column.hour); });
        hit.addEventListener('keydown', function (event) {
          if (event.key === 'Enter' || event.key === ' ') {
            event.preventDefault();
            onHour(column.hour);
          }
        });
        plot.appendChild(hit);
      }
    });
    wrap.appendChild(plot);
    return wrap;
  }

  // The model-mix donut. Each segment is a dash of one circumference, so
  // a model that took one call draws a hairline and one that took a third
  // of the day draws an arc, and no path arithmetic is involved. The total
  // is the day's calls, not its serves: the mix says where calls landed,
  // and the ledger beside it already says how many served.
  function donut(models, onModel) {
    var wrap = node('div', 'donut');
    var R = 34, C = 2 * Math.PI * R;
    var total = models.reduce(function (sum, entry) { return sum + entry.calls; }, 0);
    var plot = svgPlot(92, 92, 'the day\u2019s calls by model');
    plot.appendChild(svg('circle', { cx: 46, cy: 46, r: R, class: 'donut-track', 'stroke-width': 13 }));
    var run = 0;
    models.forEach(function (entry, index) {
      var share = total ? entry.calls / total : 0;
      if (share <= 0) return;
      var segment = svg('circle', {
        cx: 46, cy: 46, r: R, class: 'donut-seg', 'stroke-width': 13,
        stroke: MIX_COLORS[index % MIX_COLORS.length],
        'stroke-dasharray': (share * C).toFixed(2) + ' ' + (C - share * C).toFixed(2),
        'stroke-dashoffset': (-run * C).toFixed(2),
        transform: 'rotate(-90 46 46)'
      });
      var title = svg('title');
      title.textContent = entry.model + ' \u00b7 ' + entry.calls + ' calls \u00b7 ' +
        Math.round(share * 100) + '%';
      segment.appendChild(title);
      if (onModel) {
        segment.setAttribute('data-act', 'filter-model');
        segment.setAttribute('role', 'button');
        segment.setAttribute('tabindex', '0');
        segment.addEventListener('click', function () { onModel(entry.model); });
        segment.addEventListener('keydown', function (event) {
          if (event.key === 'Enter' || event.key === ' ') {
            event.preventDefault();
            onModel(entry.model);
          }
        });
      }
      plot.appendChild(segment);
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
      if (onModel) {
        item.dataset.act = 'filter-model';
        item.tabIndex = 0;
        item.setAttribute('role', 'button');
        item.title = 'filter the rows to ' + entry.model;
        item.addEventListener('click', function () { onModel(entry.model); });
        item.addEventListener('keydown', function (event) {
          if (event.key === 'Enter' || event.key === ' ') {
            event.preventDefault();
            onModel(entry.model);
          }
        });
      }
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
        chip.title = index === 0
          ? 'what a route selects for this workload \u00b7 click to copy'
          : 'the walk\u2019s next candidate \u00b7 click to copy';
        chip.tabIndex = 0;
        chip.setAttribute('role', 'button');
        chip.addEventListener('click', function () {
          if (navigator.clipboard) navigator.clipboard.writeText(candidate).catch(function () { });
          chip.textContent = 'copied';
          window.setTimeout(function () { chip.textContent = candidate; }, 1000);
        });
        chip.addEventListener('keydown', function (event) {
          if (event.key === 'Enter' || event.key === ' ') {
            event.preventDefault();
            chip.click();
          }
        });
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
      el0.appendChild(dayChart(read.hours, wide, function (hour) {
        face.routerHour = face.routerHour === hour ? null : hour;
        paintRouter();
      }));
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
    el0.appendChild(donut(read.models, function (model) {
      face.routerModel = face.routerModel === model ? '' : model;
      paintRouter();
    }));
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
    var hour = face.routerHour;
    var model = face.routerModel || '';
    var shown = read.recent.filter(function (entry) {
      if (hour != null && Number(String(entry.at).slice(11, 13)) !== hour) return false;
      if (model && entry.model !== model) return false;
      return true;
    });
    // The chart's own selection, named and removable: a filtered list
    // with no visible cause is a list the owner cannot trust.
    if (hour != null || model) {
      var bar = node('div', 'filter-bar');
      if (hour != null) {
        var hourChip = node('button', 'filter-chip filter-chip-active',
          String(hour).padStart(2, '0') + ':00 \u00d7');
        hourChip.type = 'button';
        hourChip.dataset.act = 'clear-filter';
        hourChip.title = 'clear the hour filter';
        hourChip.addEventListener('click', function () {
          face.routerHour = null;
          paintRouter();
        });
        bar.appendChild(hourChip);
      }
      if (model) {
        var modelChip = node('button', 'filter-chip filter-chip-active', model + ' \u00d7');
        modelChip.type = 'button';
        modelChip.dataset.act = 'clear-filter';
        modelChip.title = 'clear the model filter';
        modelChip.addEventListener('click', function () {
          face.routerModel = '';
          paintRouter();
        });
        bar.appendChild(modelChip);
      }
      bar.appendChild(node('span', 'filter-count',
        shown.length + ' of ' + read.recent.length + ' rows'));
      el0.appendChild(bar);
    }
    if (!shown.length) {
      el0.appendChild(node('p', 'board-empty',
        'No route row today matches this filter. Clear it to see the rows.'));
      return el0;
    }
    var rows = node('div', 'rows');
    shown.forEach(function (entry) {
      var line = node('div', 'rrow');
      line.dataset.act = 'inspect';
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
      line.tabIndex = 0;
      line.setAttribute('role', 'button');
      line.title = 'open this call: ' + entry.model + ' \u00b7 ' + entry.outcome;
      line.addEventListener('click', function () { routeDrawer(entry); });
      line.addEventListener('keydown', function (event) {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault();
          routeDrawer(entry);
        }
      });
      rows.appendChild(line);
    });
    el0.appendChild(rows);
    return el0;
  }

  // One route row's detail: the call, the walk it took, and the two
  // actions that make the row useful — copy the model, or hand the
  // decision to the judge lane for a second opinion.
  function routeDrawer(entry) {
    var walk = (entry.providers_used && entry.providers_used.length
      ? entry.providers_used : [entry.provider]).join(' \u2192 ');
    var actions = node('div', 'replay-actions');
    var copy = node('button', 'replay-button');
    copy.type = 'button';
    copy.textContent = 'copy the model';
    copy.addEventListener('click', function () {
      if (navigator.clipboard) navigator.clipboard.writeText(entry.model).catch(function () { });
      copy.textContent = 'copied';
      window.setTimeout(function () { copy.textContent = 'copy the model'; }, 1200);
    });
    actions.appendChild(copy);
    var judge = node('button', 'replay-button replay-resume');
    judge.type = 'button';
    judge.textContent = 'ask sys1 about this call';
    judge.addEventListener('click', function () {
      closeDrawer();
      ask('use the sys1 lane to review the route call to ' + entry.model +
        (entry.workload ? ' for workload ' + entry.workload : '') +
        ' and say whether the walk was right');
    });
    actions.appendChild(judge);
    openDrawer('route \u00b7 ' + entry.model, [
      ['the call', node('p', 'derivation',
        clock(entry.at) + ' \u00b7 ' + (entry.workload || 'no workload') +
        ' \u00b7 ' + entry.outcome +
        (entry.ms == null ? '' : ' \u00b7 ' + entry.ms + ' ms') +
        (entry.reason ? ' \u00b7 ' + entry.reason : ''))],
      ['the walk', node('p', 'derivation', walk)],
      ['related', relatedFor('route', entry.at)],
      ['what you can do', actions]
    ]);
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
      el.routerBoard.appendChild(pageHead('Intelligence', 'Router',
        'Where this run\u2019s work went, what it tried first, and what the day has served.',
        null));
      var waiting = boardCard(12, 'reading', null, null);
      waiting.appendChild(node('p', 'board-empty',
        'The board\u2019s read has not landed yet. The live decision below is the run\u2019s own event ' +
        'and does not wait for it.'));
      el.routerBoard.appendChild(waiting);
      el.routerBoard.appendChild(decisionCard());
      return;
    }
    el.routerBoard.appendChild(pageHead('Intelligence', 'Router',
      'Where this run\u2019s work went, what it tried first, and what the day has served.',
      read.read_at, {
        label: 'Ask sys1 for a judgment',
        run: function () {
          ask('use the sys1 lane to review today\u2019s route decisions and report the fit');
        }
      }));
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
    made.setAttribute('tabindex', '0');
    made.setAttribute('role', 'button');
    made.addEventListener('click', function () { nodeDrawer(one); });
    made.addEventListener('keydown', function (event) {
      if (event.key === 'Enter' || event.key === ' ') {
        event.preventDefault();
        nodeDrawer(one);
      }
    });
    return made;
  }

  // One graph node's detail: the state, the score it carries, the three
  // unlock conditions, and the one action that turns the model into
  // work — hand the node to the composer.
  function nodeDrawer(one) {
    var unlock = one.unlock || {};
    var actions = node('div', 'replay-actions');
    var take = node('button', 'replay-button replay-resume');
    take.type = 'button';
    take.textContent = 'take this step';
    take.addEventListener('click', function () {
      closeDrawer();
      ask('work the next ready graph node: ' + one.id + (one.approach ? '. ' + one.approach : ''));
    });
    actions.appendChild(take);
    var copy = node('button', 'replay-button');
    copy.type = 'button';
    copy.textContent = 'copy the node id';
    copy.addEventListener('click', function () {
      if (navigator.clipboard) navigator.clipboard.writeText(one.id).catch(function () { });
      copy.textContent = 'copied';
      window.setTimeout(function () { copy.textContent = 'copy the node id'; }, 1200);
    });
    actions.appendChild(copy);
    openDrawer('graph \u00b7 ' + one.id, [
      ['the node', node('p', 'derivation',
        one.state + ' \u00b7 score ' + one.score + ' of ' + one.peak +
        ' \u00b7 cost ' + one.cost +
        (one.approach ? ' \u00b7 ' + one.approach : ''))],
      ['the unlock conditions', node('p', 'derivation',
        'score zero: ' + (unlock.score_zero ? 'met' : 'not met') +
        ' \u00b7 resource free: ' + (unlock.resource_free ? 'met' : 'not met') +
        ' \u00b7 deps verified: ' + (unlock.deps_verified ? 'met' : 'not met'))],
      ['what you can do', actions]
    ]);
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
      el.delegationBoard.appendChild(pageHead('Intelligence', 'Delegation',
        'The run\u2019s own tree, the runs before it, and the dependency graph the plan declares.',
        null));
      var waiting = boardCard(12, 'reading', null, null);
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
    el.delegationBoard.appendChild(pageHead('Intelligence', 'Delegation',
      'The run\u2019s own tree, the runs before it, and the dependency graph the plan declares.',
      null, {
        label: 'Ask for the next step',
        run: function () {
          ask('read the dependency graph and tell me the next ready node to work');
        }
      }));
    el.delegationBoard.appendChild(statStrip(tiles));

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
      el.memoryBoard.appendChild(pageHead('Knowledge', 'Memory',
        'One fact per file: what this run recalled, and what the corpus already knows.',
        null));
      var waiting = boardCard(12, 'reading', null, null);
      waiting.appendChild(node('p', 'board-empty',
        'The corpus read has not landed yet. It reads the memories tree and the day\u2019s ' +
        'calibration stream, both of which are this machine\u2019s own.'));
      el.memoryBoard.appendChild(waiting);
      return;
    }
    var facts = read.facts || [];
    var live = facts.filter(function (fact) { return !fact.retired; });
    var protectedFacts = live.filter(function (fact) { return fact.protected; });
    var retired = facts.filter(function (fact) { return fact.retired; });
    var hits = read.hits || [];
    var recalls = face.events.filter(function (event) { return event.type === 'memory'; });
    var filter = face.memoryFilter || 'live';
    var tiles = node('div', 'tiles');
    tiles.appendChild(tile(live.length, 'live facts', null, function () {
      face.memoryFilter = 'live';
      paintMemory();
    }));
    tiles.appendChild(tile(protectedFacts.length, 'protected', null, function () {
      face.memoryFilter = 'protected';
      paintMemory();
    }));
    tiles.appendChild(tile(retired.length, 'retired', null, function () {
      face.memoryFilter = 'retired';
      paintMemory();
    }));
    tiles.appendChild(tile(hits.length, 'hits ' + read.day));
    tiles.appendChild(tile(recalls.length, 'recalls this run', recalls.length ? 'live' : null));
    el.memoryBoard.appendChild(pageHead('Knowledge', 'Memory',
      'One fact per file: what this run recalled, and what the corpus already knows.',
      'the corpus at ' + read.root, {
        label: 'Ask what fleet knows',
        run: function () {
          ask('summarise what the memories know about my current work and cite the facts');
        }
      }));
    el.memoryBoard.appendChild(statStrip(tiles));
    el.memoryBoard.appendChild(memorySearch());
    el.memoryBoard.appendChild(filterBar([
      { value: 'all', label: 'all', count: facts.length },
      { value: 'live', label: 'live', count: live.length },
      { value: 'protected', label: 'protected', count: protectedFacts.length },
      { value: 'retired', label: 'retired', count: retired.length }
    ], filter, function (value) {
      face.memoryFilter = value;
      paintMemory();
    }));

    var shown = facts.filter(function (fact) {
      return memoryFilterMatch(fact, filter) && matchesQuery(fact);
    });
    if (shown.length) {
      var corpus = boardCard(12, 'the corpus',
        shown.length + (filter === 'all' ? ' of ' + facts.length : ' ' + filter),
        'a fact is one file under the memories tree; opening one shows its whole body and its ' +
        'edit, written through the substrate\u2019s own rewriter.');
      corpus.appendChild(factGrid(shown));
      el.memoryBoard.appendChild(corpus);
    } else {
      var none = boardCard(12, 'the corpus', null, null);
      none.appendChild(node('p', 'board-empty',
        facts.length
          ? 'No ' + (filter === 'all' ? '' : filter + ' ') + 'fact matches' +
            (face.query ? ' \u201c' + face.query + '\u201d' : ' this filter') +
            '. The search reads the name, the sentence, the type, the shape tags and the body ' +
            '\u2014 every field the fact carries.'
          : 'This home holds no facts. A fact is one file under the memories tree, and the ' +
            'agent writes them through the substrate\u2019s own writer.'));
      el.memoryBoard.appendChild(none);
    }

    el.memoryBoard.appendChild(recallCard(recalls, hits, read.day));
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

  // The corpus's tier filter. A protected fact is always live by the
  // corpus's own rule, so the two chips agree rather than double-count.
  function memoryFilterMatch(fact, filter) {
    switch (filter) {
      case 'live': return !fact.retired;
      case 'protected': return !!fact.protected && !fact.retired;
      case 'retired': return !!fact.retired;
      default: return true;
    }
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
      card0.dataset.act = 'inspect';
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
      Array.prototype.forEach.call(badges.children, function (chip) {
        chip.classList.add('badge-filter');
        chip.tabIndex = 0;
        chip.setAttribute('role', 'button');
        chip.title = 'filter the corpus by ' + chip.textContent;
        var filter = function (event) {
          event.stopPropagation();
          face.query = chip.textContent;
          paintMemory();
        };
        chip.addEventListener('click', filter);
        chip.addEventListener('keydown', function (event) {
          if (event.key === 'Enter' || event.key === ' ') {
            event.preventDefault();
            filter(event);
          }
        });
      });
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
    el0.title = String(text);
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
        row0.dataset.act = 'inspect';
        row0.tabIndex = 0;
        row0.setAttribute('role', 'button');
        row0.title = 'open this recall\u2019s record';
        row0.appendChild(node('span', 'rrow-at', clock(hit.ts)));
        row0.appendChild(node('span', 'rrow-what',
          (hit.shape ? 'shape ' + hit.shape + ' \u00b7 ' : '') + hit.fact));
        var side = node('span', 'rrow-side');
        side.appendChild(node('span', 'rrow-outcome', hit.score.toFixed(2)));
        row0.appendChild(side);
        row0.addEventListener('click', function () {
          openDrawer('recall \u00b7 ' + hit.fact, [
            ['the record', node('pre', 'raw', JSON.stringify(hit, null, 2))]
          ]);
        });
        row0.addEventListener('keydown', function (event) {
          if (event.key === 'Enter' || event.key === ' ') {
            event.preventDefault();
            row0.click();
          }
        });
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
    var related = node('div', 'replay-actions');
    var askBtn = node('button', 'replay-button replay-resume');
    askBtn.type = 'button';
    askBtn.textContent = 'ask fleet about this fact';
    askBtn.addEventListener('click', function () {
      closeDrawer();
      ask('what do the memories say about ' + fact.name + '? cite the fact and its evidence.');
    });
    related.appendChild(askBtn);
    var copy = node('button', 'replay-button');
    copy.type = 'button';
    copy.textContent = 'copy the fact name';
    copy.addEventListener('click', function () {
      if (navigator.clipboard) navigator.clipboard.writeText(fact.name).catch(function () { });
      copy.textContent = 'copied';
      window.setTimeout(function () { copy.textContent = 'copy the fact name'; }, 1200);
    });
    related.appendChild(copy);
    if ((fact.shape || []).length) {
      var filter = node('button', 'replay-button');
      filter.type = 'button';
      filter.textContent = 'filter the corpus by ' + fact.shape[0];
      filter.addEventListener('click', function () {
        face.query = fact.shape[0];
        closeDrawer();
        paintMemory();
      });
      related.appendChild(filter);
    }
    openDrawer(fact.name, [
      ['the fact', node('p', 'derivation',
        'type ' + fact.fact_type + ' \u00b7 tier ' + fact.tier + ' \u00b7 basis ' + fact.basis +
        ' \u00b7 ' + fact.recall_count + ' recalls \u00b7 ' + fact.path)],
      ['related', relatedFor('fact', fact.name)],
      ['edit in place', form],
      ['what you can do', related]
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

  // The welcome's one read. The panels keep their own; this document is
  // the bounded summary the home renders, cached server-side for half a
  // minute so a reload or a return to the conversation does not walk the
  // corpus and every log again.
  function readHome(force) {
    var now = Date.now();
    if (!force && face.home && now - face.homeAt < 30000) return Promise.resolve();
    face.homeAt = now;
    return fetch('/api/home', { headers: { accept: 'application/json' } })
      .then(function (reply) {
        if (reply.status === 401) throw new Error('token');
        return reply.json();
      })
      .then(function (doc) {
        face.home = doc;
        paintHome();
        paintRecent();
        paintSince();
      })
      .catch(function () { });
  }

  // The usage counters: one read when the page boots and one after
  // every record, so the ranking state is the store's own. Counters
  // only — the ids are the labels the pages already show.
  function readUsage() {
    return fetch('/api/usage', { headers: { accept: 'application/json' } })
      .then(function (reply) {
        if (reply.status !== 200) throw new Error('usage');
        return reply.json();
      })
      .then(function (doc) {
        face.usage = doc;
      })
      .catch(function () { });
  }

  function usageRecord(section, id, event) {
    post('/api/usage', { section: section, id: id, event: event })
      .then(function (reply) {
        if (reply.status === 200 && reply.body && reply.body.type === 'usage') {
          face.usage = reply.body;
        }
      })
      .catch(function () { });
  }

  // Accepted-over-shown is the signal, capped at a third: a familiar
  // offer rises, but never enough to bury the primary lanes. With the
  // knob off the multiplier is exactly 1.
  function usageBoost(kind, id) {
    if (!face.usage || !face.usage.adapt) return 1;
    var row = (face.usage.suggestions || {})[kind + ':' + id];
    var shown = row ? Number(row.shown) || 0 : 0;
    var accepted = row ? Number(row.accepted) || 0 : 0;
    if (!shown || !accepted) return 1;
    return 1 + Math.min(0.35, (accepted / shown) * 0.35);
  }

  // What changed while the owner was away. The stamp is the browser's
  // own; the counts are the archive's and the corpus's, so the sentence
  // names real work rather than a streak.
  function paintSince() {
    var el0 = document.getElementById('since');
    if (!el0) return;
    var since = face.lastVisit;
    var home = face.home;
    if (!since || !home) { el0.hidden = true; return; }
    var at = Date.parse(since);
    if (isNaN(at)) { el0.hidden = true; return; }
    var runs = ((home.runs && home.runs.timeline) || []).filter(function (row) {
      return row.created && Date.parse(row.created) > at;
    }).length;
    var day = since.slice(0, 10);
    var facts = ((home.memory && home.memory.dates) || []).filter(function (date) {
      return date && date > day;
    }).length;
    if (!runs && !facts) { el0.hidden = true; return; }
    var parts = [];
    if (runs) parts.push(runs + (runs === 1 ? ' run' : ' runs'));
    if (facts) parts.push(facts + (facts === 1 ? ' new fact' : ' new facts'));
    el0.hidden = false;
    el0.textContent = 'Since your last visit: ' + parts.join(' \u00b7 ');
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
      el.skillsBoard.appendChild(pageHead('Knowledge', 'Skills',
        'The library the model can reach, the names the loader declined, and the proposals waiting for a decision.',
        null));
      var waiting = boardCard(12, 'reading', null, null);
      waiting.appendChild(node('p', 'board-empty',
        'The library read has not landed yet. It reads the canonical skills directory and the ' +
        'home\u2019s own proposals, both of which are this machine\u2019s.'));
      el.skillsBoard.appendChild(waiting);
      return;
    }
    var skills = read.skills || [];
    var skips = read.skips || [];
    var proposals = read.proposals || [];
    var used = skills.filter(function (skill) { return skill.uses > 0; }).length;
    var filter = face.skillsFilter || 'all';
    var tiles = node('div', 'tiles');
    tiles.appendChild(tile(skills.length, 'loaded', null, function () {
      face.skillsFilter = 'all';
      paintSkills();
    }));
    tiles.appendChild(tile(skips.length, 'skipped', skips.length ? 'warn' : null, function () {
      face.skillsFilter = 'skipped';
      paintSkills();
    }));
    tiles.appendChild(tile(proposals.length, 'awaiting a decision', proposals.length ? 'live' : null,
      function () {
        face.skillsFilter = 'proposals';
        paintSkills();
      }));
    tiles.appendChild(tile(read.unused, 'never selected', null, function () {
      face.skillsFilter = 'unused';
      paintSkills();
    }));
    tiles.appendChild(tile(read.sessions, 'sessions scanned'));
    el.skillsBoard.appendChild(pageHead('Knowledge', 'Skills',
      'The library the model can reach, the names the loader declined, and the proposals waiting for a decision.',
      'the library at ' + (read.canonical || '\u2014'), {
        label: 'Ask the curator',
        run: function () {
          ask('review the skill library and propose the next curation change');
        }
      }));
    el.skillsBoard.appendChild(statStrip(tiles));
    el.skillsBoard.appendChild(filterBar([
      { value: 'all', label: 'all', count: skills.length },
      { value: 'used', label: 'used', count: used },
      { value: 'unused', label: 'unused', count: skills.length - used },
      { value: 'proposals', label: 'proposals', count: proposals.length },
      { value: 'skipped', label: 'skipped', count: skips.length }
    ], filter, function (value) {
      face.skillsFilter = value;
      paintSkills();
    }));

    var showProposals = filter === 'all' || filter === 'proposals';
    if (showProposals) {
      if (proposals.length) {
        el.skillsBoard.appendChild(proposalCards(proposals));
      } else {
        var none = boardCard(12, 'the decisions queue', null, null);
        none.appendChild(node('p', 'board-empty',
          'No proposal is awaiting a decision. A proposal is a drafted change under the home\u2019s own ' +
          'proposals directory, and an approve here rides the same apply the CLI rides.'));
        el.skillsBoard.appendChild(none);
      }
    }

    if (filter === 'all' || filter === 'used' || filter === 'unused') {
      var shown = skills.filter(function (skill) {
        if (filter === 'used') return skill.uses > 0;
        if (filter === 'unused') return !skill.uses;
        return true;
      });
      // An empty filter is a state, not a missing card: the chip said
      // there were none, and the card confirms it in the list's place.
      var library = boardCard(12, filter === 'all' ? 'the library' : 'the library \u00b7 ' + filter,
        shown.length + (filter === 'all' ? '' : ' of ' + skills.length), null);
      if (shown.length) {
        library.appendChild(skillGrid(shown));
      } else {
        library.appendChild(node('p', 'board-empty',
          'No skill in the library is ' + (filter === 'used' ? 'used' : 'unused') + ' today.'));
      }
      el.skillsBoard.appendChild(library);
    }
    if (skips.length && (filter === 'all' || filter === 'skipped')) {
      el.skillsBoard.appendChild(skipCard(skips));
    }
    if (filter === 'all' || filter === 'proposals') {
      el.skillsBoard.appendChild(curationCard(read));
    }
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
      item.dataset.act = 'inspect';
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
      item.dataset.act = 'inspect';
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
      row0.dataset.act = 'inspect';
      row0.tabIndex = 0;
      row0.setAttribute('role', 'button');
      row0.title = skip.reason;
      row0.appendChild(node('span', 'rrow-what', skip.name));
      var side = node('span', 'rrow-side');
      side.appendChild(node('span', 'rrow-outcome', clip(skip.reason, 90)));
      row0.appendChild(side);
      row0.addEventListener('click', function () {
        openDrawer('skip \u00b7 ' + skip.name, [
          ['why the loader declined it', node('p', 'derivation', skip.reason)]
        ]);
      });
      row0.addEventListener('keydown', function (event) {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault();
          row0.click();
        }
      });
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
      row0.dataset.act = 'inspect';
      row0.tabIndex = 0;
      row0.setAttribute('role', 'button');
      row0.title = 'similarity ' + merge.similarity.toFixed(2) + ' against the threshold ' +
        read.threshold;
      row0.appendChild(node('span', 'rrow-what',
        merge.survivor + ' would absorb ' + merge.absorbed));
      var side = node('span', 'rrow-side');
      side.appendChild(node('span', 'rrow-outcome', merge.similarity.toFixed(2)));
      row0.appendChild(side);
      row0.addEventListener('click', function () {
        openDrawer('merge candidate', [
          ['the pair', node('p', 'derivation',
            merge.survivor + ' would absorb ' + merge.absorbed)],
          ['the measure', node('p', 'derivation',
            'description similarity ' + merge.similarity.toFixed(2) +
            ' \u00b7 the threshold is ' + read.threshold)]
        ]);
      });
      row0.addEventListener('keydown', function (event) {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault();
          row0.click();
        }
      });
      rows.appendChild(row0);
    });
    card0.appendChild(rows);
    return card0;
  }

  function inspectSkill(skill) {
    var actions = node('div', 'replay-actions');
    var copy = node('button', 'replay-button');
    copy.type = 'button';
    copy.textContent = 'copy the skill name';
    copy.addEventListener('click', function () {
      if (navigator.clipboard) navigator.clipboard.writeText(skill.name).catch(function () { });
      copy.textContent = 'copied';
      window.setTimeout(function () { copy.textContent = 'copy the skill name'; }, 1200);
    });
    actions.appendChild(copy);
    var curator = node('button', 'replay-button replay-resume');
    curator.type = 'button';
    curator.textContent = 'ask the curator about it';
    curator.addEventListener('click', function () {
      closeDrawer();
      ask('review the ' + skill.name + ' skill against the sessions that used it and say whether it should change');
    });
    actions.appendChild(curator);
    openDrawer(skill.name, [
      ['the skill', node('p', 'derivation',
        skill.uses + ' uses across ' + (face.skills ? face.skills.sessions : 0) + ' scanned sessions' +
        ' \u00b7 ' + skill.path)],
      ['related', relatedFor('skill', skill.name)],
      ['the body', node('pre', 'drawer-code', skill.body || '')],
      ['what the model sees', node('p', 'derivation',
        'the system prompt carries the name and the one sentence. The body enters context only when ' +
        'the skill is selected, and then it is already in the log as that call\u2019s result.')],
      ['what you can do', actions]
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

  // One way for any surface to hand the agent a task: open the
  // conversation, put the instruction in the composer, and leave the
  // send to the owner. The UI never speaks for the owner.
  function ask(text) {
    if (window.location.hash !== '#/chat') window.location.hash = '#/chat';
    el.task.value = text;
    try { window.localStorage.setItem('fleet-draft', text); } catch (error) { }
    grow();
    el.task.focus();
    el.task.setSelectionRange(text.length, text.length);
  }

  function send() {
    if (face.planMode) { planSend(); return; }
    var task = el.task.value.trim();
    if (!task || face.running || face.seat) return;
    el.task.value = '';
    grow();
    try { window.localStorage.removeItem('fleet-draft'); } catch (error) { }
    el.empty.hidden = true;
    el.log.hidden = false;
    var before = face.events.length;
    post('/api/run', { task: task }).then(function (reply) {
      if (reply.status === 200) {
        face.running = true;
        face.stop = null;
        resetProcess(face.planMode ? 'plan' : 'run');
        paint();
        return;
      }
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

  // The run's two controls. The server writes the control file the turn
  // loop reads before its next model call, so the latency is one step
  // and the card says "stopping" until the run's own end record lands.
  function stopRun() {
    if (!face.process || !face.process.active || face.process.stopping) return;
    // The reply races the run's own end record: a stop can be accepted
    // just as the answer lands. The card belongs to this run only, so a
    // late reply never overwrites the run that already ended.
    var p = face.process;
    el.processStop.disabled = true;
    post('/api/stop', {}).then(function (reply) {
      if (reply.status === 200) {
        if (face.process === p && p.active) {
          p.stopping = true;
          p.last = 'stopping \u00b7 the run ends at its next step';
          paintProcess();
        }
        return;
      }
      if (face.process === p && p.active) {
        notice('the stop was refused',
          (reply.body && reply.body.error) || 'the server did not take it');
        paintProcessControls(p);
      }
    }).catch(function () {
      if (face.process === p && p.active) {
        notice('the stop was refused', 'the server did not answer');
        paintProcessControls(p);
      }
    });
  }

  function steerRun(event) {
    event.preventDefault();
    var text = el.processSteer.value.trim();
    if (!text || !face.process || !face.process.active) return;
    var p = face.process;
    el.processSteer.disabled = true;
    post('/api/steer', { text: text }).then(function (reply) {
      if (face.process === p) el.processSteer.disabled = false;
      if (reply.status === 200) {
        if (face.process === p && p.active) {
          el.processSteer.value = '';
          p.last = 'steer sent \u00b7 ' + clip(text, 50);
          paintProcess();
        }
        return;
      }
      if (face.process === p && p.active) {
        notice('the steer was refused',
          (reply.body && reply.body.error) || 'the server did not take it');
      }
    }).catch(function () {
      if (face.process === p) el.processSteer.disabled = false;
      if (face.process === p && p.active) {
        notice('the steer was refused', 'the server did not answer');
      }
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

  // ---------------------------------------------------------- related objects

  // Suggestions are one server-side read: the related index compares
  // the selected object against every other kind by the same embedder
  // the recall uses, and names its own state (warm, warming, lexical).
  // The browser no longer shortlists or embeds anything — a second
  // ranking path would be a second answer to what "related" means.
  function relatedFor(kind, id) {
    var wrap = node('div', 'related');
    var key = kind + ':' + id;
    var cached = face.related[key];
    if (cached && cached.doc.index !== 'warming' && Date.now() - cached.at < 60000) {
      paintRelated(wrap, cached.doc);
      return wrap;
    }
    var wait = node('p', 'related-wait', 'reading the index\u2026');
    wrap.appendChild(wait);
    var retried = false;

    function load() {
      fetch('/api/related?kind=' + encodeURIComponent(kind) +
        '&id=' + encodeURIComponent(id) + '&limit=3', {
        headers: { accept: 'application/json' }
      })
        .then(function (reply) {
          if (reply.status === 401) throw new Error('token');
          if (reply.status !== 200) throw new Error('status ' + reply.status);
          return reply.json();
        })
        .then(function (doc) {
          if (doc.refused && doc.refused.length) {
            wait.textContent = doc.refused[0];
            return;
          }
          // One retry is the whole warming contract: the first pass
          // answers from what is ready and says so; two seconds later
          // the cold half has usually landed.
          if (doc.index === 'warming' && !retried) {
            retried = true;
            wait.textContent = 'warming the index \u00b7 the first pass is keyword-only';
            window.setTimeout(load, 2000);
            return;
          }
          if (doc.index !== 'warming') face.related[key] = { at: Date.now(), doc: doc };
          paintRelated(wrap, doc);
        })
        .catch(function () {
          wait.textContent = 'the related read did not land';
        });
    }

    load();
    return wrap;
  }

  function paintRelated(wrap, doc) {
    wrap.textContent = '';
    var rows = doc.related || [];
    if (!rows.length) {
      wrap.appendChild(node('p', 'related-wait',
        doc.scores === 'lexical' ? 'no shared words in the corpus' : 'no close match in the corpus'));
      return;
    }
    // The accepted-over-shown signal re-orders the rows without hiding
    // any: a pair the owner keeps opening rises by at most a third.
    rows = rows.slice().sort(function (left, right) {
      var leftScore = Number(left.score) * usageBoost('related', left.kind + ':' + left.id);
      var rightScore = Number(right.score) * usageBoost('related', right.kind + ':' + right.id);
      return rightScore - leftScore;
    });
    var list = node('div', 'related-list');
    rows.forEach(function (row) {
      var usageKey = 'related:' + row.kind + ':' + row.id;
      if (!face.relatedShown[usageKey]) {
        face.relatedShown[usageKey] = 1;
        usageRecord('suggestions', usageKey, 'shown');
      }
      var button = node('button', 'related-row');
      button.type = 'button';
      button.dataset.act = 'inspect';
      button.title = row.summary || ('open ' + row.name);
      button.appendChild(node('span', 'related-kind', row.kind));
      button.appendChild(node('span', 'related-name',
        row.kind === 'session' ? sessionLabel(row.id) : row.name));
      var score = node('span', 'related-score',
        doc.scores === 'cosine' ? Math.round(Number(row.score) * 100) + '%' : '\u2248');
      if (row.weak) score.dataset.weak = 'yes';
      button.appendChild(score);
      button.addEventListener('click', function () { openRelated(row); });
      list.appendChild(button);
    });
    wrap.appendChild(list);
    // The state is named, not implied: a weak percentage, a keyword
    // match, and a still-warming index are three different answers.
    wrap.appendChild(node('p', 'related-state',
      doc.scores === 'lexical'
        ? (doc.index === 'warming' ? 'keyword match \u00b7 the index is warming' : 'keyword match \u00b7 no embedder')
        : (doc.index === 'warming' ? 'cosine \u00b7 the index is still warming' : 'cosine')));
  }

  // One related row opens the object, not a second copy of it: facts
  // and skills through their own inspectors, sessions through the
  // replay, projects through the tower's drawer. A route or a cell has
  // no detail read of its own here, so it names what the relation is
  // and offers the panel that owns it.
  function openRelated(row) {
    usageRecord('suggestions', 'related:' + row.kind + ':' + row.id, 'accepted');
    if (row.kind === 'session') {
      openBranchSession(row.id);
      return;
    }
    if (row.kind === 'fact' || row.kind === 'skill') {
      ensureCorpus().then(function () {
        if (row.kind === 'fact') {
          var fact = ((face.memory && face.memory.facts) || []).find(function (item) {
            return item.name === row.id;
          });
          if (fact) { inspectFact(fact); return; }
        } else {
          var skill = ((face.skills && face.skills.skills) || []).find(function (item) {
            return item.name === row.id;
          });
          if (skill) { inspectSkill(skill); return; }
        }
        genericRelated(row);
      });
      return;
    }
    if (row.kind === 'project') {
      var project = ((face.tower && face.tower.projects) || []).find(function (item) {
        return item.name === row.id;
      });
      if (project) { projectDrawer(project, null, face.tower && face.tower.root); return; }
      readTower(false).then(function () {
        var loaded = ((face.tower && face.tower.projects) || []).find(function (item) {
          return item.name === row.id;
        });
        if (loaded) projectDrawer(loaded, null, face.tower && face.tower.root);
        else genericRelated(row);
      });
      return;
    }
    genericRelated(row);
  }

  function genericRelated(row) {
    var actions = node('div', 'replay-actions');
    var open = node('button', 'replay-button replay-resume',
      'open the ' + (row.kind === 'cell' ? 'calibration' : 'router') + ' panel');
    open.type = 'button';
    open.addEventListener('click', function () {
      window.location.hash = row.kind === 'cell' ? '#/calibration' : '#/router';
    });
    actions.appendChild(open);
    openDrawer(row.kind + ' \u00b7 ' + row.name, [
      ['the relation', node('p', 'derivation',
        (row.summary || 'no summary recorded') +
        (row.score == null ? '' : ' \u00b7 ' + Math.round(Number(row.score) * 100) + '%'))],
      ['what you can do', actions]
    ]);
  }

  function ensureCorpus() {
    var waits = [];
    if (!face.memory) waits.push(readMemory(false));
    if (!face.skills) waits.push(readSkills(false));
    return Promise.all(waits).catch(function () { });
  }

  function renderSections(container, sections) {
    sections.forEach(function (section) {
      var head = node('h3', null, section[0]);
      container.appendChild(head);
      if (section[1] instanceof Node) {
        container.appendChild(section[1]);
      } else {
        var para = node('p', 'derivation');
        para.innerHTML = section[1];
        container.appendChild(para);
      }
    });
  }

  function openDrawer(title, sections) {
    // The dock is the inspector wherever there is room. The fixed drawer
    // is the narrow-screen fallback, not a second surface to maintain.
    if (window.innerWidth > 1180 && el.dockObject) {
      setAside(true);
      el.dockObjectTitle.textContent = title;
      el.dockObjectBody.textContent = '';
      renderSections(el.dockObjectBody, sections);
      el.dockObject.hidden = false;
      el.faceCard.hidden = true;
      el.trustCard.hidden = true;
      window.setTimeout(function () {
        if (el.dockObject.scrollIntoView) {
          el.dockObject.scrollIntoView({ block: 'nearest' });
        }
      }, 0);
      return;
    }
    el.drawerTitle.textContent = title;
    el.drawerBody.textContent = '';
    renderSections(el.drawerBody, sections);
    el.drawer.hidden = false;
    el.drawer.setAttribute('aria-modal', 'true');
    el.scrim.hidden = false;
  }

  function closeDrawer() {
    if (el.dockObject && !el.dockObject.hidden) {
      el.dockObject.hidden = true;
      el.dockObjectBody.textContent = '';
      el.faceCard.hidden = false;
      el.trustCard.hidden = false;
      return;
    }
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
    syncDeckCard();
    showPaneTab('preview');
    openPane();
    paintPreview();
  }

  // The deck's action card: the grader's faults named, and the two
  // renders the owner chooses between — the auto-fit repair, or the
  // deck as written. Either click re-renders through the render route
  // and the card updates from the answer's own verdict.
  function deckCard(deck) {
    var card = node('div', 'deck-card');
    var title = node('div', 'deck-card-title');
    title.appendChild(node('span', 'deck-card-mark', '\u2726'));
    title.appendChild(node('span', null,
      'the grader found ' + JSON.parse(deck.faults).length +
      ' layout fault' + (JSON.parse(deck.faults).length === 1 ? '' : 's') +
      ' \u2014 repaired in place'));
    card.appendChild(title);
    JSON.parse(deck.faults).forEach(function (fault) {
      var what = fault.trigger === 'overfull'
        ? 'text sets to ' + fault.lines + ' lines in a box that holds ' + fault.holds
        : fault.trigger === 'unbalanced'
          ? 'columns read lopsided: ' + (fault.columns || []).join(', ') + ' lines'
          : 'the headline sets past its own box';
      card.appendChild(node('div', 'deck-card-fault', what + ' \u2014 ' + fault.node));
    });
    var actions = node('div', 'deck-card-actions');
    var apply = node('button', 'deck-card-apply', 'keep the repaired render');
    apply.type = 'button';
    apply.addEventListener('click', function () { applyDeckFix(deck.fix_deck); });
    var revert = node('button', 'deck-card-revert', 'revert to as written');
    revert.type = 'button';
    revert.addEventListener('click', function () { applyDeckFix(deck.original_deck); });
    actions.appendChild(apply);
    actions.appendChild(revert);
    card.appendChild(actions);
    return card;
  }

  // One click, one render: the deck JSON goes to the render route, the
  // answer's fragment and verdict replace what the pane shows, and the
  // card re-derives from the answer's own faults — an applied repair
  // empties it, a revert repopulates it.
  function applyDeckFix(deckJson) {
    fetch('/api/deck/render', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ deck: JSON.parse(deckJson) })
    })
      .then(function (reply) { return reply.json(); })
      .then(function (answered) {
        if (answered.status !== 'rendered') return;
        face.deck.html = answered.html;
        face.deck.faults = JSON.stringify(answered.faults || []);
        paintPreview();
        syncDeckCard();
      })
      .catch(function () { });
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
    face.workspace = state.workspace || '';
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
    face.gate = state.gate || null;
    face.process = null;

    el.log.textContent = '';
    face.stream = node('div', 'stream');
    el.log.appendChild(face.stream);
    // The empty state is the home when nothing has been said yet. A
    // boot-time skills event is not a conversation, so it must not push
    // the owner into a transcript with no words in it.
    var talked = (state.events || []).some(function (event) {
      return event.type === 'owner' || event.type === 'assistant' ||
        event.type === 'tool' || event.type === 'turn';
    });
    el.log.hidden = !talked;
    el.empty.hidden = talked;
    (state.events || []).forEach(apply);
    // The log is the truth about what happened; the snapshot is the
    // truth about now. A replayed log with no end record is an
    // interrupted run, not a live one, so the process card says so.
    face.running = !!state.running;
    if (face.process && !face.running && face.process.active) {
      face.process.active = false;
      face.process.ended = Date.now();
      face.process.stop = 'interrupted';
      face.process.stage = 'verify';
      face.process.last = 'the run was interrupted';
    }
    face.stick = true;
    settle();
    paint();
    paintEmpty();
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
    var source = new EventSource('/events');
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

  // The token leaves the address bar once the face boots, so a copied
  // URL or a screenshot carries nothing worth keeping. The hash stays:
  // it is the panel the owner opened, and the plan's route table is
  // keyed on it.
  window.history.replaceState(null, '', window.location.pathname + window.location.hash);

  // The four offers, ordered by what the owner actually accepts. A
  // familiar offer rises; nothing is hidden, and the static order is
  // the order when the knob is off or the counters are empty.
  function paintOffers() {
    el.offers.textContent = '';
    OFFERS.slice().sort(function (left, right) {
      return usageBoost('offer', right.id) - usageBoost('offer', left.id);
    }).forEach(function (offer) {
      var button = node('button', 'offer');
      button.type = 'button';
      button.dataset.act = 'offer';
      button.dataset.offer = offer.id;
      var mark = node('span', 'offer-mark');
      mark.appendChild(icon(offer.icon));
      button.appendChild(mark);
      var body = node('span', 'offer-body');
      body.appendChild(node('span', 'offer-title', offer.title));
      body.appendChild(node('span', 'offer-said', offer.said));
      button.appendChild(body);
      if (!face.offerShown[offer.id]) {
        face.offerShown[offer.id] = 1;
        usageRecord('suggestions', 'offer:' + offer.id, 'shown');
      }
      button.addEventListener('click', function () {
        usageRecord('suggestions', 'offer:' + offer.id, 'accepted');
        el.task.value = offer.prompt;
        grow();
        el.task.focus();
      });
      el.offers.appendChild(button);
    });
  }

  paintOffers();

  Array.prototype.forEach.call(el.empty.querySelectorAll('[data-step]'), function (button) {
    button.addEventListener('click', function () {
      if (button.dataset.step === 'plan') {
        el.plan.click();
        el.task.focus();
      } else if (button.dataset.step === 'delegate') {
        window.location.hash = '#/delegation';
      } else {
        ask('use the sys1 lane to review today\u2019s route decisions and report the fit');
      }
    });
  });

  Array.prototype.forEach.call(el.empty.querySelectorAll('[data-lane]'), function (button) {
    button.addEventListener('click', function () {
      var lane = button.dataset.lane;
      if (lane === 'judge') {
        ask('use the sys1 lane to review today\u2019s route decisions and report the fit');
      } else if (lane === 'memory') {
        window.location.hash = '#/memory';
      } else if (lane === 'skills') {
        window.location.hash = '#/skills';
      } else if (lane === 'web') {
        openPane();
        showPaneTab('web');
      } else if (lane === 'deck') {
        ask('build a deck that summarises the plan\u2019s next increment');
      }
    });
  });

  try { face.lastVisit = window.localStorage.getItem('fleet-last-visit') || ''; } catch (error) { }

  paintEmpty();
  // The welcome's one bounded read. It is the only boot-time read of the
  // corpus, the library, the archive, and the graph; the panels keep
  // their own reads for when the owner opens them.
  readHome(false).then(function () {
    try {
      window.localStorage.setItem('fleet-last-visit', new Date().toISOString());
    } catch (error) { }
  }).catch(function () { });

  // The draft survives a reload. A twelve-hour desk should not lose the
  // sentence being typed because the face was refreshed.
  try {
    var draft = window.localStorage.getItem('fleet-draft');
    if (draft) { el.task.value = draft; grow(); }
  } catch (error) { }

  try {
    var savedAside = window.localStorage.getItem('fleet-aside');
    // A first visit keeps the labeled nav and leaves the dock closed
    // until it is asked for; a wide desk has room for both. The topbar
    // chip and the process strip keep the run visible either way.
    setAside(savedAside ? savedAside === 'open' : window.innerWidth >= 1600);
  } catch (error) {
    setAside(window.innerWidth >= 1600);
  }
  if (el.asideToggle) {
    el.asideToggle.addEventListener('click', function () {
      setAside(document.body.dataset.aside === 'closed');
      if (face.panel === 'sessions') paintSessions();
    });
  }
  if (el.dockObjectClose) {
    el.dockObjectClose.addEventListener('click', closeDrawer);
  }
  if (el.processPill) {
    el.processPill.addEventListener('click', openProcess);
  }
  window.addEventListener('resize', function () {
    syncDock();
    if (face.panel === 'sessions') paintSessions();
  });

  if (el.railMenu) {
    el.railMenu.addEventListener('click', function () {
      setMobileRail(document.body.dataset.mobileRail !== 'open');
    });
  }
  if (el.railScrim) {
    el.railScrim.addEventListener('click', function () { setMobileRail(false); });
  }
  window.addEventListener('keydown', function (event) {
    if (event.key === 'Escape' && document.body.dataset.mobileRail === 'open') {
      setMobileRail(false);
    }
  });

  // The topbar's chips are not readouts. The route opens the board that
  // explains it, the state opens the archive at this run, and both
  // numbers open the derivation the inspector already carries.
  el.routePill.addEventListener('click', function () { window.location.hash = '#/router'; });
  el.statePill.addEventListener('click', function () {
    window.location.hash = face.session ? '#/sessions/' + encodeURIComponent(face.session) : '#/sessions';
  });
  [el.routePill, el.statePill].forEach(function (pill) {
    pill.tabIndex = 0;
    pill.setAttribute('role', 'button');
    pill.title = (pill === el.routePill ? 'where this run went' : 'this run\u2019s state') + ' \u00b7 click to open';
    pill.addEventListener('keydown', function (event) {
      if (event.key === 'Enter' || event.key === ' ') {
        event.preventDefault();
        pill.click();
      }
    });
  });
  Array.prototype.forEach.call(document.querySelectorAll('.stat'), function (stat) {
    stat.tabIndex = 0;
    stat.setAttribute('role', 'button');
    stat.title = 'open the derivation behind this number';
    stat.addEventListener('click', function () { inspect(null); });
    stat.addEventListener('keydown', function (event) {
      if (event.key === 'Enter' || event.key === ' ') {
        event.preventDefault();
        inspect(null);
      }
    });
  });

  el.composer.addEventListener('submit', function (event) {
    event.preventDefault();
    send();
  });

  el.task.addEventListener('input', function () {
    grow();
    try { window.localStorage.setItem('fleet-draft', el.task.value); } catch (error) { }
    typeAhead();
  });
  el.task.addEventListener('keydown', function (event) {
    // The type-ahead owns the keys while it is open, so a command is one
    // keystroke from anywhere in the composer. Enter runs the option the
    // screen is highlighting — the owner's ruling on KI-245: the selected
    // row is what Enter sends, whether the owner typed a command's name
    // or the intent that ranked it. Shift+Enter stays a newline.
    if (face.commandOpen) {
      if (event.key === 'ArrowDown') { event.preventDefault(); moveCommand(1); return; }
      if (event.key === 'ArrowUp') { event.preventDefault(); moveCommand(-1); return; }
      if ((event.key === 'Enter' || event.key === 'Tab') && !event.shiftKey) {
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
    // The semantic pass re-arms on every keystroke: the owner is typing a
    // sentence of intent, and the rank owes the finished sentence an
    // answer, not the fragment that happened to be in the box when the
    // first word settled. The debounce is what keeps a keystroke burst
    // from ranking every prefix; each keystroke resets it.
    window.clearTimeout(face.commandTimer);
    var epoch = face.commandEpoch;
    face.commandTimer = window.setTimeout(function () {
      rankSemantic(value.slice(1).trim(), epoch);
    }, 180);
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
  document.addEventListener('click', function (event) {
    var line = event.target.closest ? event.target.closest('.stream-line') : null;
    if (!line || line.dataset.wired || line.closest('.drawer')) return;
    openDrawer('record', [['this line', node('p', 'derivation', line.textContent)]]);
  });
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
      el.sessionsBoard.appendChild(pageHead('Work', 'Sessions',
        'Every run under this home, newest first. A row opens its replay beside the list.',
        null));
      var waiting = boardCard(12, 'reading', null, null);
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
    var viaChannel = sessions.filter(function (row) { return row.channel && row.channel.length; });
    tiles.appendChild(tile(sessions.length, 'logs in the archive', null, function () {
      face.sessionsFilter = 'all';
      paintSessions();
    }));
    tiles.appendChild(tile(viaChannel.length, 'arrived on a channel',
      viaChannel.length ? 'accent' : null, function () {
        face.sessionsFilter = 'channel';
        paintSessions();
      }));
    tiles.appendChild(tile(open, 'without an end record', open ? 'warn' : null, function () {
      face.sessionsFilter = 'open';
      paintSessions();
    }));
    tiles.appendChild(tile(children, 'delegated children', null, function () {
      face.sessionsFilter = 'delegated';
      paintSessions();
    }));
    tiles.appendChild(tile(compacted, 'compacted at least once', compacted ? 'warn' : null, function () {
      face.sessionsFilter = 'compacted';
      paintSessions();
    }));
    el.sessionsBoard.appendChild(pageHead('Work', 'Sessions',
      'Every run under this home, newest first. Select a row to read it beside the list.',
      read.read_at));
    el.sessionsBoard.appendChild(statStrip(tiles));

    if (sessions.length) {
      var shown = sessions.filter(sessionMatches);
      var bar = node('div', 'filter-bar');
      [['all', 'all'], ['open', 'open'], ['ended', 'ended'],
        ['delegated', 'delegated'], ['compacted', 'compacted'],
        ['channel', 'via a channel']].forEach(function (pair) {
        var chip = node('button', 'filter-chip', pair[1]);
        chip.type = 'button';
        chip.setAttribute('aria-pressed', face.sessionsFilter === pair[0] ? 'true' : 'false');
        chip.addEventListener('click', function () {
          face.sessionsFilter = pair[0];
          paintSessions();
        });
        bar.appendChild(chip);
      });
      bar.appendChild(node('span', 'filter-count',
        shown.length + ' of ' + sessions.length));
      el.sessionsBoard.appendChild(bar);

      // The split is a width decision, not a mode: below a 1000px track
      // the archive takes the whole board and the replay moves to the
      // dock (or, with no dock, below the list). It never overflows.
      var track = el.sessionsBoard.clientWidth || 0;
      var split = track >= 1000;
      var docked = !split && window.innerWidth > 1180 &&
        document.body.dataset.aside === 'open';
      var list = boardCard(split && face.replayId ? 7 : 12, 'the archive', null,
        split
          ? 'the row this face anchors is marked. Selecting a row reads it beside the list.'
          : docked
            ? 'the selected run reads in the dock beside the work.'
            : 'the selected run reads below the list.');
      var headRow = node('div', 'session-head-row');
      headRow.appendChild(node('span', null, 'session'));
      headRow.appendChild(node('span', null, 'served by'));
      headRow.appendChild(node('span', null, 'state'));
      headRow.appendChild(node('span', null, 'started'));
      list.appendChild(headRow);
      if (!shown.length) {
        list.appendChild(node('p', 'board-empty',
          'No run in the archive matches this filter. Clear it to see the whole list.'));
      }
      shown.forEach(function (row) { list.appendChild(sessionRow(row)); });
      el.sessionsBoard.appendChild(list);
      if (face.replayId) {
        if (face.replay) {
          if (split) paintReplayInto(el.sessionsBoard, face.replay, read.current, 5);
          else if (docked) showSessionObject(face.replay, read.current);
          else paintReplayInto(el.sessionsBoard, face.replay, read.current, 12);
        } else {
          var waitingReplay = boardCard(split ? 5 : 12, 'selected replay', null, null);
          waitingReplay.appendChild(node('p', 'board-empty', 'Reading this run\u2026'));
          el.sessionsBoard.appendChild(waitingReplay);
        }
      }
    } else {
      var none = boardCard(12, 'the archive', null, null);
      none.appendChild(node('p', 'board-empty',
        'This home holds no session logs yet. A run writes one log per session, whether it ' +
        'starts here, in the TUI, or headless.'));
      el.sessionsBoard.appendChild(none);
    }
  }

  // The archive's filter. It is the panel's own state and it never
  // survives a relaunch: a list that quietly hides rows on the next
  // visit is a list the owner cannot trust.
  function sessionMatches(row) {
    switch (face.sessionsFilter) {
      case 'open': return !row.ended;
      case 'ended': return !!row.ended;
      case 'delegated': return row.kind === 'subagent';
      case 'compacted': return !!row.compacted;
      case 'channel': return !!(row.channel && row.channel.length);
      default: return true;
    }
  }

  // Selecting a run is the interaction, not a navigation: the row stays
  // selected, the replay reads beside it, and the address carries the id
  // so back/forward and a copied link land on the same row.
  function selectSession(id) {
    closeDrawer();
    face.focus = { panel: 'sessions', id: id };
    if (face.replayId !== id) face.replay = null;
    face.replayId = id;
    if (window.location.hash !== '#/sessions/' + encodeURIComponent(id)) {
      window.history.replaceState(null, '', '#/sessions/' + encodeURIComponent(id));
    }
    readReplay(id, true);
    paintSessions();
    window.setTimeout(function () {
      var row = document.querySelector('.session-row[data-id="' + id + '"]');
      if (row) row.focus();
    }, 0);
  }

  function closeReplay() {
    face.replayId = null;
    face.replay = null;
    face.focus = null;
    window.history.replaceState(null, '', '#/sessions');
    paintSessions();
  }

  // One archive row: the id, the model, the markers the summary
  // carries, and a badge when it is the session this face anchors. A
  // click opens the replay; a double purpose needs no second button.
  // A session id is a timestamp and a hash. The list shows the part the
  // owner reads — the day, the minute, the hash — and keeps the whole id
  // in the tooltip and the copy action.
  function sessionLabel(id) {
    var text = String(id || '');
    var parts = /^(\d{4})(\d{2})(\d{2})T(\d{2})(\d{2})\d{2}Z-(.+)$/.exec(text);
    if (!parts) return text;
    return parts[2] + '-' + parts[3] + ' ' + parts[4] + ':' + parts[5] + ' \u00b7 ' + parts[6];
  }

  /// The origin line under a row's name: the project the session was
  /// born in, and — for a channel conversation — which transport carried
  /// it and who the peer is. An ordinary session in the launch directory
  /// has nothing to say here, and says nothing.
  function sessionWhere(row) {
    var parts = [];
    if (row.workspace) {
      var full = String(row.workspace);
      var segments = full.split('/');
      var project = node('span', 'session-row-project', segments[segments.length - 1] || full);
      project.title = full;
      parts.push(project);
    }
    if (row.channel && row.channel.length >= 2) {
      // The read carries (channel, peer id, name). The name is what an
      // owner recognizes; the id is what the Bots surface matches on.
      var peerName = row.channel.length > 2 ? row.channel[2] : row.channel[1];
      var via = node('span', 'session-row-via',
        'via ' + row.channel[0] + ' \u00b7 ' + (peerName || row.channel[1]));
      via.title = row.channel[0] + ' \u00b7 ' + row.channel[1];
      parts.push(via);
    }
    if (!parts.length) return null;
    var line = node('span', 'session-row-where');
    parts.forEach(function (part) { line.appendChild(part); });
    return line;
  }

  function sessionRow(row) {
    var line = node('button', 'session-row');
    line.type = 'button';
    line.dataset.act = 'inspect';
    line.dataset.id = row.id;
    line.title = row.id;
    if (face.replayId === row.id) line.setAttribute('aria-selected', 'true');
    var name = node('span', 'session-row-id');
    var top = node('span', 'session-row-top');
    top.appendChild(node('span', 'session-row-label', sessionLabel(row.id)));
    if (row.current) top.appendChild(badge('this face', 'accent'));
    name.appendChild(top);
    // Where the run came from, under its name rather than in a sixth
    // column: the archive stays four columns wide, and a row that has
    // something to say about its origin grows one line instead of
    // dragging the whole table out of alignment.
    var where = sessionWhere(row);
    if (where) name.appendChild(where);
    line.appendChild(name);
    // The model that actually served (the end record's, via the read).
    // Null while the run is in flight — nothing has served yet — and a
    // dash on an ended unrouted run. Never a default's name standing in
    // for a decision nobody made.
    line.appendChild(node('span', 'session-row-model',
      row.model || (row.ended ? '\u2014' : 'routing\u2026')));
    var marks = node('span', 'session-row-marks');
    if (row.kind) marks.appendChild(badge(row.kind, null));
    if (row.compacted) marks.appendChild(badge('compacted', 'warn'));
    marks.appendChild(badge(row.ended ? 'ended' : 'open', row.ended ? null : 'live'));
    line.appendChild(marks);
    line.appendChild(node('span', 'session-row-created', row.created || '\u2014'));
    line.addEventListener('click', function () { selectSession(row.id); });
    line.addEventListener('keydown', function (event) {
      var rows = Array.prototype.slice.call(
        line.parentNode.querySelectorAll('.session-row'));
      var at = rows.indexOf(line);
      if (event.key === 'ArrowDown' && at < rows.length - 1) {
        event.preventDefault();
        rows[at + 1].focus();
        selectSession(rows[at + 1].dataset.id);
      } else if (event.key === 'ArrowUp' && at > 0) {
        event.preventDefault();
        rows[at - 1].focus();
        selectSession(rows[at - 1].dataset.id);
      } else if (event.key === 'Escape') {
        event.preventDefault();
        closeReplay();
      }
    });
    return line;
  }

  function replayActions(replay, current) {
    var actions = node('div', 'replay-actions');
    var back = node('button', 'replay-button');
    back.type = 'button';
    back.textContent = 'close the replay';
    back.addEventListener('click', closeReplay);
    actions.appendChild(back);
    var copy = node('button', 'replay-button');
    copy.type = 'button';
    copy.textContent = 'copy id';
    copy.addEventListener('click', function () {
      if (navigator.clipboard) navigator.clipboard.writeText(replay.id).catch(function () { });
      copy.textContent = 'copied';
      window.setTimeout(function () { copy.textContent = 'copy id'; }, 1200);
    });
    actions.appendChild(copy);
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
    return actions;
  }

  function replayBody(replay) {
    var wrap = node('div', 'replay-body');
    if (replay.omitted) {
      wrap.appendChild(node('p', 'board-empty',
        replay.omitted + ' earlier card' + (replay.omitted === 1 ? ' is' : 's are') +
        ' left out of this preview. The log holds them; the preview shows the newest ' +
        'and the resume reseeds from the whole log.'));
    }
    var lines = node('div', 'replay-lines');
    (replay.cards || []).forEach(function (entry) { lines.appendChild(replayLine(entry)); });
    if (!(replay.cards || []).length) {
      lines.appendChild(node('p', 'board-empty', 'This log carries no messages yet.'));
    }
    wrap.appendChild(lines);
    return wrap;
  }

  // The replay in the dock, for a narrow track: the same run summary,
  // the same lines, and the same actions, without a second board card.
  function showSessionObject(replay, current) {
    var summary = (replay.model || 'routing\u2026') +
      (replay.kind ? ' \u00b7 ' + replay.kind : '') +
      ' \u00b7 ' + (replay.ended
        ? 'ended: ' + replay.stop_reason + ' \u00b7 ' + replay.turns + ' turns'
        : 'no end record \u2014 the run was interrupted');
    openDrawer('session \u00b7 ' + sessionLabel(replay.id), [
      ['the run', node('p', 'derivation', summary)],
      ['related', relatedFor('session', replay.id)],
      ['the replay', replayBody(replay)],
      ['what you can do', replayActions(replay, current)]
    ]);
  }

  // The replay card: the head the start record pinned, the summary the
  // end record and the compaction marks carry, the resume that
  // re-anchors this face, and the cards themselves.
  function paintReplayInto(board, replay, current, span) {
    if (!replay) return;
    if (replay.refused && replay.refused.length) {
      board.appendChild(refusedCard(replay.refused));
      return;
    }
    var summary = (replay.model || 'routing\u2026') +
      (replay.kind ? ' \u00b7 ' + replay.kind : '') +
      ' \u00b7 ' + (replay.ended
        ? 'ended: ' + replay.stop_reason + ' \u00b7 ' + replay.turns + ' turns'
        : 'no end record \u2014 the run was interrupted');
    var card = boardCard(span || 5, 'replay \u00b7 ' + sessionLabel(replay.id), replay.created, summary);
    card.appendChild(replayActions(replay, current));
    card.appendChild(relatedFor('session', replay.id));
    card.appendChild(replayBody(replay));
    board.appendChild(card);
  }

  // One replayed record, as a line. The vocabulary is the live
  // transcript's, narrowed to what a log can reconstruct.
  function replayLine(entry) {
    var line = node('div', 'replay-line');
    line.dataset.kind = entry.type;
    line.tabIndex = 0;
    line.setAttribute('role', 'button');
    line.addEventListener('click', function () {
      openDrawer('replay \u00b7 ' + entry.type, [
        ['the record', node('pre', 'raw', JSON.stringify(entry, null, 2))]
      ]);
    });
    line.addEventListener('keydown', function (event) {
      if (event.key === 'Enter' || event.key === ' ') {
        event.preventDefault();
        line.click();
      }
    });
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
    } else if (entry.type === 'channel') {
      // The booking the log opens with: which transport carried this
      // conversation and who the peer is. It reads here rather than
      // falling through to a bare type name, which is what a record the
      // page does not know looks like.
      line.appendChild(node('span', 'replay-who', 'channel'));
      line.appendChild(node('span', 'replay-text',
        'via ' + entry.channel + ' \u00b7 ' + (entry.peer || entry.peer_id || 'an unnamed peer')));
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
      el.calibrationBoard.appendChild(pageHead('System', 'Calibration',
        'The day\u2019s stream, the judged quality of what served, and the gate that decides whether the failover may learn from it.',
        null));
      var waiting = boardCard(12, 'reading', null, null);
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
    var filter = face.calibrationFilter || 'all';
    var tiles = node('div', 'tiles');
    tiles.appendChild(tile(read.rows_read, 'rows, the day\u2019s last ' + read.cap));
    tiles.appendChild(tile(judged, 'judged rows'));
    tiles.appendChild(tile(cells.length, 'fit cells', null, function () {
      face.calibrationFilter = 'all';
      paintCalibration();
    }));
    tiles.appendChild(tile(fit, 'fit', fit ? 'live' : null, function () {
      face.calibrationFilter = 'fit';
      paintCalibration();
    }));
    tiles.appendChild(tile(informing, 'informing the failover', null, function () {
      face.calibrationFilter = 'informing';
      paintCalibration();
    }));
    tiles.appendChild(tile(read.gate && read.gate.informs_failover ? 'on' : 'off',
      'failover gate'));
    el.calibrationBoard.appendChild(pageHead('System', 'Calibration',
      'The day\u2019s stream, the judged quality of what served, and the gate that decides whether the failover may learn from it.',
      read.day, {
        label: 'Run a gate review',
        run: function () {
          ask('use the sys1 lane to grade today\u2019s route rows and report the fit');
        }
      }));
    el.calibrationBoard.appendChild(statStrip(tiles));
    el.calibrationBoard.appendChild(filterBar([
      { value: 'all', label: 'all', count: cells.length },
      { value: 'fit', label: 'fit', count: fit },
      { value: 'informing', label: 'informing', count: informing }
    ], filter, function (value) {
      face.calibrationFilter = value;
      paintCalibration();
    }));

    var shownCells = cells.filter(function (cell) {
      if (filter === 'fit') return cell.fit === 'fit';
      if (filter === 'informing') return !!cell.informs_failover;
      return true;
    });
    if (shownCells.length) {
      var cellCard = boardCard(12, 'the judged quality, per provider and shape',
        shownCells.length + (filter === 'all' ? '' : ' of ' + cells.length),
        'a cell is fit when its sample is big enough and wide enough \u2014 ' +
        read.fit_rule.rows + ' graded rows, a ' + read.fit_rule.span + ' confidence span');
      shownCells.forEach(function (cell) { cellCard.appendChild(calibrationCell(cell)); });
      el.calibrationBoard.appendChild(cellCard);
    } else if (cells.length) {
      var empty = boardCard(12, 'the judged quality', null, null);
      empty.appendChild(node('p', 'board-empty',
        'No cell matches ' + filter + '. The chip\u2019s count says how many there are; ' +
        'clear the filter to see them.'));
      el.calibrationBoard.appendChild(empty);
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
    card.dataset.act = 'inspect';
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
    card.tabIndex = 0;
    card.setAttribute('role', 'button');
    card.title = 'open this fit cell';
    card.addEventListener('click', function () {
      var actions = node('div', 'replay-actions');
      var judge = node('button', 'replay-button replay-resume');
      judge.type = 'button';
      judge.textContent = 'ask sys1 about this cell';
      judge.addEventListener('click', function () {
        closeDrawer();
        ask('use the sys1 lane to review the ' + cell.provider +
          (cell.workload ? ' \u00b7 ' + cell.workload : '') +
          ' fit cell and say what is still missing');
      });
      actions.appendChild(judge);
      openDrawer('calibration \u00b7 ' + cell.provider, [
        ['the cell', node('p', 'derivation',
          (cell.workload || 'no shape judged') + ' \u00b7 ' + cell.fit +
          ' \u00b7 ' + cell.judged + ' judged \u00b7 ' + cell.right + ' right \u00b7 ' +
          cell.wrong + ' wrong \u00b7 span ' + fmtRate(cell.confidence_span))],
        ['what it still needs', node('p', 'derivation',
          wants.length ? wants.join(' \u00b7 ') : 'this cell informs the failover')],
        ['related', relatedFor('cell', cell.provider + '::' + (cell.workload || ''))],
        ['what you can do', actions]
      ]);
    });
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
    line.dataset.act = 'inspect';
    line.title = 'open this ' + op + ' record';
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
    line.tabIndex = 0;
    line.setAttribute('role', 'button');
    line.title = 'open this record';
    line.dataset.wired = '1';
    line.addEventListener('click', function () {
      openDrawer(op + ' row', [['the record', node('pre', 'raw', JSON.stringify(row, null, 2))]]);
    });
    line.addEventListener('keydown', function (event) {
      if (event.key === 'Enter' || event.key === ' ') {
        event.preventDefault();
        line.click();
      }
    });
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
      el.towerBoard.appendChild(pageHead('System', 'Tower',
        'The projects, the claims filed against them, and the evidence behind both.',
        null));
      var waiting = boardCard(12, 'reading', null, null);
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
    var dirty = projects.filter(function (row) { return row.dirty > 0; }).length;
    var filter = face.towerFilter || 'all';
    var tiles = node('div', 'tiles');
    tiles.appendChild(tile(projects.length, 'projects tracked', null, function () {
      face.towerFilter = 'all';
      paintTower();
    }));
    tiles.appendChild(tile(kpis.doing || 0, 'open tasks', kpis.doing ? 'warn' : null));
    tiles.appendChild(tile(kpis.open_conflicts || 0, 'open conflicts',
      kpis.open_conflicts ? 'down' : null));
    tiles.appendChild(tile(reportsToday(feed), 'reports today'));
    tiles.appendChild(tile(dirty, 'dirty worktrees', null, function () {
      face.towerFilter = 'dirty';
      paintTower();
    }));
    el.towerBoard.appendChild(pageHead('System', 'Tower',
      'The projects, the claims filed against them, and the evidence behind both.',
      read.read_at));
    el.towerBoard.appendChild(statStrip(tiles));
    el.towerBoard.appendChild(filterBar([
      { value: 'all', label: 'all', count: projects.length },
      { value: 'dirty', label: 'dirty', count: dirty }
    ], filter, function (value) {
      face.towerFilter = value;
      paintTower();
    }));

    // The hero: reports filed per day against commits shipped per day.
    var hero = boardCard(12, 'activity', null,
      'reports filed per day against the commits the shipped timeline names, over the last ' +
      'fourteen days. The feed is as fresh as the derived state\u2019s last build and says so. ' +
      'A day opens the feed on that day.');
    hero.appendChild(twoSeries(daily, timeline, function (day) {
      face.towerDay = face.towerDay === day ? '' : day;
      paintTower();
    }));
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
    var rankedAll = projects.slice();
    var ranked = (filter === 'dirty'
      ? rankedAll.filter(function (row) { return row.dirty > 0; })
      : rankedAll
    ).sort(function (left, right) {
      return right.commits_30d - left.commits_30d;
    });
    var top = ranked.reduce(function (max, row) { return Math.max(max, row.commits_30d); }, 0);
    var rankCard = boardCard(12, 'the ranked projects', null,
      'trailing thirty days of commits, the share of the whole, and the git facts as read. ' +
      'The board\u2019s own read is dirtiest-first; this list ranks by the thirty-day count.');
    ranked.forEach(function (row) {
      var health = brain.filter(function (entry) { return entry.name === row.name; })[0];
      rankCard.appendChild(projectRow(row, health, top, read.root));
    });
    if (!ranked.length) {
      rankCard.appendChild(node('p', 'board-empty',
        'No dirty worktree right now. Clear the filter to see every project.'));
    }
    el.towerBoard.appendChild(rankCard);

    // The claims against the evidence, and the precision each harness
    // earned.
    var claims = boardCard(12, 'claims against the evidence', kpis.open_conflicts + ' open',
      'a claim the evidence rejected, with the reality that rejected it. The trust lines are ' +
      'each harness\u2019s own precision.');
    if (conflicts.length) {
      conflicts.forEach(function (row) {
        var line = node('div', 'stream-line');
        line.dataset.act = 'inspect';
        line.title = 'open this claim\u2019s record';
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
        line.dataset.act = 'inspect';
        line.title = 'open this harness\u2019s precision record';
        line.appendChild(node('span', 'stream-ts', row.harness));
        line.appendChild(node('span', 'stream-text',
          row.evidenced + ' of ' + row.claimed + ' claims carried evidence \u00b7 precision ' +
          fmtRate(row.precision)));
        claims.appendChild(line);
      });
    }
    el.towerBoard.appendChild(claims);

    // The feed, newest first, with its own staleness named. A day
    // chosen on the activity chart narrows it; the chip is removable.
    var feedCard = boardCard(12, 'the report feed', (read.reports_at || '\u2014') + ' built',
      'every filed report, newest first. The feed is as fresh as the derived state\u2019s last ' +
      'build and says so rather than pretending to be live.');
    var feedShown = feed;
    if (face.towerDay) {
      feedShown = feed.filter(function (row) {
        return (row.ts || '').slice(0, 10) === face.towerDay;
      });
      var dayChip = node('button', 'filter-chip filter-chip-active',
        face.towerDay + ' \u00b7 ' + feedShown.length + ' \u00d7');
      dayChip.type = 'button';
      dayChip.dataset.act = 'clear-filter';
      dayChip.title = 'clear the day filter';
      dayChip.addEventListener('click', function () {
        face.towerDay = '';
        paintTower();
      });
      var dayBar = node('div', 'filter-bar');
      dayBar.appendChild(dayChip);
      feedCard.appendChild(dayBar);
    }
    if (feedShown.length) {
      feedShown.forEach(function (row) {
        var line = node('div', 'stream-line');
        line.dataset.act = 'inspect';
        line.tabIndex = 0;
        line.setAttribute('role', 'button');
        line.title = 'open the project this report names';
        line.appendChild(node('span', 'stream-ts', (row.ts || '').slice(5, 16)));
        line.appendChild(node('span', 'stream-text',
          row.kind + ' \u00b7 ' + row.project + ' \u00b7 ' + row.summary +
          (row.commit ? ' \u00b7 ' + row.commit.slice(0, 12) : ' \u00b7 no commit filed')));
        line.addEventListener('click', function (event) {
          event.stopPropagation();
          var project = projects.filter(function (entry) {
            return entry.name === row.project;
          })[0];
          if (project) {
            var health = brain.filter(function (entry) {
              return entry.name === row.project;
            })[0];
            projectDrawer(project, health, read.root);
          } else {
            openDrawer('report \u00b7 ' + row.project, [
              ['the report', node('pre', 'raw', JSON.stringify(row, null, 2))]
            ]);
          }
        });
        feedCard.appendChild(line);
      });
    } else {
      feedCard.appendChild(node('p', 'board-empty',
        face.towerDay
          ? 'No report names ' + face.towerDay + '. Clear the day to see the whole feed.'
          : 'No reports are in the feed. The ledger at ~/.second-brain is where they are filed.'));
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
        line.dataset.act = 'inspect';
        line.title = 'open this skipped directory\u2019s record';
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
  function twoSeries(daily, timeline, onDay) {
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
      if (onDay) {
        // A transparent hit area over the day's pair: a bar is a shape,
        // and the day is what a click means.
        var hit = svg('rect', {
          x: x - 4, y: 6, width: 28, height: base - 6,
          fill: 'transparent', class: 'chart-hit', 'data-act': 'filter-day',
          role: 'button', tabindex: '0'
        });
        hit.addEventListener('click', function () { onDay(day); });
        hit.addEventListener('keydown', function (event) {
          if (event.key === 'Enter' || event.key === ' ') {
            event.preventDefault();
            onDay(day);
          }
        });
        var title = svg('title');
        title.textContent = day + ' \u00b7 ' + (reports[day] || 0) + ' reports \u00b7 ' +
          (commits[day] || 0) + ' commits';
        hit.appendChild(title);
        plot.appendChild(hit);
      }
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
  function projectRow(row, health, top, root) {
    var line = node('div', 'rank-row');
    line.dataset.act = 'inspect';
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
    line.tabIndex = 0;
    line.setAttribute('role', 'button');
    line.title = 'open ' + row.name + '\u2019s git facts';
    line.addEventListener('click', function () { projectDrawer(row, health, root); });
    line.addEventListener('keydown', function (event) {
      if (event.key === 'Enter' || event.key === ' ') {
        event.preventDefault();
        projectDrawer(row, health, root);
      }
    });
    return line;
  }

  // One project's detail: the git facts as the read returned them, the
  // plans the derived state names, and the two actions worth having —
  // copy the path, or ask the agent to look at the project's next step.
  function projectDrawer(row, health, root) {
    var actions = node('div', 'replay-actions');
    var path = root ? String(root).replace(/\/$/, '') + '/' + row.name : row.name;
    var copy = node('button', 'replay-button');
    copy.type = 'button';
    copy.textContent = 'copy the path';
    copy.addEventListener('click', function () {
      if (navigator.clipboard) navigator.clipboard.writeText(path).catch(function () { });
      copy.textContent = 'copied';
      window.setTimeout(function () { copy.textContent = 'copy the path'; }, 1200);
    });
    actions.appendChild(copy);
    var askBtn = node('button', 'replay-button replay-resume');
    askBtn.type = 'button';
    askBtn.textContent = 'ask about this project';
    askBtn.addEventListener('click', function () {
      closeDrawer();
      ask('read ' + path + ' and tell me the next feature-sized step, given the plan and the dirty tree');
    });
    actions.appendChild(askBtn);
    var plans = (health && health.plans) || [];
    openDrawer('project \u00b7 ' + row.name, [
      ['the git facts', node('p', 'derivation',
        (row.branch || '\u2014') + ' @ ' + (row.head || '\u2014') +
        ' \u00b7 ' + row.commits_30d + ' commits in 30 days' +
        (row.dirty ? ' \u00b7 ' + row.dirty + ' dirty files' : ' \u00b7 clean') +
        (row.ahead == null ? '' : ' \u00b7 \u2191' + row.ahead + ' \u2193' + row.behind))],
      ['the plans', node('p', 'derivation',
        plans.length
          ? plans.map(function (plan) { return plan.status + ' ' + plan.count; }).join(' \u00b7 ')
          : 'the derived state names no plan for this project')],
      ['related', relatedFor('project', row.name)],
      ['what you can do', actions]
    ]);
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
      el.brainBoard.appendChild(pageHead('Knowledge', 'Second brain',
        'The derived state, quieter: the gates, the loops, the harness runs, and the maturity signals.',
        null));
      var waiting = boardCard(12, 'reading', null, null);
      waiting.appendChild(node('p', 'board-empty',
        'The derived-state read has not landed yet. It reads the six state files under the ' +
        'second-brain home, which is this machine\u2019s own.'));
      el.brainBoard.appendChild(waiting);
      return;
    }
    if (read.refused && read.refused.length) {
      el.brainBoard.appendChild(refusedCard(read.refused));
    }
    var tiles = node('div', 'tiles');
    var reports = read.reports || {};
    tiles.appendChild(tile(reports.doing_total || 0, 'open tasks'));
    tiles.appendChild(tile(reports.pending_verification || 0, 'awaiting verification',
      reports.pending_verification ? 'warn' : null));
    tiles.appendChild(tile(reports.open_conflicts || 0, 'conflicts',
      reports.open_conflicts ? 'down' : null));
    tiles.appendChild(tile(reports.suppressed || 0, 'suppressed rows'));
    el.brainBoard.appendChild(pageHead('Knowledge', 'Second brain',
      'The derived state, quieter: the gates, the loops, the harness runs, and the maturity signals.',
      read.read_at));
    el.brainBoard.appendChild(statStrip(tiles));
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
          line.dataset.act = 'inspect';
          line.title = 'open this gate verdict';
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
        line.dataset.act = 'inspect';
        line.title = 'open this quality loop';
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
        line.dataset.act = 'inspect';
        line.title = 'open this harness loop';
        line.appendChild(node('span', 'stream-ts', row.name));
        line.appendChild(node('span', 'stream-text',
          row.runs + ' runs \u00b7 ' + row.converged + ' converged \u00b7 last ' + row.last_status));
        harnessCard.appendChild(line);
      });
      (harness.recent || []).slice(0, 8).forEach(function (row) {
        var line = node('div', 'stream-line');
        line.dataset.act = 'inspect';
        line.title = 'open this harness event';
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
        line.dataset.act = 'inspect';
        line.title = 'open this milestone';
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

  // ------------------------------------------------------------- bots

  // The Bots surface: who can reach this fleet, whether the service that
  // carries them is up, and what they have done. Five cards and the
  // actions that change them — not a console. The tower is the one
  // surface allowed cockpit density.
  //
  // The read is `GET /api/channels`, one bounded document: the service's
  // own status (and whether its heartbeat still beats), every channel's
  // paired and pending peers, and the newest activity with the bound
  // named. The CLI and the TUI read the same files, so the three cannot
  // disagree about who is in or whether anything is listening.

  function readBots(force) {
    var now = Date.now();
    if (!force && face.bots && now - face.botsAt < 4000) return Promise.resolve();
    face.botsAt = now;
    return fetch('/api/channels', { headers: { accept: 'application/json' } })
      .then(function (reply) {
        if (reply.status === 401) throw new Error('token');
        return reply.json();
      })
      .then(function (doc) {
        face.bots = doc;
        markBotsCount();
        if (face.panel === 'bots') paintBots();
      })
      .catch(function () { });
  }

  /// The rail's count: the peers waiting at the door plus the peer seats
  /// waiting on an answer. Both are things an owner has to act on, and
  /// nothing else on this surface is.
  function markBotsCount() {
    var read = face.bots;
    if (!read) return;
    var pending = 0;
    (read.channels || []).forEach(function (channel) {
      pending += (channel.pending || []).length;
    });
    var waiting = Number(read.waiting_seats || 0);
    var total = pending + waiting;
    var spot = el.railTree.querySelector('[data-count="bots"]');
    if (!spot) return;
    if (total > 0) {
      spot.textContent = String(total);
      spot.hidden = false;
      spot.title = pending + ' waiting at the door, ' + waiting + ' waiting on an answer';
    } else {
      spot.textContent = '';
      spot.hidden = true;
      spot.removeAttribute('title');
    }
  }

  // One write, then the read again: the surface paints what the server
  // confirmed, and a refusal says what the writer said rather than a
  // generic failure.
  function writeBots(path, body, onDone) {
    return fetch(path, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(body || {})
    }).then(function (reply) {
      return reply.json().then(function (doc) { return { ok: reply.ok, doc: doc }; });
    }).then(function (result) {
      if (!result.ok) {
        notice(result.doc.error || 'the write was refused');
        return null;
      }
      return readBots(true).then(function () {
        paintBots();
        if (onDone) onDone(result.doc);
        return result.doc;
      });
    }).catch(function () { notice('could not reach fleet'); return null; });
  }

  function channelName(report) {
    return report.name;
  }

  function phaseTone(phase) {
    if (phase === 'live') return 'ok';
    if (phase === 'failed') return 'down';
    if (phase === 'starting') return 'warn';
    return null;
  }

  function capabilityWord(value) {
    return value === 'observe' ? 'may only listen' : 'may act';
  }

  function paintBots() {
    var read = face.bots;
    el.botsBoard.textContent = '';
    if (!read) {
      el.botsBoard.appendChild(pageHead('System', 'Bots',
        'The channels that carry work in from outside: who is paired, what they may do, ' +
        'whether the service is listening, and what they have asked for.',
        null));
      var waiting = boardCard(12, 'reading', null, null);
      waiting.appendChild(node('p', 'board-empty',
        'The channel read has not landed yet. It reads the home\u2019s channel stores and ' +
        'the service\u2019s own status document.'));
      el.botsBoard.appendChild(waiting);
      return;
    }
    if (read.refused && read.refused.length) {
      el.botsBoard.appendChild(refusedCard(read.refused));
    }
    var service = read.service || {};
    var channels = read.channels || [];
    var activity = read.activity || [];
    var bound = read.activity_bound == null ? activity.length : read.activity_bound;
    var reports = service.channels || [];

    var live = reports.filter(function (row) { return row.phase === 'live'; }).length;
    var failed = reports.filter(function (row) { return row.phase === 'failed'; }).length;
    var peers = [];
    var pending = [];
    channels.forEach(function (channel) {
      (channel.peers || []).forEach(function (peer) {
        peers.push(peer);
      });
      (channel.pending || []).forEach(function (peer) {
        pending.push(peer);
      });
    });
    var deliveries = activity.filter(function (line) { return line.kind === 'delivery'; }).length;
    var refusedAtTheDoor = activity.filter(function (line) {
      return line.kind === 'refusal';
    }).length;

    var filter = face.botsFilter || 'all';
    // A count of one says so: "1 peers paired" is the kind of number that
    // makes an owner distrust the rest of the board.
    var plural = function (count, one, many) {
      return count === 1 ? '1 ' + one : count + ' ' + many;
    };
    var tiles = node('div', 'tiles');
    tiles.appendChild(tile(live, live === 1 ? 'channel live' : 'channels live',
      live ? 'ok' : null, function () {
      face.botsFilter = 'all';
      paintBots();
    }));
    tiles.appendChild(tile(failed, failed === 1 ? 'channel failed' : 'channels failed',
      failed ? 'down' : null));
    tiles.appendChild(tile(peers.length, peers.length === 1 ? 'peer paired' : 'peers paired',
      null, function () {
      face.botsFilter = 'peers';
      paintBots();
    }));
    tiles.appendChild(tile(pending.length, 'waiting at the door', pending.length ? 'warn' : null,
      function () {
        face.botsFilter = 'pending';
        paintBots();
      }));
    tiles.appendChild(tile(refusedAtTheDoor,
      refusedAtTheDoor === 1 ? 'refusal logged' : 'refusals logged'));
    var waitingSeats = Number(read.waiting_seats || 0);
    tiles.appendChild(tile(waitingSeats, 'waiting on an answer', waitingSeats ? 'warn' : null));
    el.botsBoard.appendChild(pageHead('System', 'Bots',
      'The channels that carry work in from outside: who is paired, what they may do, ' +
      'whether the service is listening, and what they have asked for.',
      read.read_at));
    el.botsBoard.appendChild(statStrip(tiles));
    el.botsBoard.appendChild(filterBar([
      { value: 'all', label: 'all', count: peers.length },
      { value: 'acting', label: 'may act',
        count: peers.filter(function (peer) { return peer.capability === 'task'; }).length },
      { value: 'listening', label: 'may only listen',
        count: peers.filter(function (peer) { return peer.capability === 'observe'; }).length },
      { value: 'pending', label: 'pending', count: pending.length }
    ], filter, function (value) {
      face.botsFilter = value;
      paintBots();
    }));

    // The service: what is listening, and the start/stop that changes it.
    // A stale heartbeat is named rather than rendered as a live service.
    var serviceCard = boardCard(12, 'the service',
      service.running ? 'live' : (service.updated || null),
      service.stale
        ? (service.note || 'the heartbeat has stopped')
        : (service.pid
          ? 'pid ' + service.pid + ' \u00b7 started ' + (service.started || '\u2014') +
            ' \u00b7 beating every ' + (service.heartbeat_secs || 0) + 's'
          : (service.note || 'no channel service has run in this home')));
    if (!reports.length) {
      serviceCard.appendChild(node('p', 'board-empty',
        'Nothing to start: this build hosts the webhook channel and Telegram when its bot ' +
        'token is set. `channels.enabled` in the settings, or `fleet gui --channels`, turns ' +
        'the service on.'));
    }
    reports.forEach(function (report) {
      var row = node('div', 'bots-row');
      row.dataset.channel = report.name;
      var head = node('div', 'bots-row-main');
      head.appendChild(node('span', 'bots-row-name', report.name));
      head.appendChild(badge(report.phase, phaseTone(report.phase)));
      if (report.port) head.appendChild(node('span', 'bots-row-meta', '127.0.0.1:' + report.port));
      head.appendChild(node('span', 'bots-row-meta',
        'since ' + (report.since || '\u2014') + (service.stale ? ' (last report)' : '')));
      row.appendChild(head);
      if (report.error) row.appendChild(node('p', 'board-empty', report.error));
      var acts = node('div', 'bots-acts');
      if (report.phase === 'live' || report.phase === 'starting') {
        acts.appendChild(botsButton('stop', 'stop', function () {
          writeBots('/api/channels/stop', { name: report.name });
        }));
      } else {
        acts.appendChild(botsButton('start', 'start', function () {
          writeBots('/api/channels/start', { name: report.name });
        }));
      }
      row.appendChild(acts);
      serviceCard.appendChild(row);
    });
    el.botsBoard.appendChild(serviceCard);

    // Waiting at the door: a refusal recorded the peer, so it can be let
    // in from here instead of hunting an id out of a log.
    var doorCard = boardCard(12, 'waiting at the door', pending.length
      ? pending.length + (pending.length === 1 ? ' peer' : ' peers')
      : null,
      'peers that tried to reach a channel and are not paired. Letting one in mints its ' +
      'credential; turning one away forgets it until it tries again.');
    if (!pending.length) {
      doorCard.appendChild(node('p', 'board-empty',
        'Nothing is waiting. A refusal lands here the moment an unpaired peer tries.'));
    }
    pending.forEach(function (peer) {
      var row = node('div', 'bots-row');
      row.dataset.id = peer.id;
      var head = node('div', 'bots-row-main');
      head.appendChild(node('span', 'bots-row-name', peer.name || '(no name announced)'));
      head.appendChild(node('span', 'bots-row-meta', peer.channel + ' \u00b7 ' + peer.id));
      head.appendChild(node('span', 'bots-row-meta',
        peer.attempts + (peer.attempts === 1 ? ' attempt' : ' attempts') +
        ' \u00b7 last ' + (peer.last_seen || '\u2014')));
      row.appendChild(head);
      var acts = node('div', 'bots-acts');
      acts.appendChild(botsButton('let-in-task', 'let it in \u00b7 may act', function () {
        writeBots('/api/channels/pair', {
          channel: peer.channel, id: peer.id, name: peer.name, capability: 'task'
        }, revealPairing);
      }));
      acts.appendChild(botsButton('let-in-listen', 'let it in \u00b7 may only listen', function () {
        writeBots('/api/channels/pair', {
          channel: peer.channel, id: peer.id, name: peer.name, capability: 'observe'
        }, revealPairing);
      }));
      acts.appendChild(botsButton('turn-away', 'turn away', function () {
        writeBots('/api/channels/forget', { channel: peer.channel, id: peer.id });
      }));
      row.appendChild(acts);
      doorCard.appendChild(row);
    });
    el.botsBoard.appendChild(doorCard);

    // The pair form: a webhook peer's token is minted server-side and
    // shown exactly once, in a card that is dismissed rather than
    // re-read — the store keeps only its digest, so there is nothing to
    // come back to.
    var pairCard = boardCard(5, 'let one in', null,
      'A webhook peer gets a token minted here and shown once. A Telegram chat is let in by ' +
      'the id it refused under. The capability is part of the pairing, not a later edit.');
    var form = node('form', 'bots-form');
    var channelPick = node('select', 'bots-input');
    channelPick.name = 'channel';
    ['webhook', 'telegram'].forEach(function (name) {
      var choice = node('option', null, name);
      choice.value = name;
      channelPick.appendChild(choice);
    });
    form.appendChild(labelled('channel', channelPick));
    var idInput = node('input', 'bots-input');
    idInput.name = 'id';
    idInput.type = 'text';
    idInput.placeholder = 'a telegram chat id (webhook mints its own)';
    form.appendChild(labelled('peer id', idInput));
    var nameInput = node('input', 'bots-input');
    nameInput.name = 'name';
    nameInput.type = 'text';
    nameInput.placeholder = 'the CI box';
    form.appendChild(labelled('name', nameInput));
    var capPick = node('select', 'bots-input');
    capPick.name = 'capability';
    [['task', 'may act'], ['observe', 'may only listen']].forEach(function (pair) {
      var choice = node('option', null, pair[1]);
      choice.value = pair[0];
      capPick.appendChild(choice);
    });
    form.appendChild(labelled('may', capPick));
    var send = node('button', 'page-action', 'pair');
    send.type = 'submit';
    send.dataset.act = 'pair';
    form.appendChild(send);
    form.addEventListener('submit', function (event) {
      event.preventDefault();
      writeBots('/api/channels/pair', {
        channel: channelPick.value,
        id: idInput.value,
        name: nameInput.value,
        capability: capPick.value
      }, revealPairing);
    });
    pairCard.appendChild(form);
    el.botsBoard.appendChild(pairCard);

    // Who can reach the fleet: the peers, their capability, and the two
    // actions that change it.
    var shown = peers.filter(function (peer) {
      if (filter === 'acting') return peer.capability === 'task';
      if (filter === 'listening') return peer.capability === 'observe';
      if (filter === 'pending') return false;
      return true;
    });
    var peersCard = boardCard(7, 'who can reach the fleet', shown.length
      ? shown.length + (shown.length === 1 ? ' peer' : ' peers')
      : null,
      'a row opens the peer: its capability, its conversation, and its last delivery. ' +
      'The capability changes here; nothing about a peer is hidden behind a second surface.');
    if (!shown.length) {
      peersCard.appendChild(node('p', 'board-empty',
        filter === 'all'
          ? 'Nothing is paired. Let a peer in from the form, or `fleet channels pair-webhook`.'
          : 'No peer matches this filter. Clear it to see every peer.'));
    }
    shown.forEach(function (peer) {
      var row = node('div', 'bots-row bots-peer');
      row.dataset.id = peer.id;
      var head = node('div', 'bots-row-main');
      var open = node('button', 'bots-row-name bots-open', peer.name || peer.id);
      open.type = 'button';
      open.dataset.act = 'inspect-peer';
      open.addEventListener('click', function () {
        openPeer(peer, activity, peers, channels);
      });
      head.appendChild(open);
      head.appendChild(badge(capabilityWord(peer.capability),
        peer.capability === 'task' ? 'ok' : null));
      head.appendChild(node('span', 'bots-row-meta', peer.channel + ' \u00b7 ' + peer.id));
      head.appendChild(node('span', 'bots-row-meta', 'paired ' + (peer.paired || '\u2014')));
      row.appendChild(head);
      var acts = node('div', 'bots-acts');
      var flip = peer.capability === 'task' ? 'observe' : 'task';
      acts.appendChild(botsButton(
        'capability', 'make it ' + capabilityWord(flip), function () {
          writeBots('/api/channels/capability', {
            channel: peer.channel, id: peer.id, capability: flip
          });
        }
      ));
      acts.appendChild(botsButton('revoke', 'throw it out', function () {
        writeBots('/api/channels/revoke', { channel: peer.channel, id: peer.id });
      }));
      row.appendChild(acts);
      peersCard.appendChild(row);
    });
    el.botsBoard.appendChild(peersCard);

    // What they have done. The bound is the document's, named here: a
    // surface that shows fifty of three hundred silently is lying about
    // what it shows.
    var fedCard = boardCard(12, 'what they have done', activity.length
      ? 'newest ' + activity.length + ' of the last ' + bound
      : null,
      'deliveries, refusals, replies, seats, and state changes, newest last. The log is the ' +
      'service\u2019s own file; a row names the peer and the session it landed in.');
    if (!activity.length) {
      fedCard.appendChild(node('p', 'board-empty',
        'Nothing recorded yet. The log fills as peers deliver, answer, and are refused.'));
    }
    var feed = node('div', 'bots-feed');
    activity.slice().reverse().forEach(function (line) {
      feed.appendChild(activityRow(line));
    });
    fedCard.appendChild(feed);
    el.botsBoard.appendChild(fedCard);
  }

  /// The pairing's one-time reveal: the token exists here and nowhere
  /// else, because the store kept its digest. Dismissing the card is the
  /// only way to lose it, and the card says so.
  function revealPairing(doc) {
    var sections = [
      ['the peer', node('p', 'derivation',
        (doc.name || doc.id) + ' on ' + doc.channel + ' \u00b7 ' + capabilityWord(doc.capability))],
      ['how to reach it', node('p', 'derivation',
        doc.channel === 'webhook'
          ? 'POST to http://127.0.0.1:' + webhookPort() + '/hook with the header ' +
            'x-fleet-peer, carrying the token below.'
          : 'the bot answers chat ' + doc.id + '; it presents no token.')]
    ];
    if (doc.token) {
      var token = node('code', 'bots-token', doc.token);
      sections.push(['the token \u2014 shown once', token]);
      sections.push(['what happens next', node('p', 'derivation',
        doc.reveal || 'this is the only time the token is shown; store it now')]);
    }
    openDrawer('paired \u00b7 ' + (doc.name || doc.id), sections);
  }

  /// The port the service says the webhook holds, so the reveal names the
  /// address that is actually listening rather than the compiled-in
  /// default. A service that is not running has no port to name, and the
  /// pairing still happened — the line says where it will answer.
  function webhookPort() {
    var service = (face.bots && face.bots.service) || {};
    var reports = service.channels || [];
    for (var index = 0; index < reports.length; index += 1) {
      if (reports[index].name === 'webhook' && reports[index].port) {
        return reports[index].port;
      }
    }
    return 'the configured port';
  }

  /// One peer's inspector: what it may do, where its conversation is, and
  /// what it last did.
  function openPeer(peer, activity, peers, channels) {
    var mine = activity.filter(function (line) {
      return line.peer_id === peer.id && line.channel === peer.channel;
    });
    var session = null;
    mine.forEach(function (line) {
      if (line.session) session = line.session;
    });
    var last = mine.length ? mine[mine.length - 1] : null;
    var sections = [
      ['what it may do', node('p', 'derivation',
        capabilityWord(peer.capability) +
        (peer.capability === 'task'
          ? ' \u2014 it can hand fleet a task and receives the outcome'
          : ' \u2014 it is notified and cannot hand fleet a task'))],
      ['its conversation', node('p', 'derivation',
        session ? session : 'no delivery has landed yet, so it has no conversation')],
      ['its last delivery', node('p', 'derivation',
        last ? (last.kind + ' at ' + (last.at || '\u2014') + ' \u00b7 ' +
          (last.task || last.detail || '')) : 'nothing recorded')],
      ['paired', node('p', 'derivation', peer.paired || '\u2014')]
    ];
    if (session) {
      var go = node('button', 'replay-button replay-resume');
      go.type = 'button';
      go.textContent = 'open the conversation';
      go.addEventListener('click', function () {
        window.location.hash = '#/sessions/' + encodeURIComponent(session);
      });
      sections.push(['where it lands', go]);
    }
    openDrawer(peer.channel + ' \u00b7 ' + (peer.name || peer.id), sections);
  }

  function activityRow(line) {
    var row = node('div', 'bots-activity');
    row.dataset.kind = line.kind;
    row.appendChild(node('span', 'bots-activity-at', clock(line.at)));
    row.appendChild(badge(line.kind, line.kind === 'refusal' ? 'down'
      : line.kind === 'delivery' ? 'ok' : null));
    var said = node('span', 'bots-activity-said');
    said.textContent = line.peer || line.peer_id || line.channel;
    row.appendChild(said);
    var detail = line.task || line.detail || '';
    if (detail) row.appendChild(node('span', 'bots-row-meta', detail));
    if (line.session) {
      var open = node('button', 'bots-session', line.session);
      open.type = 'button';
      open.dataset.act = 'open-session';
      open.addEventListener('click', function () {
        window.location.hash = '#/sessions/' + encodeURIComponent(line.session);
      });
      row.appendChild(open);
    }
    return row;
  }

  function botsButton(act, label, run) {
    var button = node('button', 'bots-act', label);
    button.type = 'button';
    button.dataset.act = act;
    button.addEventListener('click', function () {
      usageRecord('actions', 'bots:' + act, 'uses');
      run();
    });
    return button;
  }

  function labelled(text, field) {
    var wrap = node('label', 'bots-field');
    wrap.appendChild(node('span', 'bots-label', text));
    wrap.appendChild(field);
    return wrap;
  }

  function paintSettings() {
    var read = face.settings;
    el.settingsBoard.textContent = '';
    if (!read) {
      el.settingsBoard.appendChild(pageHead('System', 'Settings',
        'Every knob this home carries, written through the same writer the CLI uses.',
        null));
      var waiting = boardCard(12, 'reading', null, null);
      waiting.appendChild(node('p', 'board-empty',
        'The settings read has not landed yet. It reads this home\u2019s own config, routing, ' +
        'and env files.'));
      el.settingsBoard.appendChild(waiting);
      return;
    }
    if (read.refused && read.refused.length) {
      el.settingsBoard.appendChild(refusedCard(read.refused));
    }
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
    el.settingsBoard.appendChild(pageHead('System', 'Settings',
      'Every knob this home carries, the two lists and the rules, the cron jobs, and the identity slot. ' +
      'Every write rides the writer the CLI rides, so the two faces cannot disagree about what the file holds.',
      read.read_at));
    el.settingsBoard.appendChild(statStrip(tiles));

    // The knobs, each editable in place: the value box saves through
    // the one writer, and the source badge says which file answers.
    var knobs = boardCard(12, 'the knobs', null,
      'the value in the box is what the home holds. A knob sourced from the environment says ' +
      'so \u2014 a file write would not take effect until the env var is unset.');
    var groups = {};
    var order = [];
    (read.scalars || []).forEach(function (knob) {
      var prefix = knob.key.indexOf('.') > 0
        ? knob.key.split('.')[0]
        : (/^[A-Z0-9_]+$/.test(knob.key) ? 'environment' : 'general');
      if (!groups[prefix]) { groups[prefix] = []; order.push(prefix); }
      groups[prefix].push(knob);
    });
    var groupNames = {
      general: 'general',
      environment: 'environment',
      calib: 'calibration',
      route_cache: 'route cache',
      skills: 'skills',
      ui: 'interface'
    };
    order.forEach(function (prefix) {
      knobs.appendChild(node('h3', 'form-section',
        groupNames[prefix] || prefix.replace(/_/g, ' ')));
      groups[prefix].forEach(function (knob) { knobs.appendChild(knobRow(knob)); });
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
  if (el.processStop) el.processStop.addEventListener('click', stopRun);
  if (el.processSteerForm) el.processSteerForm.addEventListener('submit', steerRun);
  if (el.processSteer) {
    el.processSteer.addEventListener('keydown', function (event) {
      if (event.key === 'Escape') {
        event.preventDefault();
        el.processSteer.value = '';
        el.processSteer.blur();
      }
    });
  }

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
    try { window.localStorage.removeItem('fleet-draft'); } catch (error) { }
    el.empty.hidden = true;
    el.log.hidden = false;
    face.planned = true;
    post('/api/plan', { task: task }).then(function (reply) {
      if (reply.status === 200) {
        face.running = true;
        face.stop = null;
        resetProcess('plan');
        paint();
        return;
      }
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
        if (reply.status === 200) {
          face.running = true;
          face.stop = null;
          resetProcess('run');
          paint();
          return;
        }
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
    bots: el.panelBots,
    settings: el.panelSettings
  };

  function showRoute() {
    var raw = (window.location.hash || '').replace(/^#\/?/, '');
    var parts = raw.split('/');
    var which = parts[0] || '';
    var focusId = parts.length > 1 ? decodeURIComponent(parts.slice(1).join('/')) : '';
    var known = ROUTES.filter(function (route) { return route[0] === which; })[0];
    var address = which === 'chat' || !known ? 'chat' : which;
    closeDrawer();
    face.panel = address;
    usageRecord('panels', address, 'opens');
    face.focus = focusId ? { panel: address, id: focusId } : null;
    try { window.localStorage.setItem('fleet-panel', address); } catch (error) { }
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
      bots: function () { paintBots(); readBots(true).then(paintBots); },
      sessions: function () {
        if (face.focus && face.focus.id) face.replayId = face.focus.id;
        paintSessions();
        readSessions(true).then(function () {
          if (!face.replayId && face.sessions && (face.sessions.sessions || []).length) {
            face.replayId = face.sessions.sessions[0].id;
            face.focus = { panel: 'sessions', id: face.replayId };
            window.history.replaceState(null, '',
              '#/sessions/' + encodeURIComponent(face.replayId));
          }
          paintSessions();
          if (face.replayId) readReplay(face.replayId, true).then(paintSessions);
        });
      },
      calibration: function () { paintCalibration(); readCalibration(true).then(paintCalibration); },
      tower: function () { paintTower(); readTower(true).then(paintTower); },
      'second-brain': function () { paintBrain(); readBrain(true).then(paintBrain); },
      settings: function () { paintSettings(); readSettings(true).then(paintSettings); }
    };
    if (arrive[address]) arrive[address]();
    // The welcome is a read, not a snapshot: returning to the
    // conversation refreshes its counts after a run or a reload.
    if (address === 'chat') {
      readHome(false).then(function () {
        paintHome();
        paintRecent();
        paintSince();
      }).catch(function () { });
    }
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
      // The action card, when the grader found faults and the repair is
      // pre-baked: what was measured, and the two renders between which
      // the owner chooses — the repair or their own words. One click
      // either way, and the pane re-renders from the answer.
      if ((face.deck.faults || '').length && face.deck.fix_deck) {
        el.panePreview.appendChild(deckCard(face.deck));
      }
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
  function rankSemantic(intent, epoch) {
    if (intent.length < 3) return Promise.resolve();
    return post('/api/rank', {
      asked: intent,
      candidates: COMMANDS.map(function (command) {
        return command[0] + ' ' + command[1].join(' ') + ' ' + command[2];
      })
    }).then(function (reply) {
      if (epoch !== undefined && epoch !== face.commandEpoch) return;
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
  }

  function closeCommands() {
    face.commandOpen = false;
    face.commands = [];
    face.commandEpoch += 1;
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

  // The rail: the plan's left column. Each destination is a heading and a
  // row that opens the surface an owner means first, with the group's own
  // surfaces flat beneath it — built from GROUPS, the same declaration the
  // palette searches, so the two can never disagree about where a surface
  // lives. The merged row names are the approved mockup's own words.
  function buildRail() {
    el.railTree.textContent = '';
    GROUPS.forEach(function (group) {
      var id = group[0], label = group[1], items = group[2];
      var section = node('div', 'surfaces-section');
      section.dataset.group = id;
      section.appendChild(node('span', 'surfaces-group', label));
      items.forEach(function (item) {
        var address = item[0], name = item[1];
        var link = node('button', 'surfaces-link');
        link.type = 'button';
        link.dataset.address = address;
        link.setAttribute('aria-label', name);
        link.title = name;
        link.appendChild(icon(address, 'surfaces-icon'));
        link.appendChild(node('span', 'surfaces-label', name));
        // A surface that has something waiting says so on the rail, and
        // says nothing when it does not — an empty counter is decoration.
        var count = node('span', 'surfaces-count');
        count.dataset.count = address;
        count.hidden = true;
        link.appendChild(count);
        link.addEventListener('click', function () {
          window.location.hash = '#/' + address;
          setMobileRail(false);
        });
        section.appendChild(link);
      });
      el.railTree.appendChild(section);
    });
  }

  // Which surfaces are live today, and which are a later increment — the
  // rail says so rather than pretending all ten are built.
  var BUILT = ['chat', 'router', 'delegation', 'memory', 'skills', 'sessions', 'calibration',
    'tower', 'second-brain', 'bots', 'settings'];

  function markRail() {
    var which = (window.location.hash || '').replace(/^#\/?/, '');
    Array.prototype.forEach.call(el.rail.querySelectorAll('.surfaces-link'), function (link) {
      if (link.dataset.address === which) {
        link.setAttribute('aria-current', 'page');
      } else {
        link.removeAttribute('aria-current');
      }
    });
    // The topbar says where you are in the same words the rail uses. The
    // browser tab follows, so a desk with three fleet windows open is
    // navigable by name rather than by address.
    var place = WHERE[which] || ['work', 'conversation'];
    if (el.crumbGroup) el.crumbGroup.textContent = place[0];
    if (el.crumbPage) el.crumbPage.textContent = place[1];
    var title = place[1].replace(/(^|\s)\S/g, function (letter) { return letter.toUpperCase(); });
    document.title = title + ' \u00b7 fleet';
  }

  // The mobile rail: a drawer over the work, opened from the topbar and
  // dismissed by the scrim, the menu button, Escape, or any destination.
  function setMobileRail(open) {
    document.body.dataset.mobileRail = open ? 'open' : '';
    if (el.railScrim) el.railScrim.hidden = !open;
    if (el.railMenu) el.railMenu.setAttribute('aria-expanded', open ? 'true' : 'false');
  }

  // The rail's collapsed state, remembered across launches: deep in a run
  // the rail is chrome, and while navigating it is the map, so the one
  // switch worth offering is a mode rather than a fork of the structure.
  function railCollapsed() {
    // The live mode, not the stored preference: the dock can auto-fold
    // the rail without writing that fold to storage.
    return document.body.dataset.rail === 'icons';
  }

  function setRail(mode) {
    applyRail(mode, true);
    syncDock();
  }

  function applyRail(mode, remember) {
    document.body.dataset.rail = mode;
    el.railToggle.setAttribute('aria-expanded', mode === 'icons' ? 'false' : 'true');
    el.railToggle.setAttribute('aria-label',
      mode === 'icons' ? 'expand the surfaces rail' : 'collapse the surfaces rail');
    if (el.railToggleName) {
      el.railToggleName.textContent = mode === 'icons' ? 'expand the rail' : 'collapse the rail';
    }
    if (remember) {
      try { window.localStorage.setItem('fleet-rail', mode); } catch (error) { }
    }
  }

  function dockMode() {
    if (document.body.dataset.aside === 'closed' || window.innerWidth <= 1180) return 'closed';
    if (window.innerWidth >= 1600) return 'column';
    return document.body.dataset.rail === 'icons' ? 'column' : 'overlay';
  }

  function syncDock() {
    // At the wide breakpoint the nav is the owner's choice again; a fold
    // that was only there to make room for the dock is undone.
    if (window.innerWidth >= 1600 && face.railBeforeDock) {
      var restore = face.railBeforeDock;
      face.railBeforeDock = null;
      applyRail(restore, false);
    }
    document.body.dataset.dock = dockMode();
    paintProcess();
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
        var mark = node('span', 'palette-row-mark');
        mark.appendChild(icon(item[0]));
        row.appendChild(mark);
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
    ['paper', 'paper', 'warm light \u00b7 the default, a console read in daylight'],
    ['console', 'console', 'warm dark \u00b7 for a long evening beside the work'],
    ['graphite', 'graphite', 'cool dark \u00b7 a neutral ground without the warm cast'],
    ['contrast', 'contrast', 'high contrast \u00b7 pure ground, near-black ink, the accessibility theme']
  ];

  // The theme. One attribute on the document element is the whole
  // switch — the alias tier re-points under it and every component token
  // follows — so there is one thing to set and nothing else to keep in
  // step. Paper is the default; the choice is remembered.
  function themeNow() {
    var now = document.documentElement.dataset.theme;
    return THEMES.some(function (theme) { return theme[0] === now; }) ? now : 'paper';
  }

  function setTheme(mode) {
    document.documentElement.dataset.theme = mode;
    el.themeToggle.dataset.theme = mode;
    var named = THEMES.filter(function (theme) { return theme[0] === mode; })[0];
    el.themeToggle.setAttribute('aria-label', 'choose a theme \u2014 now ' + (named ? named[1] : mode));
    el.themeToggle.title = 'choose a theme \u2014 now ' + (named ? named[1] : mode);
    try { window.localStorage.setItem('fleet-theme-2', mode); } catch (error) { }
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
  // ------------------------------------------------- the action cards

  // The card bus: a source measures a flaw, pre-bakes the correction,
  // and pushes one card — id, mark, title, the words beside it, and the
  // clicks it offers. The strip above the composer renders whatever is
  // here, regardless of which surface the work is in; a source refreshes
  // by pushing the same id again, and clears by dropping it.
  function pushActionCard(card) {
    face.actionCards = face.actionCards.filter(function (existing) {
      return existing.id !== card.id;
    });
    face.actionCards.push(card);
    paintActions();
  }

  function dropActionCard(id) {
    var before = face.actionCards.length;
    face.actionCards = face.actionCards.filter(function (existing) {
      return existing.id !== id;
    });
    if (face.actionCards.length !== before) paintActions();
  }

  function paintActions() {
    if (!el.actions) return;
    el.actions.textContent = '';
    if (!face.actionCards.length) {
      el.actions.hidden = true;
      return;
    }
    face.actionCards.forEach(function (card) {
      var card0 = node('div', 'action');
      card0.dataset.kind = card.id;
      card0.appendChild(node('span', 'action-mark', card.mark || '\u25d0'));
      var body = node('div', 'action-body');
      body.appendChild(node('p', 'action-title', card.title));
      if (card.said) body.appendChild(node('p', 'action-said', card.said));
      (card.do || []).forEach(function (doable) {
        var fix = node('button', 'action-do', doable.label);
        fix.type = 'button';
        fix.addEventListener('click', doable.run);
        body.appendChild(fix);
      });
      card0.appendChild(body);
      el.actions.appendChild(card0);
    });
    el.actions.hidden = false;
  }

  // The deck's card: what the grader found on the deck as written, and
  // the two clicks — keep the repaired render, or revert to the owner's
  // own words. Either click re-renders through the render route, and
  // the card re-derives from the answer's own faults.
  function syncDeckCard() {
    var deck = face.deck;
    var faults = [];
    try { faults = JSON.parse((deck && deck.faults) || '[]'); } catch (error) { faults = []; }
    if (deck && faults.length && deck.fix_deck && deck.original_deck) {
      pushActionCard({
        id: 'deck-repair',
        mark: '\u2726',
        title: 'the deck has ' + faults.length + ' layout fault' + (faults.length === 1 ? '' : 's') +
          ' \u2014 repaired in place',
        said: faults.map(function (fault) {
          return fault.trigger + ' on ' + fault.node;
        }).join(' \u00b7 ') + ' \u2014 one click keeps the repair or reverts to your words',
        do: [
          { label: 'keep the repaired render', run: function () { applyDeckFix(deck.fix_deck); } },
          { label: 'revert to as written', run: function () { applyDeckFix(deck.original_deck); } },
        ],
      });
    } else {
      dropActionCard('deck-repair');
    }
  }

  function paintContrast() {
    var doc = face.contrast;
    var here = doc && (doc.themes || []).filter(function (entry) {
      return entry.theme === themeNow();
    })[0];
    if (!here || here.holds) { dropActionCard('contrast'); return; }
    var best = (doc.themes || []).filter(function (entry) { return entry.holds; })
      .sort(function (left, right) { return right.worst - left.worst; })[0];
    pushActionCard({
      id: 'contrast',
      mark: '\u25d0',
      title: 'this theme falls below the readable floor',
      said: 'its worst text is ' + here.worst.toFixed(2) + ':1 against the ' + doc.floor.toFixed(1) +
        ':1 floor \u2014 ' + here.failing.map(function (entry) {
          return entry.token + ' at ' + entry.ratio.toFixed(2) + ':1';
        }).join(', '),
      do: best ? [{
        label: 'switch to ' + best.theme + ' (' + best.worst.toFixed(2) + ':1)',
        run: function () {
          setTheme(best.theme);
          readContrast(true);
        },
      }] : [],
    });
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
    if (face.panel === 'sessions') paintSessions();
  });
  el.paletteOpen.addEventListener('click', openPalette);
  el.railJump.addEventListener('click', openPalette);
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
  // A returning owner lands where they left off, not on a default page.
  if (!window.location.hash) {
    try {
      var lastPanel = window.localStorage.getItem('fleet-panel');
      if (lastPanel && PANEL_EL[lastPanel]) window.location.hash = '#/' + lastPanel;
    } catch (error) { }
  }
  showRoute();

  window.setInterval(function () {
    if (face.process && face.process.active) paintProcess();
  }, 1000);

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

  // The counters land after the first offers paint; the offers repaint
  // in their learned order, and the exposure was already recorded once.
  readUsage().then(paintOffers);

  load().then(listen).catch(function (error) {
    if (error && error.message === 'token') {
      el.empty.hidden = false;
      el.empty.querySelector('h1').textContent = 'This page is not the one fleet printed.';
      el.empty.querySelector('p').textContent =
        'The launch token did not arrive with this request. Open the URL fleet printed.';
    }
  });
})();
