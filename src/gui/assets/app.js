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
    ['vault', 'vault'],
    ['settings', 'settings'],
    ['second-brain', 'second brain'],
    ['workspace-history', 'workspace history'],
    ['bots', 'bots']
  ];

  // The same surfaces, grouped by what the owner came to do rather
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
      ['second-brain', 'second brain'],
      ['workspace-history', 'workspace history']
    ]],
    ['system', 'system', [
      ['bots', 'bots'],
      ['vault', 'vault'],
      ['settings', 'settings'],
      ['calibration', 'calibration']
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
        if (face.running || face.seat) {
          controlRun({ fold: true }, 'the fold is armed',
            'the context folds at the next turn boundary', 'the fold was refused');
          return;
        }
        notice('nothing to fold yet',
          'the context folds between turns; with no run in flight there is nothing assembled to fold');
      }],
    ['pin', ['provider', 'model'], 'send the run in flight to a provider, or auto',
      function (rest) {
        var asked = rest.trim();
        if (!asked) {
          notice('pin needs a provider',
            '/pin <provider|provider/model|auto> \u00b7 a mid-run pin re-routes the next turn; with no run it sets the next run\u2019s pin');
          return;
        }
        if (face.running || face.seat) {
          controlRun({ pin: asked },
            'the route is pinned for the next turn',
            asked.toLowerCase() === 'auto' ? 'the pin is dropped' : 'the next turn crosses ' + asked,
            'the pin was refused');
          return;
        }
        // No run in flight: this is the composer's provider pin, the
        // same choice the select makes, applied to the next run.
        var match = null;
        if (el.runProvider) {
          for (var i = 0; i < el.runProvider.options.length; i++) {
            if (el.runProvider.options[i].value === asked) { match = asked; break; }
          }
        }
        if (match === null) {
          notice('no such provider', '/providers lists the roster \u00b7 `auto` drops the pin');
          return;
        }
        face.providerPin = match === 'auto' ? '' : match;
        paintRunbar();
        notice('the pin is set for the next run', match === 'auto' ? 'the router decides' : match);
      }],
    ['rename', ['name', 'title'], 'name this session in the archive',
      function (rest) {
        var title = rest.trim();
        var path = '/api/session/title' + (face.session ? '?session=' + encodeURIComponent(face.session) : '');
        post(path, { title: title }).then(function (reply) {
          if (reply.status === 200) {
            notice(title ? 'the session is named' : 'the session name is cleared',
              title || 'the archive falls back to the first message\u2019s line');
            if (face.panel === 'sessions') readSessions(true).then(paintSessions);
            return;
          }
          notice('the rename was refused', reply.body.error || 'the face said no without a reason');
        }).catch(function () {
          notice('the rename did not cross', 'the server did not answer');
        });
      }],
    ['pause', ['freeze', 'hold'], 'freeze the run in flight at its next step',
      function () {
        if (!(face.running || face.seat)) {
          notice('no run to pause', 'a pause holds a run in flight');
          return;
        }
        controlRun({ pause: true }, 'paused by you',
          'the run holds its place at the next model call \u00b7 /resume continues it',
          'the pause was refused');
      }],
    ['resume', ['continue', 'unpause'], 'let a paused run continue',
      function () {
        controlRun({ resume: true }, 'the run resumes',
          'it continues from where the pause stopped it', 'the resume was refused');
      }],
    ['clear', ['new', 'reset'], 'start a new conversation \u2014 a real session, not a cleared view',
      function () { newConversation(); }],
    ['approve', ['allow', 'yes'], 'answer the waiting approval: once, always, or deny',
      function (rest) { answer(rest.trim() || 'once'); }],
    ['help', ['commands', '?'], 'list every command and what it does',
      function () { openDrawer('commands', commandHelp()); }],
    ['review', ['diff', 'changes'], 'review this session’s file changes against a checkpoint',
      function () { openReview(); }],
    ['btw', ['aside', 'ps'], 'ask a question beside the run (one model call)',
      function (rest) {
        var question = rest.trim();
        if (!question) {
          notice('btw needs a question',
            '/btw <question> \u00b7 answered beside the run, never inside it');
          return;
        }
        var path = '/api/btw' + (face.session ? '?session=' + encodeURIComponent(face.session) : '');
        post(path, { text: question }).then(function (reply) {
          if (reply.status !== 200) {
            notice('the aside was refused', reply.body.error || 'the face said no without a reason');
            return;
          }
          notice('btw \u00b7 ' + clip(question, 50),
            reply.body.answer + ' \u00b7 a side ask on ' +
            (reply.body.provider || 'the route') + '/' + (reply.body.model || '') +
            ', not part of the run');
        }).catch(function () {
          notice('the aside did not cross', 'the server did not answer');
        });
      }],
    ['steer', ['redirect', 'adjust'], 'send an instruction into the run in flight',
      function (rest) {
        var text = rest.trim();
        if (!text) {
          notice('steer needs an instruction',
            '/steer <text> \u00b7 the run takes it at its next step');
          return;
        }
        post('/api/steer', { text: text }).then(function (reply) {
          if (reply.status === 200) {
            notice('the steer is sent', 'the run takes it at its next step');
            return;
          }
          notice('the steer was refused', reply.body.error || 'the face said no without a reason');
        }).catch(function () {
          notice('the steer did not cross', 'the server did not answer');
        });
      }],
    ['goal', ['objective', 'target'], 'set, show or clear this session’s standing objective',
      function (rest) {
        var text = rest.trim();
        if (!text) {
          notice('this session\u2019s goal', face.goal
            ? face.goal + ' \u00b7 /goal clear drops it'
            : 'no goal is set \u00b7 /goal <text> sets one');
          return;
        }
        var clearing = /^(clear|drop|remove|off)$/i.test(text);
        var path = '/api/goal' + (face.session ? '?session=' + encodeURIComponent(face.session) : '');
        post(path, { text: clearing ? '' : text }).then(function (reply) {
          if (reply.status === 200) {
            face.goal = reply.body.text || null;
            paintGoal();
            notice(clearing ? 'the goal is cleared' : 'the goal is set',
              clearing ? 'every run in this session goes back to the task alone'
                : 'carried by every run in this session \u00b7 the chip names it');
            return;
          }
          notice('the goal was refused', reply.body.error || 'the face said no without a reason');
        }).catch(function () {
          notice('the goal did not cross', 'the server did not answer');
        });
      }]
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
    'workspace-history': [
      ['circle', { cx: 6, cy: 5.5, r: 2.4 }],
      ['circle', { cx: 6, cy: 12, r: 2.4 }],
      ['circle', { cx: 6, cy: 18.5, r: 2.4 }],
      ['path', { d: 'M6 8v2.5M6 14.5v2' }],
      ['circle', { cx: 17, cy: 9.5, r: 2.4 }],
      ['path', { d: 'M8.4 6.6 14.6 8.8' }],
      ['path', { d: 'M8.4 12.6 14.6 10.2' }]
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
    dockFold: document.getElementById('dock-fold'),
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
    pairOpen: document.getElementById('pair-open'),
    pairScrim: document.getElementById('pair-scrim'),
    pairCard: document.getElementById('pair-card'),
    pairBody: document.getElementById('pair-body'),
    pairClose: document.getElementById('pair-close'),
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
    workCol: document.getElementById('work-col'),
    workHead: document.getElementById('work-head'),
    workList: document.getElementById('work-list'),
    convHead: document.getElementById('conv-head'),
    convBack: document.getElementById('conv-back'),
    convTitle: document.getElementById('conv-title'),
    convName: document.getElementById('conv-name'),
    convState: document.getElementById('conv-state'),
    convNew: document.getElementById('conv-new'),
    convRename: document.getElementById('conv-rename'),
    convArchive: document.getElementById('conv-archive'),
    panelCalibration: document.getElementById('panel-calibration'),
    panelSecondBrain: document.getElementById('panel-second-brain'),
    panelWorkspaceHistory: document.getElementById('panel-workspace-history'),
    panelBots: document.getElementById('panel-bots'),
    panelVault: document.getElementById('panel-vault'),
    panelSettings: document.getElementById('panel-settings'),
    panelLater: document.getElementById('panel-later'),
    laterTitle: document.getElementById('later-title'),
    laterNote: document.getElementById('later-note'),
    routerBoard: document.getElementById('router-board'),
    delegationBoard: document.getElementById('delegation-board'),
    memoryBoard: document.getElementById('memory-board'),
    skillsBoard: document.getElementById('skills-board'),
    sessionsBoard: document.getElementById('work-list'),
    calibrationBoard: document.getElementById('calibration-board'),
    brainBoard: document.getElementById('brain-board'),
    workspaceHistoryBoard: document.getElementById('workspace-history-board'),
    botsBoard: document.getElementById('bots-board'),
    vaultBoard: document.getElementById('vault-board'),
    settingsBoard: document.getElementById('settings-board'),
    routePill: document.getElementById('route-pill'),
    statePill: document.getElementById('state-pill'),
    spend: document.getElementById('spend'),
    context: document.getElementById('context'),
    contextMeter: document.getElementById('context-meter'),
    contextFill: document.getElementById('context-fill'),
    log: document.getElementById('log'),
    toLive: document.getElementById('to-live'),
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
    chatSmaller: document.getElementById('chat-smaller'),
    chatBigger: document.getElementById('chat-bigger'),
    send: document.getElementById('send'),
    sendLabel: document.getElementById('send-label'),
    queueChip: document.getElementById('queue-chip'),
    queueCount: document.getElementById('queue-count'),
    queueNote: document.getElementById('queue-note'),
    queueList: document.getElementById('queue-list'),
    runGoal: document.getElementById('run-goal'),
    runGoalBtn: document.getElementById('run-goal-btn'),
    goalPop: document.getElementById('goal-pop'),
    goalPopState: document.getElementById('goal-pop-state'),
    goalPopInput: document.getElementById('goal-pop-input'),
    goalPopSet: document.getElementById('goal-pop-set'),
    goalPopClear: document.getElementById('goal-pop-clear'),
    goalPopClose: document.getElementById('goal-pop-close'),
    plan: document.getElementById('plan'),
    hint: document.getElementById('hint'),
    runPerms: document.getElementById('run-perms'),
    runMode: document.getElementById('run-mode'),
    runProvider: document.getElementById('run-provider'),
    runTurns: document.getElementById('run-turns'),
    runFolder: document.getElementById('run-folder'),
    docOpen: document.getElementById('doc-open'),
    docPop: document.getElementById('doc-pop'),
    docPopPath: document.getElementById('doc-pop-path'),
    docPopList: document.getElementById('doc-pop-list'),
    docPopNote: document.getElementById('doc-pop-note'),
    docConsentSend: document.getElementById('doc-consent-send'),
    docConsentCancel: document.getElementById('doc-consent-cancel'),
    folderPop: document.getElementById('folder-pop'),
    folderPopPath: document.getElementById('folder-pop-path'),
    folderPopList: document.getElementById('folder-pop-list'),
    folderUse: document.getElementById('folder-use'),
    folderReset: document.getElementById('folder-reset'),
    folderPopInput: document.getElementById('folder-pop-input'),
    attach: document.getElementById('attach'),
    attachInput: document.getElementById('attach-input'),
    attachChips: document.getElementById('attach-chips'),
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

  // The conversation's text size: the one surface the owner reads at
  // length, so the one size that is theirs. A multiplier, not a pixel
  // step, so a reply, its headings and its code keep their ratios as they
  // grow; a step index is what is kept, so the scale can change without a
  // stored number landing between two steps.
  var CHAT_SCALES = [0.85, 1, 1.15, 1.3, 1.5, 1.7];
  var CHAT_SCALE_DEFAULT = 1;

  // Everything the header and the rail derive, in one place. `rows` is
  // the transcript; `events` is the raw log the inspector reads.
  var face = {
    session: '',
    // The messages typed while a run executes, in the owner's order:
    // one flushes per run, the head first.
    queued: [],
    // The conversation's text size, as an index into CHAT_SCALES. The
    // owner's own, and remembered; only the transcript reads it.
    chatStep: CHAT_SCALE_DEFAULT,
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
    // The archive groups by project; a folded group and a query are the
    // two ways to narrow 147 rows without losing the grouping.
    sessionFolded: {},
    // The return anchors (D6): events counted while reading backwards,
    // the seen stamp per conversation, and the divider's replay state.
    unread: 0,
    eventSeq: 0,
    dividerAt: null,
    dividerShown: false,
    // Turn navigation (D8): which turns are folded.
    turnFolded: {},
    sessionQuery: '',
    memoryFilter: 'live',
    skillsFilter: 'all',
    // The library holds 442 rows; the query and the show-all toggle are
    // what make a table of that length usable rather than a wall.
    skillQuery: '',
    skillShowAll: false,
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
    // Whether the dock was closed before an object opened it. Closing
    // the object then collapses the dock too, rather than leaving a
    // column the owner never asked for standing open.
    asideBeforeObject: false,
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
    // A run in flight somewhere, and the session it belongs to. The
    // bridge drives one session at a time; the two are kept apart so a
    // run in another conversation changes a row's words, not this
    // composer's.
    running: false,
    driving: '',
    dropped: 0,
    seat: null,
    seatKey: '',
    rules: [],
    feedback: {},
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
    // The composer's run policy: how this chat's tool calls are approved
    // and the provider it pins ('auto' is the empty string). Both ride
    // the next run and never a global config write.
    permissions: 'ask',
    providerPin: '',
    // Where this chat's runs work: `{path, name, rel}` when the owner
    // chose a folder, null for the launch folder. Rel is the pane's
    // rooted form the picker browses and the server accepts.
    folder: null,
    // The files this turn carries, as text. A browser can read the file
    // the owner picked; the content rides the run and clears when the
    // server takes it.
    attachments: [],
    providers: null,
    providersAt: 0,
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
    // The work list's own state: the search, the project groups the
    // owner folded, and the background families they opened.
    sessionQuery: '',
    sessionFolded: {},
    workExpanded: {},
    // The app's own background runs — scheduled jobs, delegated children,
    // peer conversations — are hidden by default: they keep the home
    // alive but they are not the owner's work. One line names them and
    // opens them on request.
    showBackground: false,
    // The work list folded to its strip: the owner's choice, kept for the
    // next reload the way the dock's is. The conversation takes the width
    // either way.
    workFolded: false,
    // The MCP scan's last answer, held for the settings card's discovery
    // half. In memory only: the scan is a point-in-time read of the
    // machine's other harnesses, and a reload drops it.
    serversScan: null,
    // The calibration panel's read and its clock.
    calibration: null,
    calibrationAt: 0,
    // The tower console's and the second-brain panel's reads and clocks.
    tower: null,
    towerAt: 0,
    brain: null,
    brainAt: 0,
    // The second-brain tool plugin's read: the tool's own address and
    // pipeline, separate from the state read because it crosses a
    // process boundary and a dead tool must not slow the state down.
    plugin: null,
    pluginAt: 0,
    pluginWasRunning: false,
    // The job the owner picked. It lives on the face rather than on the
    // select element because the card repaints on every poll: a selection
    // that reset mid-choice would send a different job than the one the
    // owner read, and `all` is a much longer run than most.
    pluginJob: '',
    // The atlas's own two choices: which attention class the rail is
    // filtered to, and whether the project list is showing every row.
    // Both survive a poll repaint for the same reason the job does.
    brainClass: '',
    brainShowAll: false,
    // The workspace-history section's reads and its two choices: which
    // branch the Files pane browses, and where in that branch's tree it
    // stands. Both survive a repaint because the read refreshes while
    // the owner navigates.
    history: null,
    historyAt: 0,
    historyBranch: '',
    historyPath: '',
    historyFiles: null,
    historyFilesAt: 0,
    // The settings surface's read and its clock.
    settings: null,
    settingsAt: 0,
    // The access read: paired devices beside the channel peers.
    access: null,
    accessAt: 0,
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
    if (face.stick) {
      el.log.scrollTop = el.log.scrollHeight;
      clearUnread();
    }
  }

  // The return anchors (D6): while the owner reads backwards, arriving
  // events count instead of yanking; the back-to-live control names the
  // count and returns on a press. The since-you-were-here divider marks
  // the first event after the last one the owner actually saw at the
  // bottom — per conversation, kept across visits.
  function clearUnread() {
    face.unread = 0;
    if (el.toLive) {
      el.toLive.hidden = true;
      el.toLive.textContent = '';
    }
    markSeen();
  }

  function paintToLive() {
    if (!el.toLive) return;
    el.toLive.hidden = !(face.unread > 0);
    el.toLive.textContent = '\u2193 ' + face.unread + ' new';
  }

  // One notification when the tab is hidden and the work needs the
  // owner (D6): a seat waits, or the run ended. Permission is asked
  // lazily on the first need and the feature degrades silently.
  function notifyIfHidden(title, body) {
    if (!document.hidden) return;
    try {
      if (typeof Notification === 'undefined') return;
      if (Notification.permission === 'default') {
        Notification.requestPermission();
        return;
      }
      if (Notification.permission === 'granted') {
        new Notification(title, { body: body });
      }
    } catch (error) { }
  }

  function markSeen() {
    if (!face.stick) return;
    if (!face.events.length) return;
    var last = face.events[face.events.length - 1];
    var stamp = last.ts || last.at || String(face.events.length);
    try {
      window.localStorage.setItem('fleet-seen-' + face.session, stamp);
    } catch (error) { }
  }

  // The divider's stamp for this conversation: the last event seen at
  // the bottom, or null when the owner never read this one live.
  function seenStamp() {
    try {
      return window.localStorage.getItem('fleet-seen-' + face.session);
    } catch (error) { return null; }
  }

  // Whether an event is the first after the seen stamp. The stamp rides
  // a sequence number when the event carries no clock: the count is
  // monotonic within a visit, which is all the divider needs.
  function pastSeen(event) {
    var stamp = face.dividerAt;
    if (stamp == null) return false;
    var seq = event.ts || event.at || null;
    if (seq != null) return String(seq) > String(stamp);
    return false;
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
    var made = { el: el0, event: event, update: update || null,
      turn: face.lastTurn || null };
    face.rows.push(made);
    face.stream.appendChild(el0);
    return made;
  }

  // One exchange is one owner message and the harness's answer to it —
  // the unit the conversation counts in. Steer frames and scheduled
  // prompts ride the log as user records, but a reseed turns them into
  // owner cards while the live stream never carried them: excluding
  // their frames keeps the count the same before and after a reload.
  function systemFrame(text) {
    var s = String(text || '');
    return s.indexOf('[the owner of this run steered it through its control file') === 0 ||
      s.indexOf('[scheduled run ') === 0;
  }
  function exchangesIn(list) {
    var n = 0;
    for (var i = 0; i < (list || []).length; i += 1) {
      var one = list[i];
      if (one && one.type === 'message' && one.role === 'owner' && !systemFrame(one.text)) n += 1;
    }
    return n;
  }
  function exchanges() { return exchangesIn(face.events); }

  // A message's own copy control, its label flip the visible trace —
  // an effect with no visible trace reads as broken.
  function makeCopy(getText, title) {
    var button = node('button', 'row-rewind msg-copy', 'copy');
    button.type = 'button';
    button.title = title;
    button.addEventListener('click', function () {
      if (navigator.clipboard) navigator.clipboard.writeText(getText()).catch(function () { });
      button.textContent = 'copied';
      window.setTimeout(function () { button.textContent = 'copy'; }, 1200);
    });
    return button;
  }

  // The reply's own controls: copy, the owner's verdict on this
  // exchange, a fork carrying this point's history, and a hook that
  // arms a scheduled job from this turn. Every verdict button names
  // its exchange — the same count the header carries — so the key
  // survives a reload and the pressed state can be repainted from the
  // state document.
  function assistantFooter(getText, ex) {
    var tools = node('div', 'msg-tools');
    tools.appendChild(makeCopy(getText, 'copy this reply'));
    var held = (face.feedback || {})[String(ex)] || '';
    var up = node('button', 'row-rewind msg-fb', held === 'up' ? 'liked' : 'like');
    up.type = 'button';
    up.title = 'this reply was right';
    up.dataset.exchange = String(ex);
    up.dataset.label = 'like';
    up.dataset.state = held === 'up' ? 'up' : '';
    up.addEventListener('click', function () { stampFeedback(ex, 'up'); });
    var down = node('button', 'row-rewind msg-fb', held === 'down' ? 'disliked' : 'dislike');
    down.type = 'button';
    down.title = 'this reply missed';
    down.dataset.exchange = String(ex);
    down.dataset.label = 'dislike';
    down.dataset.state = held === 'down' ? 'down' : '';
    down.addEventListener('click', function () { stampFeedback(ex, 'down'); });
    var fork = node('button', 'row-rewind msg-fork', 'fork');
    fork.type = 'button';
    fork.title = 'a new conversation carrying this reply’s history';
    fork.addEventListener('click', function () { forkFrom(ex); });
    var hook = node('button', 'row-rewind msg-hook', 'hook');
    hook.type = 'button';
    hook.title = 'arm a scheduled job from this turn';
    hook.addEventListener('click', function () { openHook(ex, ownerMessageFor(ex)); });
    tools.appendChild(up);
    tools.appendChild(down);
    tools.appendChild(fork);
    tools.appendChild(hook);
    return tools;
  }

  // One verdict, freshest wins: the same press clears, the other
  // press switches, and the buttons repaint from what the server read
  // back — the state document is the truth, the click is the request.
  function stampFeedback(ex, verdict) {
    var current = (face.feedback || {})[String(ex)];
    var next = current === verdict ? 'clear' : verdict;
    post('/api/feedback?session=' + encodeURIComponent(face.session || ''), {
      exchange: ex,
      verdict: next
    }).then(function (reply) {
      if (reply.status !== 200) {
        notice('the verdict did not land',
          (reply.body && reply.body.error) || 'the server refused it');
        return;
      }
      face.feedback = reply.body.feedback || {};
      paintFeedback();
    }).catch(function () {
      notice('the verdict did not land', 'the server did not answer');
    });
  }

  // The pressed state as the state document holds it: one verdict per
  // exchange, so only the button that verdict belongs to flips — its
  // label to the past tense, its state to the accent. The trace is
  // words as well as tint; an effect with no visible trace reads as
  // broken.
  function paintFeedback() {
    if (!face.stream || !face.stream.querySelectorAll) return;
    var held = face.feedback || {};
    face.stream.querySelectorAll('.msg-fb').forEach(function (button) {
      var label = button.dataset.label || '';
      var state = held[button.dataset.exchange] || '';
      var mine = (label === 'like' && state === 'up') ||
        (label === 'dislike' && state === 'down');
      button.dataset.state = mine ? state : '';
      button.textContent = mine
        ? (state === 'up' ? 'liked' : 'disliked')
        : label;
    });
  }

  // The owner message this exchange answered — a hook's task has to
  // stand alone in a fresh session, so the task is the ask itself.
  function ownerMessageFor(ex) {
    var count = 0;
    for (var i = 0; i < face.events.length; i += 1) {
      var one = face.events[i];
      if (one.type === 'message' && one.role === 'owner' && !systemFrame(one.text)) {
        count += 1;
        if (count === ex) return one.text || '';
      }
    }
    return '';
  }

  // The fork: a new conversation carrying this exchange's history.
  // The source log is left whole — the cut is the new file's own.
  function forkFrom(ex) {
    post('/api/session/fork', { exchange: ex }).then(function (reply) {
      if (reply.status !== 200 || !reply.body || !reply.body.id) {
        notice('the fork did not land',
          (reply.body && reply.body.error) || 'the server refused it');
        return;
      }
      notice('forked', 'the new conversation carries this reply’s history');
      openWork(reply.body.id);
    }).catch(function () {
      notice('the fork did not land', 'the server did not answer');
    });
  }

  // The hook as fleet can honestly arm it: a scheduled job seeded from
  // this turn. A job sees its task and nothing else, so the task is
  // the ask itself and the owner edits before arming — the write is
  // the settings surface's own cron op, and nothing runs in-loop.
  function openHook(ex, task) {
    var session = face.session || 'launch';
    var form = node('form', 'hook-form');
    var nameLabel = node('label', 'hook-label', 'name');
    var nameInput = node('input', 'hook-input');
    nameInput.value = 'hook-' + session.slice(-8) + '-' + ex;
    var scheduleLabel = node('label', 'hook-label', 'schedule');
    var scheduleInput = node('input', 'hook-input');
    scheduleInput.value = '0 9 * * *';
    var taskLabel = node('label', 'hook-label', 'task');
    var taskInput = node('textarea', 'hook-task');
    taskInput.value = task;
    var arm = node('button', 'row-rewind hook-arm', 'arm the job');
    arm.type = 'button';
    function armJob(event) {
      if (event && event.preventDefault) event.preventDefault();
      var body = {
        op: 'cron',
        name: (nameInput.value || '').trim(),
        schedule: (scheduleInput.value || '').trim(),
        task: taskInput.value
      };
      if (!body.name || !body.schedule || !body.task) {
        notice('the hook needs all three', 'a name, a schedule, and a task');
        return;
      }
      post('/api/settings', body).then(function (reply) {
        if (reply.status !== 200) {
          notice('the hook did not arm',
            (reply.body && reply.body.error) || 'the settings write refused it');
          return;
        }
        closeDrawer();
        notice('hooked · ' + body.name, 'runs on ' + body.schedule);
      }).catch(function () {
        notice('the hook did not arm', 'the server did not answer');
      });
    }
    arm.addEventListener('click', armJob);
    form.addEventListener('submit', armJob);
    form.appendChild(nameLabel);
    form.appendChild(nameInput);
    form.appendChild(scheduleLabel);
    form.appendChild(scheduleInput);
    form.appendChild(taskLabel);
    form.appendChild(taskInput);
    form.appendChild(arm);
    openDrawer('hook this turn', [
      ['the job', form],
      ['what it sees', node('p', 'derivation',
        'A scheduled job runs in its own session with this task and nothing else — write it to stand alone.')]
    ]);
  }

  function ownerRow(event) {
    var el0 = node('div', 'owner');
    el0.appendChild(node('span', 'owner-tag', 'you'));
    // The words render once, below: the session-ref branch and the plain
    // branch between them own the whole text. The original one-line body
    // appended the raw text here first, and survived the branches — so
    // every owner message printed its text twice.
    // The owner's own words are correctable in place (D3), the affordance
    // in the reply row's grammar: a quiet footer on the bubble itself,
    // visible without a hover — a hover-only control does not exist on
    // a touch face. Pressing edit fills the composer; sending rewinds to
    // the turn before the run those words launched and posts the edit as
    // a new run — the log keeps both, append-only.
    var text = event.text || '';
    var refs = face.sessionRefs || {};
    var tokens = Object.keys(refs).filter(function (token) { return text.indexOf(token) !== -1; });
    if (tokens.length) {
      var parts = text.split(tokens[0]);
      el0.appendChild(node('span', 'owner-text', parts[0]));
      var link = node('button', 'owner-session-ref', tokens[0]);
      link.type = 'button';
      link.title = 'open ' + refs[tokens[0]];
      link.addEventListener('click', function () { openWork(refs[tokens[0]]); });
      el0.appendChild(link);
      el0.appendChild(node('span', 'owner-text', parts.slice(1).join(tokens[0])));
    } else {
      el0.appendChild(node('span', 'owner-text', text));
    }
    var edit = node('button', 'row-rewind owner-edit', '\u270e edit');
    edit.type = 'button';
    edit.title = 'edit this message and re-run from before its run';
    edit.addEventListener('click', function () { startEdit(event); });
    var tools = node('div', 'msg-tools');
    tools.appendChild(edit);
    tools.appendChild(makeCopy(function () { return event.text || ''; }, 'copy this message'));
    el0.appendChild(tools);
    return row(el0, event);
  }

  // Begin an edit (D3). While a run drives, the edit queues with the
  // queue's own named reason rather than competing with the composer;
  // otherwise the composer fills with the message, and the send path
  // rewinds to the turn before the run the message launched.
  function startEdit(event) {
    var text = event.text || '';
    if (face.running || face.seat ||
        (face.driving && face.driving !== face.session)) {
      if (text.trim()) {
        face.queued.push(text);
        paintQueue();
      }
      return;
    }
    face.editing = { turn: turnAfter(event) };
    el.task.value = text;
    saveDraft();
    grow();
    el.task.focus();
    el.task.setSelectionRange(text.length, text.length);
  }

  // The first turn of the run this message launched, minus one: the
  // checkpoint that run started from. No turn after it — the message
  // never ran — and a first-ever turn both mean no rewind is needed;
  // the edit is simply the next run.
  function turnAfter(event) {
    var index = face.events.indexOf(event);
    if (index < 0) return null;
    for (var i = index + 1; i < face.events.length; i += 1) {
      if (face.events[i].type === 'turn') {
        var target = face.events[i].turn - 1;
        return target >= 1 ? target : null;
      }
    }
    return null;
  }

  function turnRow(event) {
    var el0 = node('div', 'turn-mark');
    el0.dataset.turn = String(event.turn);
    // The turn is a navigational unit (D8): the mark's own label is a
    // press that folds the turn's rows to the mark line, and the next
    // and previous marks are one press away from every mark. Nothing
    // auto-collapses — quiet by default is the rule the owner set.
    var fold = node('button', 'turn-fold', 'turn ' + event.turn);
    fold.type = 'button';
    fold.title = 'fold or unfold this turn\u2019s rows';
    fold.setAttribute('aria-expanded', 'true');
    fold.addEventListener('click', function () { toggleTurn(event.turn); });
    el0.appendChild(fold);
    var jumps = node('span', 'turn-jumps');
    var prev = node('button', 'turn-jump', '\u2191');
    prev.type = 'button';
    prev.title = 'the previous turn';
    prev.addEventListener('click', function () { jumpTurn(event.turn, -1); });
    var next = node('button', 'turn-jump', '\u2193');
    next.type = 'button';
    next.title = 'the next turn';
    next.addEventListener('click', function () { jumpTurn(event.turn, 1); });
    jumps.appendChild(prev);
    jumps.appendChild(next);
    el0.appendChild(jumps);
    // Checkpoints are per-turn (the agent captures after every tool
    // wave), so every turn mark is a rewind point: the affordance
    // rides the row on hover instead of a checkpoint card per N turns
    // (the contract, C3). Same endpoint, same confirm, same state the
    // old card flow landed.
    var rewind = node('button', 'row-rewind', '\u21a9 rewind');
    rewind.type = 'button';
    rewind.title = 'rewind the tree and the conversation to this turn';
    rewind.addEventListener('click', function () { rewindTo(event.turn); });
    // A rewind is refused while the run is in flight (the bridge's own
    // guard, and rightly so — the run's state is live). The affordance
    // waits for the end rather than inviting a click that cannot land:
    // hidden now, unhidden by the run's end.
    if (face.running) {
      rewind.hidden = true;
      rewind.dataset.live = 'true';
    }
    el0.appendChild(rewind);
    return row(el0, event);
  }

  // Fold one turn (D8): the rows between this mark and the next hide,
  // the mark stays. Rows are tagged with their turn as they are made.
  function toggleTurn(turn) {
    var folded = !face.turnFolded[turn];
    face.turnFolded[turn] = folded;
    paintTurnFold(turn);
  }

  function paintTurnFold(turn) {
    var folded = !!face.turnFolded[turn];
    face.rows.forEach(function (one) {
      if (one.turn === turn && one.el &&
          !one.el.className.includes('turn-mark')) {
        one.el.hidden = folded;
      }
    });
    var mark = face.rows.filter(function (one) {
      return one.turn === turn && one.el && one.el.className.includes('turn-mark');
    })[0];
    if (mark) {
      var fold = mark.el.querySelector('.turn-fold');
      if (fold) fold.setAttribute('aria-expanded', folded ? 'false' : 'true');
    }
  }

  // Jump to the next or previous turn mark from `turn`.
  function jumpTurn(turn, step) {
    var marks = face.rows
      .filter(function (one) { return one.el && one.el.className.includes('turn-mark'); })
      .map(function (one) { return one.turn; })
      .sort(function (a, b) { return a - b; });
    var at = marks.indexOf(turn);
    var to = marks[Math.min(marks.length - 1, Math.max(0, at + step))];
    if (to == null) return;
    var target = face.rows.filter(function (one) { return one.turn === to && one.el; })[0];
    if (target && target.el.scrollIntoView) target.el.scrollIntoView({ block: 'start' });
  }

  // Expand this run: every folded turn of the current run unfolds at
  // once — the audit pass's one press.
  function expandRun() {
    Object.keys(face.turnFolded).forEach(function (turn) {
      face.turnFolded[turn] = false;
      paintTurnFold(Number(turn));
    });
  }

  // One thought block, closed: a row that says how long it took and opens
  // its words on a press. The reasoning is display material — the log
  // holds it, the provider never sees it again.
  function thoughtRow(event) {
    var el0 = node('div', 'thought');
    var head = node('button', 'thought-head');
    head.type = 'button';
    head.setAttribute('aria-expanded', 'false');
    head.appendChild(node('span', 'thought-glyph', '\u25c7'));
    head.appendChild(node('span', 'thought-label',
      'Thought' + (event.seconds != null ? ' \u00b7 ' + event.seconds + 's' : '')));
    var body = node('div', 'thought-body');
    body.hidden = true;
    if (event.text) body.appendChild(markdown(event.text));
    head.addEventListener('click', function () {
      body.hidden = !body.hidden;
      head.setAttribute('aria-expanded', body.hidden ? 'false' : 'true');
      el0.classList.toggle('open', !body.hidden);
      settle();
    });
    el0.appendChild(head);
    el0.appendChild(body);
    return row(el0, event);
  }

  function assistantRow() {
    var el0 = node('div', 'assistant streaming');
    return row(el0, null, function (text) {
      el0.textContent += text;
    });
  }

  // A reply renders as markdown when its block closes: streaming stays
  // plain text (the cheapest thing to append per frame), and the block
  // boundary is where the whole answer is in hand — a fenced code block
  // can only be recognised once its fence has arrived.
  function closeAssistant() {
    var open = face.assistant;
    face.assistant = null;
    if (!open || !open.el) {
      face.buffer = '';
      return;
    }
    var el0 = open.el;
    var text = (el0.textContent || '') + (face.buffer || '');
    face.buffer = '';
    el0.classList.remove('streaming');
    el0.classList.add('md-rendered');
    el0.textContent = '';
    if (text) el0.appendChild(markdown(text));
    el0.appendChild(assistantFooter(function () { return text; }, exchanges()));
  }

  // The value a call actually names — the command, the path, the content.
  // The raw arguments ride the event so an expanded row shows the whole
  // text rather than the clipped line.
  function toolArgumentText(event) {
    var raw = event.arguments;
    if (!raw) return '';
    try {
      var parsed = JSON.parse(raw);
      var value = parsed.command || parsed.path || parsed.content || parsed.name;
      if (typeof value === 'string') return value;
    } catch (error) { }
    return String(raw);
  }

  function toolRow(event) {
    var status = node('span', 'status', 'running \u00b7 0s');
    var el0 = card('tool', GLYPHS[event.tool] || event.tool.charAt(0).toUpperCase(),
      event.tool, event.summary, null, null);
    el0.classList.add('tool');
    el0.dataset.tool = event.tool;
    el0.dataset.status = 'running';
    el0.querySelector('.body').appendChild(status);
    // The expanded detail: what ran, how long, what came back, and the
    // raw record one more press away. Closed by default, one line while
    // closed.
    var detail = node('div', 'tool-detail');
    detail.hidden = true;
    var command = toolArgumentText(event);
    if (command) detail.appendChild(node('pre', 'tool-command', command));
    var facts = node('p', 'tool-facts', 'running');
    detail.appendChild(facts);
    var output = node('pre', 'tool-output');
    output.hidden = true;
    detail.appendChild(output);
    var raw = node('button', 'tool-raw', 'raw record');
    raw.type = 'button';
    raw.addEventListener('click', function (stop) {
      if (stop && stop.stopPropagation) stop.stopPropagation();
      openDrawer('tool \u00b7 ' + event.tool, [
        ['the call', node('pre', 'raw', JSON.stringify(event, null, 2))],
        ['the result', node('pre', 'raw', JSON.stringify(made.event || {}, null, 2))]
      ]);
    });
    detail.appendChild(raw);
    // The re-run (D11): one press through the executor — the same bound,
    // the same approval classes — with the answer in the drawer, never
    // in the conversation the model reads. The write class is refused
    // by name on the server; the arguments are the recorded ones.
    var rerun = node('button', 'tool-raw', 're-run');
    rerun.type = 'button';
    rerun.title = 'run this call again through the tools\u2019 own executor; the answer stays in the drawer';
    rerun.addEventListener('click', function (stop) {
      if (stop && stop.stopPropagation) stop.stopPropagation();
      var args = event.arguments && event.arguments.trim()
        ? event.arguments : '{}';
      post('/api/replay', { name: event.tool, arguments: args })
        .then(function (reply) {
          if (reply.status !== 200) {
            notice('the re-run was refused',
              (reply.body && reply.body.error) || 'the face said no without a reason');
            return;
          }
          if (reply.body.seat) {
            notice('the seat above is the re-run',
              'answer it and the call runs; the answer stays in the drawer');
            return;
          }
          var out = node('pre', 'raw',
            (reply.body.output || '') + (reply.body.truncated ? '\n(cut at the bound)' : ''));
          openDrawer('re-run \u00b7 ' + event.tool, [
            ['the output', out],
            ['never logged', node('p', 'derivation',
              'this result did not enter the conversation the model reads')]
          ]);
        })
        .catch(function () { });
    });
    detail.appendChild(rerun);
    el0.appendChild(detail);
    el0.querySelector('.card-open').addEventListener('click', function () {
      detail.hidden = !detail.hidden;
      el0.classList.toggle('open', !detail.hidden);
      settle();
    });
    // The live seconds: informative loading (the contract, C2), not a
    // spinner - the number is the truth about how long the call has
    // run, and it stops the moment the answer lands.
    var started = Date.now();
    var tick = setInterval(function () {
      status.textContent = 'running \u00b7 ' + Math.round((Date.now() - started) / 1000) + 's';
    }, 1000);
    var made = row(el0, event, function (result) {
      clearInterval(tick);
      var took = Math.round((Date.now() - started) / 1000) + 's';
      // How the call was resolved, when the run recorded it: the method
      // on the collapsed line, the whole reason in the detail.
      var method = result.resolution ? result.resolution.split(' \u00b7 ')[0] : null;
      el0.dataset.status = result.approved ? 'ok' : 'refused';
      status.textContent = method ? took + ' \u00b7 ' + method : took;
      facts.textContent = took + ' \u00b7 ' + (result.approved ? 'ran' : 'not run · refused') +
        (result.resolution ? ' \u00b7 ' + result.resolution : '');
      if (result.preview) {
        output.textContent = result.preview;
        output.hidden = false;
      }
      if (result.approved) {
        // Done collapses: one line, outcome dot in the css, the command
        // and its output one press away.
        el0.classList.add('collapsed');
      } else {
        // A refused call stays expanded - the owner's decision should
        // never hide, and the reason is the reading.
        detail.hidden = false;
        el0.classList.add('open');
      }
      made.event = { type: 'tool_result', tool: result.tool, preview: result.preview, approved: result.approved,
        resolution: result.resolution };
    });
    face.pending.push(made);
    return made;
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

  // The run's one plumbing line. The events that carry no work of
  // their own - routing, memory, skills, lanes, cron, compaction,
  // checkpoints, folder changes, repair, rewinds, the log's drops -
  // fold into this strip instead of each rendering a card (the
  // chat-UX contract, D1): the transcript carries the work and the
  // words, and the harness tells its story one click away. The count
  // is the strip's honest weight; replay and live share apply(), so
  // a replayed history folds the same way the live run did.
  // A deck the run produced: work, so it stays a card (D1), one line
  // with the render a click away in the drawer. KI-292: this case
  // called a function the file never carried, so every deck event
  // threw and the transcript stopped at the throw - rebuilt on
  // inspectDrawer's own inspector.
  function deckRow(event) {
    var el0 = card('deck', '\u25a6', event.project + ' \u00b7 ' + event.nodes + ' nodes',
      'the render opens from the card', null, 'deck');
    el0.querySelector('.card-open').addEventListener('click', function () {
      inspectDeck(event);
    });
    return row(el0, event);
  }

  // One file handed to the owner. The card is the delivery: the name and
  // size are on it, and opening it puts the file in the pane the way any
  // other path does.
  function artifactRow(event) {
    var el0 = card('artifact', '\u25c7', event.path,
      (event.note ? event.note + ' \u00b7 ' : '') + bytes(event.size || 0),
      null, 'open');
    el0.querySelector('.card-open').addEventListener('click', function () {
      openFile(event.path);
    });
    return row(el0, event);
  }

  var STRIP_LABEL = {
    routed: 'route', memory: 'memory', skills: 'skills', lane: 'lane',
    cron: 'cron', compacted: 'compaction', checkpoint: 'checkpoint',
    folder: 'folder', repair: 'repair', rewound: 'rewind', dropped: 'dropped'
  };

  function stripFold(event) {
    if (face.strip && face.strip.event.type === event.type) {
      face.strip.count += 1;
      // The expanded list shows the run's latest per kind - a folded
      // pair of checkpoints reads turn 20, not the first one it saw -
      // and carries the turn so the line's rewind affordance has a
      // destination.
      var last = face.stripLog[face.stripLog.length - 1];
      if (last && last.kind === (STRIP_LABEL[event.type] || event.type)) {
        last.text = stripLine(event);
        last.turn = event.turn || null;
      }
    } else {
      if (face.strip) stripSeal();
      face.strip = { event: event, count: 1 };
    }
    stripPaint();
  }

  function stripLine(event) {
    switch (event.type) {
      case 'routed': return (event.how || '') + (event.provider ? ' \u00b7 ' + event.provider : '');
      case 'memory': return (event.facts != null ? event.facts + ' facts' : '') + (event.shape ? ' \u00b7 ' + event.shape : '');
      case 'skills': return event.skipped && event.skipped.length
        ? 'skipped ' + event.skipped.join(', ') : 'library loaded';
      case 'lane': return (event.lane || '') + (event.tools != null ? ' \u00b7 ' + event.tools + ' tools' : '');
      case 'cron': return (event.job || '') + (event.outcome ? ' \u00b7 ' + event.outcome : '');
      case 'compacted': return (event.turns || '?') + ' turns \u00b7 ' + tokens(event.before) + ' \u2192 ' + tokens(event.after);
      case 'checkpoint': return 'turn ' + event.turn + ' \u00b7 ' + event.files + ' files held';
      case 'folder': return event.note || (event.path ? 'set to ' + event.path : 'cleared');
      case 'goal': return event.text ? 'goal \u00b7 ' + event.text : 'goal cleared';
      case 'control':
        if (event.control === 'fold') return 'fold armed at the next boundary';
        if (event.control === 'pause') return 'paused by you';
        if (event.control === 'resume') return 'resumed';
        if (event.control === 'pin') return 'pinned \u00b7 ' + ((event.body && event.body.pin) || '');
        if (event.control === 'steer') return 'steer \u00b7 ' + ((event.body && event.body.steer) || '');
        if (event.control === 'stop') return 'stopped by you';
        return 'control \u00b7 ' + event.control;
      case 'repair': return 'asked once for the closing report (attempt ' + (event.attempt || 1) + ')';
      case 'rewound': return 'to turn ' + event.turn + ' \u00b7 ' + event.restored + ' restored';
      case 'dropped': return 'the face holds the last 4096; one fell off the front';
      default: return event.type;
    }
  }

  function stripPaint() {
    if (!face.stripEl) {
      var el0 = node('div', 'run-strip');
      var head = node('button', 'strip-head');
      head.type = 'button';
      head.appendChild(node('span', 'strip-glyph', '\u2261'));
      head.appendChild(node('span', 'strip-text', ''));
      head.appendChild(node('span', 'strip-count', ''));
      var list = node('div', 'strip-list');
      list.hidden = true;
      el0.appendChild(head);
      el0.appendChild(list);
      head.addEventListener('click', function () {
        list.hidden = !list.hidden;
        el0.classList.toggle('open', !list.hidden);
        if (!list.hidden) stripList(el0);
      });
      face.stripEl = el0;
      face.stream.appendChild(el0);
    }
    // The head names the segment the run is in (or the last sealed one
    // when the run has moved on), and the count chip is the honest
    // total of everything the strip holds.
    var held = face.strip;
    var label = held
      ? (STRIP_LABEL[held.event.type] || held.event.type)
      : (face.stripLog.length ? face.stripLog[face.stripLog.length - 1].kind : '');
    var detail = held && held.event.type === 'routed' && held.event.provider
      ? held.event.provider : '';
    var total = face.stripLog.length + (held ? 1 : 0);
    face.stripEl.querySelector('.strip-text').textContent =
      'run' + (label ? ' \u00b7 ' + label : '') + (detail ? ' \u00b7 ' + detail : '');
    face.stripEl.querySelector('.strip-count').textContent = total > 1 ? String(total) : '';
  }

  // A different event kind arrived, or the run ended: the held run
  // becomes one line in the strip's expanded list, re-rendered from
  // face.stripLog - the folded record, not the live stream - so
  // expansion is stable however many events followed.
  function stripSeal() {
    var held = face.strip;
    face.stripLog.push({
      kind: STRIP_LABEL[held.event.type] || held.event.type,
      count: held.count,
      text: stripLine(held.event),
      turn: held.event.type === 'checkpoint' ? held.event.turn : null,
      event: held.event
    });
    face.strip = null;
    stripPaint();
    stripList(face.stripEl);
  }

  function stripList(el0) {
    if (!el0) return;
    var list = el0.querySelector('.strip-list');
    list.textContent = '';
    face.stripLog.forEach(function (one) {
      var line = node('div', 'strip-line');
      // Every folded line opens its record: the same click-to-see-more
      // the transcript's rows have, so no action is a dead label.
      line.setAttribute('role', 'button');
      line.tabIndex = 0;
      var open = function (stop) {
        if (stop && stop.target && stop.target.closest && stop.target.closest('.row-rewind')) return;
        openDrawer(one.kind, [
          ['the record', node('pre', 'raw', JSON.stringify(one.event || {}, null, 2))]
        ]);
      };
      line.addEventListener('click', open);
      line.addEventListener('keydown', function (event) {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault();
          open(event);
        }
      });
      line.appendChild(node('span', 'strip-line-kind', one.kind));
      line.appendChild(node('span', 'strip-line-text',
        one.text + (one.count > 1 ? ' \u00b7 \u00d7' + one.count : '')));
      if (one.turn != null) {
        var rewind = node('button', 'row-rewind', '\u21a9 rewind');
        rewind.type = 'button';
        rewind.title = 'rewind the tree and the conversation to turn ' + one.turn;
        rewind.addEventListener('click', function () { rewindTo(one.turn); });
        line.appendChild(rewind);
      }
      list.appendChild(line);
    });
  }

  // The run's end (or the owner's stop) seals whatever the strip held.
  function stripFinish() {
    if (face.strip) stripSeal();
    if (face.stripEl) face.stripEl.classList.add('done');
  }

  function endRow(event) {
    // The core reports an owner's stop through `failed`; the reason is
    // the only thing that separates it from a failure, and calling the
    // owner's own decision a failure would be the wrong card.
    var stopped = event.type === 'failed' && /stopped by the owner/i.test(event.reason || '');
    var failed = event.type === 'failed' && !stopped;
    var capped = event.reason === 'max-turns' && face.turn > 0;
    // The continuation receipt (the contract, D4): reaching the cap is
    // not a dead stop - the card states where the run stands, and the
    // continue control resumes this session with a hand-off that names
    // the budget, so the model picks the work back up with its state
    // carried. The hand-off is seeded from the transcript the face
    // holds; the run's own report may name it better, and a real final
    // message wins when the event carries one.
    var el0 = card(failed ? 'failed' : 'finished', stopped ? '\u25a0' : failed ? '!' : '\u2713',
      stopped ? 'stopped by you' : failed ? 'failed' : capped ? 'the turn budget ran out'
        : 'finished', event.reason, null, null);
    if (!stopped && !failed) {
      paintReceipt(el0, event);
    }
    if (capped) {
      var note = el0.querySelector('.note');
      note.hidden = false;
      note.textContent = 'the run reached ' + face.turn + ' turns - its state is carried in ' +
        'this session. Continue and it picks the work back up; raise the budget first in ' +
        'settings (runs.max_turns) if the task needs more room.';
      var actions = node('div', 'receipt-actions');
      var go = node('button', 'replay-button receipt-continue', 'continue the work');
      go.type = 'button';
      go.addEventListener('click', function () {
        go.disabled = true;
        go.textContent = 'continuing\u2026';
        var runPath = '/api/run' + (face.session
          ? '?session=' + encodeURIComponent(face.session) : '');
        post(runPath, {
          task: 'continue the previous task from where the turn budget stopped it. ' +
            'The conversation above carries your state; finish the remaining work and ' +
            'close with the result.',
          mode: face.permissions,
          pin: face.providerPin || null,
          budget_turns: face.turnBudget || null
        }).then(function (reply) {
          if (reply.status === 200) {
            face.running = true;
            face.stop = null;
            resetProcess(face.planMode ? 'plan' : 'run');
            paint();
            return;
          }
          go.disabled = false;
          go.textContent = 'continue the work';
          notice('the continue was refused',
            (reply.body && reply.body.error) || 'the face said no without a reason');
        });
      });
      actions.appendChild(go);
      el0.appendChild(actions);
    }
    return row(el0, event);
  }

  // The end receipt (the contract, D5): outcome, artifacts,
  // verification evidence, and the ask. Everything it renders is read
  // from face.events - the same record replay feeds - so a receipt is
  // true on a live finish and identical on a replayed one. The closing
  // report is the token stream after the last tool result; on a replay
  // it is the assistant message event instead. A finish with no report
  // got the repair beat's ask already (the strip names it); what the
  // model answered to that ask is this same tail.
  // The receipt's artifacts (D7): the diff against the checkpoint
  // before this run's first turn names the files the run touched; each
  // row opens that diff in the drawer. No prior checkpoint (a first
  // run) measures against the run's own last checkpoint, which names
  // everything the session changed — the honest answer for a session
  // with no before.
  function appendArtifacts(body) {
    if (!face.session) return;
    var session = face.session;
    fetch('/api/checkpoints', { headers: { accept: 'application/json' } })
      .then(function (reply) { return reply.json(); })
      .then(function (doc) {
        var snapshots = doc.snapshots || [];
        if (!snapshots.length) return;
        var turns = snapshots.map(function (one) { return one.turn; }).sort(function (a, b) { return a - b; });
        var first = face.runFirstTurn || 0;
        var before = null;
        for (var i = 0; i < turns.length; i += 1) {
          if (turns[i] < first) before = turns[i];
        }
        var turn = before != null ? before : turns[turns.length - 1];
        return fetch('/api/diff?turn=' + turn, { headers: { accept: 'application/json' } })
          .then(function (reply) { return reply.json(); })
          .then(function (diff) {
            var files = (diff.files || []).map(function (one) { return one.path; });
            if (!files.length) return;
            var line = node('p', 'receipt-line');
            line.appendChild(document.createTextNode('artifacts \u00b7 ' + files.length +
              (files.length === 1 ? ' file' : ' files') + ' \u00b7 '));
            files.slice(0, 5).forEach(function (path, at) {
              if (at) line.appendChild(document.createTextNode(', '));
              var open = node('button', 'receipt-file', path);
              open.type = 'button';
              open.title = 'the diff for ' + path;
              open.addEventListener('click', function () { openArtifactsDiff(turn); });
              line.appendChild(open);
            });
            if (files.length > 5) {
              line.appendChild(document.createTextNode(' \u00b7 ' + (files.length - 5) + ' more'));
            }
            body.appendChild(line);
          });
      })
      .catch(function () { });
  }

  // The artifacts' diff in the drawer: the run's whole diff against the
  // checkpoint, the review's own painter.
  // The context inspector (D10): the session's log reconstructed into
  // the request its run sent — the system prompt, the message list, and
  // the tool declarations — read-only, in the drawer.
  function openContext() {
    if (!face.session) return;
    fetch('/api/context?session=' + encodeURIComponent(face.session),
      { headers: { accept: 'application/json' } })
      .then(function (reply) { return reply.json(); })
      .then(function (doc) {
        if (doc.refused && doc.refused.length) {
          notice('the context did not read', doc.refused.join(' \u00b7 '));
          return;
        }
        var sections = [];
        var systemHead = node('p', 'derivation',
          (doc.system || '').length + ' characters \u00b7 the system prompt the model received');
        sections.push(['the system prompt', function () {
          var box = node('div', 'context-system');
          box.appendChild(systemHead);
          var text = node('pre', 'context-pre', doc.system || '');
          text.hidden = true;
          var toggle = node('button', 'receipt-toggle', 'show the prompt');
          toggle.type = 'button';
          toggle.addEventListener('click', function () {
            text.hidden = !text.hidden;
            toggle.textContent = text.hidden ? 'show the prompt' : 'hide the prompt';
          });
          var wrap = node('div');
          wrap.appendChild(toggle);
          wrap.appendChild(text);
          box.appendChild(wrap);
          return box;
        }()]);
        sections.push(['the messages', function () {
          var list = node('div', 'context-messages');
          (doc.messages || []).forEach(function (message) {
            var line = node('div', 'context-message');
            line.appendChild(node('span', 'context-role', String(message.role || '')));
            var content = typeof message.content === 'string'
              ? message.content : JSON.stringify(message.content);
            line.appendChild(node('span', 'context-text', content));
            list.appendChild(line);
          });
          return list;
        }()]);
        sections.push(['the tools', function () {
          var list = node('div', 'context-tools');
          (doc.tools || []).forEach(function (tool) {
            var name = tool && tool.function && tool.function.name;
            list.appendChild(node('span', 'context-tool', String(name || '')));
          });
          return list;
        }()]);
        openDrawer('what the model saw', sections);
      })
      .catch(function () { });
  }

  function openArtifactsDiff(turn) {
    fetch('/api/diff?turn=' + turn, { headers: { accept: 'application/json' } })
      .then(function (reply) { return reply.json(); })
      .then(function (diff) { paintDiff(diff, turn); })
      .catch(function () { });
  }

  function paintReceipt(el0, event) {
    var events = face.events;
    var calls = {};
    var decks = 0;
    events.forEach(function (one) {
      if (one.type === 'tool_call') {
        calls[one.tool] = (calls[one.tool] || 0) + 1;
      }
      if (one.type === 'deck') decks += 1;
    });
    var body = node('div', 'receipt');
    var bits = [];
    var tally = Object.keys(calls).map(function (tool) {
      return calls[tool] + ' ' + tool;
    }).join(', ');
    if (tally) bits.push(tally);
    if (decks) bits.push(decks + (decks === 1 ? ' deck rendered' : ' decks rendered'));
    if (bits.length) {
      body.appendChild(node('p', 'receipt-line',
        'work \u00b7 ' + bits.join(' \u00b7 ')));
    }
    var evidence = [];
    var ex = exchanges();
    if (ex) evidence.push(ex + (ex === 1 ? ' exchange' : ' exchanges'));
    else if (face.turn) evidence.push(face.turn + (face.turn === 1 ? ' turn' : ' turns'));
    if (face.spend) evidence.push(tokens(face.spend) + ' tokens');
    if (face.context && face.budget) {
      evidence.push(Math.min(100, Math.round((face.context / face.budget) * 100)) +
        '% of the window');
    }
    if (evidence.length) {
      body.appendChild(node('p', 'receipt-line',
        'evidence \u00b7 ' + evidence.join(' \u00b7 ')));
    }
    // What the model saw (D10): the run's request reconstructed from
    // the log, in the drawer. The receipt is where the audit begins.
    var saw = node('button', 'receipt-toggle', 'what the model saw');
    saw.type = 'button';
    saw.title = 'the exact request the run sent: system, messages, tools';
    saw.addEventListener('click', openContext);
    body.appendChild(saw);
    // The audit pass's one press (D8): every folded turn unfolds.
    if (Object.keys(face.turnFolded).some(function (turn) { return face.turnFolded[turn]; })) {
      var expand = node('button', 'receipt-toggle', 'expand this run');
      expand.type = 'button';
      expand.addEventListener('click', expandRun);
      body.appendChild(expand);
    }
    // The artifacts (D7): the files this run's checkpoint window holds,
    // each opening the diff that checkpoint names. The read is the
    // checkpoints' own; a run that touched nothing shows nothing.
    appendArtifacts(body);
    // The closing report: the tokens after the last tool result, or
    // the replayed assistant message when there are no tokens.
    var said = '';
    var lastWork = -1;
    for (var i = events.length - 1; i >= 0; i -= 1) {
      var kind = events[i].type;
      if (kind === 'tool_call' || kind === 'tool_result') {
        lastWork = i;
        break;
      }
    }
    // Only what the run said after its last work: an unbounded walk
    // would cross a tool boundary and quote the pre-work prose - a
    // report the run never closed with.
    var tokensTail = [];
    for (var i = events.length - 1; i > lastWork; i -= 1) {
      var one = events[i];
      if (one.type === 'message' && one.role === 'assistant' && one.text) {
        said = one.text;
        break;
      }
      if (one.type === 'token') tokensTail.unshift(one.text);
    }
    if (!said && tokensTail.length) said = tokensTail.join('');
    if (said) {
      var report = node('div', 'receipt-report');
      var label = node('button', 'receipt-toggle', 'the closing report');
      label.type = 'button';
      var bodyText = node('div', 'receipt-report-text');
      bodyText.appendChild(markdown(said));
      bodyText.hidden = true;
      label.addEventListener('click', function () {
        bodyText.hidden = !bodyText.hidden;
        label.textContent = bodyText.hidden ? 'the closing report' : 'hide the report';
      });
      report.appendChild(label);
      report.appendChild(bodyText);
      body.appendChild(report);
    }
    // The ask: one click fills the composer with the follow-up the
    // report's own contract promises - never sent for the owner.
    var actions = node('div', 'receipt-actions');
    var ask = node('button', 'replay-button', 'ask what is next');
    ask.type = 'button';
    ask.addEventListener('click', function () {
      ask('The run above just finished. What is the one next step you would ' +
        'take from here, and what do you need from me for it?');
    });
    actions.appendChild(ask);
    body.appendChild(actions);
    var note = el0.querySelector('.note');
    note.hidden = false;
    note.textContent = '';
    note.appendChild(body);
  }

  // The log is a ring, and the ring let one go. The bridge says so once,
  // when it first happens; after that the rail's own dropped count is
  // the standing answer, which is why this row is not repeated.
  // The chat's working folder, as a line in the transcript: a chosen
  // folder, a reset to the launch folder, or the fallback a run took
  // when the stored folder had gone missing. The note is the server's
  // own sentence, so the page does not paraphrase a reason it did not
  // measure.
  // The run stopped without the report it owes and the harness asked
  // once. One line, so the extra request is seen and paid for knowingly.
  // The answer that closed a seat: one line in the transcript, so the
  // grant is seen where it happened and the log holds what the owner
  // allowed. The card itself closes; a seat for the next call is a new
  // card, not this one changing under the owner's eyes — which is what
  // made the popup read as "it never closes" (KI-283).
  function approvalRow(event) {
    var allowed = event.decision !== 'deny';
    var word = event.decision === 'deny' ? 'refused this call'
      : event.decision === 'always' ? 'allowed for the rest of this run'
      : event.decision === 'remember' ? 'allowed and remembered'
      : 'allowed this call';
    var what = event.tool || 'the call';
    var el0 = card('approval', allowed ? '\u2713' : '\u2715', word,
      what + (event.summary ? ' \u00b7 ' + event.summary : ''), null, null);
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
    // Every event names the session it belongs to. One conversation's
    // words never land in another's transcript: a background run stays a
    // row in the work list until the owner opens it, and the row's state
    // refreshes when a seat or an ending arrives for another session.
    // The exception is a delegation this conversation's own run raised:
    // it names its CHILD in `session` — the branch key — and carries the
    // parent in `parent`, so the card and the branch lines pass while
    // another conversation's children still do not.
    var ownChild = event.type === 'delegation' && event.parent &&
      event.parent === face.session;
    if (!ownChild && event.session && face.session && event.session !== face.session) {
      if (event.type === 'finished' || event.type === 'failed') {
        // That run is over. A message queued in this conversation was
        // waiting on it, so the queue drains now — the send names this
        // session, and the run it starts is this one's.
        face.driving = '';
        face.sessions = null;
        readSessions(true);
        if ((face.queued || []).length && !face.running && !face.seat) flushQueue();
      } else if (event.type === 'seat' || event.type === 'seat_cleared' ||
                 event.type === 'answered') {
        face.sessions = null;
        readSessions(false);
      }
      return;
    }
    face.eventSeq = (face.eventSeq || 0) + 1;
    // The divider lands before the first event past the seen stamp
    // (D6), once per replay.
    if (face.dividerAt != null && !face.dividerShown &&
        face.eventSeq === face.dividerAt + 1) {
      face.dividerShown = true;
      var divider = node('div', 'since-divider', '\u2014 since you were here \u2014');
      face.stream.appendChild(divider);
    }
    face.events.push(event);
    // Reading backwards: count instead of yank (D6).
    if (!face.stick && face.unread != null) {
      face.unread += 1;
      paintToLive();
    }
    switch (event.type) {
      case 'session':
        // The live session the run created: the face's reads, its answers
        // and its runs all name it, so the reducer has to learn it as it
        // happens. Without this the face kept whatever session the last
        // state load anchored — and the answered check compared the
        // seat's session against that stale value, leaving the card
        // standing (KI-283's regression).
        if (event.id) face.session = event.id;
        break;
      case 'message':
        if (event.role === 'owner') {
          face.running = true;
          face.stop = null;
          ownerRow(event);
          break;
        }
        // The replay's shape for the assistant: the live path streams
        // the words as tokens into the open row and never emits this
        // card, so on a replay — where there are no tokens — the card
        // has to build the row, or a resumed history shows the owner's
        // questions with the answers missing between them.
        if (!face.assistant && event.text) {
          var said = node('div', 'assistant md-rendered');
          said.appendChild(markdown(event.text));
          said.appendChild(assistantFooter(function () { return event.text; }, exchanges()));
          row(said, event);
        }
        break;
      case 'reasoning':
        thoughtRow(event);
        break;
      case 'token':
        if (!face.assistant) face.assistant = assistantRow();
        face.buffer += event.text;
        schedule();
        break;
      case 'turn':
        face.running = true;
        if (!face.runFirstTurn) face.runFirstTurn = event.turn;
        if (event.turn !== face.lastTurn) {
          face.lastTurn = event.turn;
          face.turn = event.turn;
          turnRow(event);
          paintRunbar();
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
        closeAssistant();
        toolRow(event);
        break;
      case 'tool_result':
        resolveTool(event);
        break;
      case 'routed':
        face.route = event;
        stripFold(event);
        break;
      case 'lane':
        stripFold(event);
        break;
      case 'memory':
        stripFold(event);
        break;
      case 'skills':
        stripFold(event);
        break;
      case 'deck':
        deckRow(event);
        break;
      case 'artifact':
        closeAssistant();
        artifactRow(event);
        break;
      case 'delegation':
        applyDelegation(event);
        break;
      case 'compacted':
        stripFold(event);
        break;
      case 'cron':
        stripFold(event);
        break;
      case 'checkpoint':
        stripFold(event);
        break;
      case 'goal':
        // The session's standing objective changed: the run strip
        // records the line, and the run bar's chip names it for the
        // next run.
        face.goal = event.text || null;
        stripFold(event);
        paintRunbar();
        break;
      case 'folder':
        // The chat's working folder changed: one line, and the run bar
        // now says where the next run works. The event is the truth in
        // both directions, so a change made from a phone lands here too.
        face.folder = event.path
          ? { path: event.path, name: event.name, rel: event.rel || '' }
          : null;
        stripFold(event);
        paintRunbar();
        break;
      case 'rewound':
        stripFold(event);
        break;
      case 'repair':
        // The harness asked for the report it was owed. The fold keeps
        // the extra call visible - in the strip, where the run's
        // plumbing lives - rather than a turn that seemed to do nothing.
        stripFold(event);
        break;
      case 'plan':
        planRow(event);
        break;
      case 'finished':
      case 'failed':
        face.runFirstTurn = 0;
        notifyIfHidden('fleet \u00b7 the run ' + (event.type === 'finished' ? 'finished' : 'failed'),
          event.stop_reason || 'the outcome is in the conversation');
        closeAssistant();
        face.running = false;
        // The run this face was executing is over: no conversation is
        // driving, so the next send anywhere starts a run rather than
        // queueing behind one that has ended.
        if (face.driving && (!event.session || event.session === face.driving)) {
          face.driving = '';
        }
        face.stop = event;
        stripFinish();
        endRow(event);
        // A plan-mode run's answer is a plan: the card it lands in is
        // the editable one, so the owner edits and builds.
        if (face.planned && event.type === 'finished') {
          face.planned = false;
          finishPlan();
        }
        // The turns a live run painted hid their rewind; the run is
        // over, so the affordance they describe is real again.
        Array.prototype.forEach.call(
          document.querySelectorAll('.row-rewind[data-live]'),
          function (button) { button.hidden = false; });
        // The queued messages drain one per run: the head sends now,
        // the rest wait for the next end.
        flushQueue();
        break;
      case 'seat':
        notifyIfHidden('fleet \u00b7 a seat is waiting',
          (event.label || event.tool || 'a tool call') + ' needs an answer');
        face.running = true;
        face.seat = event;
        face.seatSince = event.since || Date.now();
        break;
      case 'seat_cleared':
        face.seat = null;
        break;
      // The resolution of a seat this face did not answer: the seat is
      // The answer that closed a seat. This surface's own answer leaves
      // the transcript line as its acknowledgment; another surface's
      // answer says who took it, because a button that vanishes without
      // a word reads as a bug. Either way the seat closes — and a seat
      // for the next call opens as a new card, so the popup reads as
      // closing rather than as never closing (KI-283).
      case 'answered': {
        // Match the answer to the SEAT this face is showing, not to the
        // loaded session: the seat carries its own session, the answer
        // names the session whose seat it was, and those two agree even
        // when a run created its log after the last state load.
        var standing = face.seat;
        var seatSession = standing && standing.session;
        var mineSeat = !!standing && (!event.session || !seatSession || event.session === seatSession);
        if (mineSeat) {
          face.seat = null;
          face.answeredElsewhere = event.by || 'another surface';
          approvalRow(event);
          // A seat this face did not answer says who took it; a button
          // that vanishes without a word reads as a bug. This surface's
          // own answer is acknowledged by the line above instead.
          if ((event.by || '') !== 'this browser') {
            notice('the seat was answered elsewhere',
              (event.by || 'another surface') + ' answered it ' +
              (event.decision === 'deny' ? 'no' : 'yes') + '.');
          }
        } else if (!standing && (!event.session || event.session === face.session)) {
          // No standing seat: another surface took one this face had
          // already painted past. The transcript still records it.
          approvalRow(event);
        }
        break;
      }
      case 'rules':
        face.rules = event.rules || [];
        break;
      case 'dropped':
        face.dropped = Number(event.dropped) || face.dropped;
        stripFold(event);
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

    paintConvHead();
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
    paintQueue();
    paintProcess();
    // The router's live decision is the run's own `routed` event, so it
    // is repainted with the stream rather than waiting for the board's
    // clock. The rest of that panel is the board's read and does not move
    // on an event.
    if (face.panel === 'router') paintRouter();
  }

  function paintFacts() {
    var ex = exchanges();
    var rows = [
      // A face that has not run yet has no session: the log is created
      // on the first task (KI-022), so the row reads a dash until one
      // exists rather than naming an id that does not.
      ['session', face.session || '\u2014'],
      ['model', face.model],
      ['spend', tokens(face.spend) + ' tokens'],
      ['context', face.context ? tokens(face.context) + ' of ' + tokens(face.budget) : '\u2014'],
      ['exchanges', ex ? String(ex) : '\u2014'],
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

  // The queued messages: an ordered list above the composer, one sent
  // per run in the owner's order. A row drags to a new position, sends
  // itself first, or drops; the head is what the next run sends.
  var dragIndex = null;

  function paintQueue() {
    if (!el.queueChip || !el.queueList) return;
    var items = face.queued || [];
    el.queueChip.hidden = !items.length;
    if (!items.length) return;
    el.queueCount.textContent = 'queued \u00b7 ' + items.length;
    if (el.queueNote) {
      el.queueNote.textContent = (face.driving && face.driving !== face.session)
        ? 'a run is working in ' + conversationName(face.driving) +
          ' \u00b7 this sends when it ends \u00b7 a reload drops it'
        : 'sends when the run ends \u00b7 a reload drops it';
    }
    el.queueList.textContent = '';
    items.forEach(function (text, index) {
      var row = node('div', 'queue-row');
      row.draggable = true;
      row.tabIndex = 0;
      row.dataset.index = String(index);
      row.title = 'drag to reorder \u00b7 alt+\u2191\u2193 moves it too';
      row.appendChild(node('span', 'queue-handle', '\u28ff'));
      row.appendChild(node('span', 'queue-text', text));
      var first = node('button', 'queue-first', index === 0 ? 'first' : 'send first');
      first.type = 'button';
      first.disabled = index === 0;
      first.title = index === 0
        ? 'already first in line'
        : 'move this message to the head of the queue';
      first.addEventListener('click', function () { moveQueued(index, 0); });
      row.appendChild(first);
      var drop = node('button', 'queue-drop', '\u00d7');
      drop.type = 'button';
      drop.title = 'drop the queued message';
      drop.addEventListener('click', function () {
        face.queued.splice(index, 1);
        paintQueue();
      });
      row.appendChild(drop);
      row.addEventListener('dragstart', function (event) {
        dragIndex = index;
        row.dataset.dragging = 'true';
        if (event.dataTransfer) {
          event.dataTransfer.effectAllowed = 'move';
          event.dataTransfer.setData('text/plain', String(index));
        }
      });
      row.addEventListener('dragend', function () {
        row.dataset.dragging = '';
        dragIndex = null;
      });
      row.addEventListener('dragover', function (event) {
        event.preventDefault();
        row.dataset.over = 'true';
      });
      row.addEventListener('dragleave', function () { row.dataset.over = ''; });
      row.addEventListener('drop', function (event) {
        event.preventDefault();
        row.dataset.over = '';
        var from = dragIndex;
        if (from == null && event.dataTransfer) {
          from = Number(event.dataTransfer.getData('text/plain'));
        }
        if (from == null || !isFinite(from)) return;
        moveQueued(from, index);
      });
      row.addEventListener('keydown', function (event) {
        if (!event.altKey) return;
        if (event.key === 'ArrowUp' && index > 0) {
          event.preventDefault();
          moveQueued(index, index - 1);
        }
        if (event.key === 'ArrowDown' && index < items.length - 1) {
          event.preventDefault();
          moveQueued(index, index + 1);
        }
      });
      el.queueList.appendChild(row);
    });
  }

  function moveQueued(from, to) {
    var items = face.queued || [];
    if (from < 0 || from >= items.length || to < 0 || to >= items.length || from === to) return;
    var moved = items.splice(from, 1)[0];
    items.splice(to, 0, moved);
    paintQueue();
  }

  // The run is over: send the head; the next row follows when that run
  // ends. A run still in flight declines, so a stray call never
  // double-sends.
  function flushQueue() {
    if (!(face.queued || []).length || face.running || face.seat) return;
    var text = face.queued.shift();
    paintQueue();
    el.task.value = text;
    send();
  }

  // One control into the run in flight: the body is written verbatim to
  // the run's control file, and the agent's own parser is the only
  // thing that decides what it means.
  function controlRun(body, okTitle, okNote, refusedTitle) {
    post('/api/control', body).then(function (reply) {
      if (reply.status === 200) {
        notice(okTitle, okNote);
        return;
      }
      notice(refusedTitle, reply.body.error || 'the face said no without a reason');
    }).catch(function () {
      notice(refusedTitle, 'the server did not answer');
    });
  }

  // The one ticker the face runs while something is in flight, so the
  // composer can show how long since the last event — and an idle face
  // has no timer at all.
  var busyTick = null;

  function paintChatScale() {
    var scale = CHAT_SCALES[face.chatStep] || CHAT_SCALES[CHAT_SCALE_DEFAULT];
    // The token lives on the transcript's scroller, so the messages and
    // their markdown read it and the composer, strip, cards and rail do
    // not.
    if (el.log) el.log.style.setProperty('--chat-scale', String(scale));
    var smallest = face.chatStep <= 0;
    var largest = face.chatStep >= CHAT_SCALES.length - 1;
    var percent = Math.round(scale * 100) + '%';
    if (el.chatSmaller) {
      el.chatSmaller.disabled = smallest;
      el.chatSmaller.setAttribute('aria-disabled', smallest ? 'true' : 'false');
      el.chatSmaller.title = smallest
        ? percent + ' is the smallest text'
        : 'smaller text \u00b7 Alt+\u2193';
    }
    if (el.chatBigger) {
      el.chatBigger.disabled = largest;
      el.chatBigger.setAttribute('aria-disabled', largest ? 'true' : 'false');
      el.chatBigger.title = largest
        ? percent + ' is the largest text'
        : 'larger text \u00b7 Alt+\u2191';
    }
    // Reaching an end dims the press that got there. A keyboard user
    // must not be dropped with the button, so focus moves to the pair.
    if (document.activeElement === el.chatBigger && el.chatBigger.disabled && el.chatSmaller) {
      el.chatSmaller.focus();
    } else if (document.activeElement === el.chatSmaller && el.chatSmaller.disabled && el.chatBigger) {
      el.chatBigger.focus();
    }
  }

  // One step up or down, clamped at the ends of the scale. A press at the
  // end is not a silent no-op: the button is already dimmed and its title
  // names the smallest or largest size.
  function stepChatScale(delta) {
    var next = Math.min(CHAT_SCALES.length - 1,
      Math.max(0, face.chatStep + delta));
    if (next === face.chatStep) return;
    face.chatStep = next;
    try {
      window.localStorage.setItem('fleet-chat-size', String(next));
    } catch (error) { }
    paintChatScale();
  }

  function paintComposer() {
    var busy = face.running || !!face.seat;
    // The button is the run's own control: Send when idle, Stop while a
    // run is in flight or a seat waits — the composer is never a dead
    // grey square the owner cannot use to end the work.
    if (el.sendLabel) el.sendLabel.textContent = busy ? 'Stop' : 'Send';
    if (el.send) {
      el.send.dataset.mode = busy ? 'stop' : 'send';
      el.send.disabled = false;
      el.send.setAttribute('aria-label', busy ? 'stop the run' : 'send the task');
      el.send.title = busy
        ? 'stop the run \u00b7 a tool call already running finishes first, then the run ends'
        : 'send the task';
    }
    if (el.hint) el.hint.classList.toggle('busy', busy);
    if (face.seat) {
      // How long the owner has kept the run waiting is the number that
      // matters here: a seat is the run's whole state.
      el.hint.textContent = 'the seat above is waiting for an answer \u00b7 ' +
        elapsed(Date.now() - (face.seatSince || Date.now()));
    } else if (face.running) {
      // Seconds since the last event: a live run shows a small number, a
      // long tool call shows the number climb — which is the honest
      // answer to "is it doing anything", rather than a spinner that
      // claims work it cannot see.
      var since = face.process && face.process.at ? Date.now() - face.process.at : 0;
      var ex = exchanges();
      var turn = face.turn ? (ex ? ' \u00b7 exchange ' + ex : ' \u00b7 turn ' + face.turn) : '';
      el.hint.textContent = 'working' + turn + ' \u00b7 ' + elapsed(since) +
        ' since the last event';
    } else {
      el.hint.textContent = 'Shift+Enter makes a newline';
    }
    // One ticker, alive only while something is: a run moves the seconds,
    // a seat counts the owner's wait, and an idle face has no timer.
    if (busy && !busyTick) {
      busyTick = setInterval(paintComposer, 1000);
    } else if (!busy && busyTick) {
      clearInterval(busyTick);
      busyTick = null;
    }
  }

  // mm:ss under an hour, h:mm:ss above it — a run can outlive a hour and
  // a wrapped clock reads as a bug.
  function elapsed(ms) {
    var total = Math.max(0, Math.round(ms / 1000));
    var hours = Math.floor(total / 3600);
    var minutes = Math.floor((total % 3600) / 60);
    var seconds = total % 60;
    var pad = function (n) { return (n < 10 ? '0' : '') + n; };
    return hours ? hours + ':' + pad(minutes) + ':' + pad(seconds)
      : minutes + ':' + pad(seconds);
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
        'lane', 'delegation', 'control', 'plan', 'artifact', 'finished', 'failed'].indexOf(event.type) === -1) {
        return;
      }
      resetProcess(face.planMode ? 'plan' : 'run', true);
    }
    var p = face.process;
    p.at = Date.now();
    var ex = exchanges();
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
        p.last = ex ? 'exchange ' + ex : 'turn ' + p.turn;
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
      case 'artifact':
        p.stage = 'act';
        p.last = 'handed over ' + (event.path || '');
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
    var ex = exchanges();
    switch (key) {
      case 'route':
        return p.provider ? p.provider + (p.model ? ' \u00b7 ' + p.model : '') : 'waiting for the decision';
      case 'think':
        return p.turn ? (ex ? 'exchange ' + ex : 'turn ' + p.turn) : 'the model\u2019s turn';
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
    var ex = exchanges();
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
        el.processPill.textContent = p.turn
          ? (ex ? 'exchange ' + ex : 'turn ' + p.turn) + ' \u00b7 ' + p.stage
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
    if (p.turn) meta.appendChild(node('span', null, ex ? 'exchange ' + ex : 'turn ' + p.turn));
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
    var ex = exchanges();
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
      p.turn ? (ex ? 'exchange ' + ex : 'turn ' + p.turn)
        : p.mode === 'plan' ? 'plan mode' : 'the run'));
    main.appendChild(node('span', 'process-strip-note',
      p.tool || (p.stage === 'review' ? 'the plan is ready'
        : p.last && p.last !== (ex ? 'exchange ' + ex : 'turn ' + p.turn) ? p.last : '')));
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
    var ex = exchanges();
    if (!p) {
      openDrawer('the run', [['process', node('p', 'derivation',
        'No run is in flight. Ask fleet for something and the loop lands here.')]]);
      return;
    }
    var sections = [
      ['where we are', processSteps(p)],
      ['the last step', node('p', 'derivation', p.last || 'no step yet')],
      ['the counts', node('p', 'derivation',
        (ex ? 'exchange ' + ex : 'turn ' + p.turn) + ' \u00b7 ' + p.calls + ' calls \u00b7 ' + p.facts +
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
        ? 'Local to ' + face.workspace + ' \u00b7 the archive stays here; turns go to the providers that serve them; a document escalated to Perception goes to Nace.'
        : 'Your archive stays on this machine. Every turn is a card you can open; the providers that serve it are named in the route bar and the lanes. A document escalated to Perception goes to Nace.';
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
        window.location.hash = '#/sessions/' + encodeURIComponent(row.id);
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
    ];
    // The second head, when the lane judged it (C4): the topology
    // verdict beside the workload's, with the head's own confidence.
    // A workload-only answer adds no row — the absence is not a dash
    // nobody reads.
    if (route.execution) {
      facts.push(['execution', route.execution.label +
        (typeof route.execution.confidence === 'number'
          ? ' \u00b7 ' + route.execution.confidence.toFixed(2)
          : '')]);
    }
    facts.forEach(function (pair) {
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
    el.routerBoard.textContent = '';
    if (!read) {
      el.routerBoard.appendChild(surfaceHead('Intelligence', 'Router',
        'Where this run\u2019s work went, what it tried first, and what the day has served.', null));
      var waiting = boardCard(12, 'reading', null, null);
      waiting.appendChild(node('p', 'board-empty',
        'The board\u2019s read has not landed yet. The live decision below is the run\u2019s own event ' +
        'and does not wait for it.'));
      el.routerBoard.appendChild(waiting);
      el.routerBoard.appendChild(decisionCard());
      return;
    }
    var tiers = read.tiers || [];
    var totals = read.totals || {};
    var errors = totals.errors || 0;
    var rate = totals.success_rate == null ? '\u2014' : Math.round(totals.success_rate * 100) + '%';
    var instrument = surfaceInstrument('the router', read.pin || 'no pin',
      'judge ' + (read.judge ? read.judge.lane : '\u2014') + ' \u00b7 ' + tiers.length + ' tier chains',
      [
        {
          label: 'ask sys1 for a judgment',
          run: function () {
            ask('use the sys1 lane to review today\u2019s route decisions and report the fit');
          }
        },
        {
          label: 'open settings',
          run: function () { window.location.hash = '#/settings'; }
        }
      ]);
    el.routerBoard.appendChild(surfaceHead('Intelligence', 'Router',
      'Where this run\u2019s work went, what it tried first, and what the day has served.', instrument));
    el.routerBoard.appendChild(surfaceStrip([
      [totals.calls || 0, 'calls today'],
      [totals.ok || 0, 'ok'],
      [errors, 'errors', errors ? 'down' : null],
      [rate, 'success'],
      [totals.total_tokens || 0, 'tokens'],
      [read.judge ? read.judge.lane : '\u2014', 'judge']
    ], read.pin ? 'pinned to ' + read.pin : 'no pin'));

    var left = node('div', 'surface-column');
    left.appendChild(decisionCard());
    left.appendChild(chainCard(read));
    left.appendChild(rowsCard(read));
    el.routerBoard.appendChild(left);

    var rail = [
      dayCard(read, cardWide(el.routerBoard, 4)),
      mixCard(read),
      mixSplitCard(read),
      cacheCard(read)
    ];
    var guidance = guidanceCard(read);
    if (guidance) rail.push(guidance);
    el.routerBoard.appendChild(surfaceRail(rail));

    if (read.refused && read.refused.length) {
      var refused = boardCard(12, 'reads that did not land', 'the board\u2019s read');
      refused.appendChild(node('p', 'board-empty', read.refused.join(' \u00b7 ')));
      el.routerBoard.appendChild(refused);
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
    // The workspace branch, when the child worked one: the row's link
    // into the workspace-history section, where the branch's own line
    // and its merge state are drawn.
    if (one.workspace_branch) {
      made.appendChild(label(x + 12, y + 54, 'tree-branch',
        clip(one.workspace_branch, Math.max(18, Math.floor((boxW - 24) / 6.0)))));
      said += '\nbranch ' + one.workspace_branch + ' \u2014 click for its history';
      made.style.cursor = 'pointer';
      made.addEventListener('click', function () {
        window.location.hash = '#/workspace-history';
      });
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
    el.delegationBoard.textContent = '';
    if (!read) {
      el.delegationBoard.appendChild(surfaceHead('Intelligence', 'Delegation',
        'The run\u2019s own tree, the runs before it, and the dependency graph the plan declares.', null));
      var waiting = boardCard(12, 'reading', null, null);
      waiting.appendChild(node('p', 'board-empty',
        'The delegation read has not landed yet. It reads the session logs and the graph file, ' +
        'both of which are this machine\u2019s own.'));
      el.delegationBoard.appendChild(waiting);
      return;
    }
    var live = read.live;
    var history = read.history || [];
    var graph = read.graph;
    var instrument = surfaceInstrument('the tree',
      live ? 'live run' : 'no run in flight',
      'depth cap ' + read.depth_cap + ' \u00b7 ' + history.length +
      (history.length === 1 ? ' past run' : ' past runs'),
      [{
        label: 'ask for the next step',
        run: function () {
          ask('read the dependency graph and tell me the next ready node to work');
        }
      }]);
    el.delegationBoard.appendChild(surfaceHead('Intelligence', 'Delegation',
      'The run\u2019s own tree, the runs before it, and the dependency graph the plan declares.',
      instrument));
    el.delegationBoard.appendChild(surfaceStrip([
      [read.depth_cap, 'depth cap'],
      [live ? countNodes(live) : 0, 'in this run'],
      [history.length, 'past runs'],
      [graph ? graph.nodes.length : 0, 'graph nodes'],
      [(read.refused || []).length, 'reads refused', (read.refused || []).length ? 'down' : null]
    ], live ? 'a run is in flight' : 'no run in flight'));

    var left = node('div', 'surface-column');
    var wide = cardWide(el.delegationBoard, 8);
    if (live) {
      left.appendChild(treeCard(live, 'the run in flight', null, read.depth_cap, wide));
    } else {
      var none = boardCard(12, 'the run in flight', null, null);
      none.appendChild(node('p', 'board-empty',
        'No readable session log for this run yet. A tree is rebuilt off the logs\u2019 own start ' +
        'records, so a run that has not written one has no tree to draw.'));
      left.appendChild(none);
    }
    history.forEach(function (root) {
      left.appendChild(treeCard(root,
        'run ' + clip(root.id, 22),
        (root.stop_reason ? 'stopped: ' + root.stop_reason : 'no end record')
        + (root.turns != null ? ' \u00b7 ' + root.turns + ' turns' : '')
        + (root.goal ? ' \u00b7 ' + root.goal : ''),
        read.depth_cap, wide));
    });
    el.delegationBoard.appendChild(left);

    var summary = boardCard(12, 'what the read saw', live ? 'live' : 'idle',
      'the tree is rebuilt off the start records\u2019 own parent ids; the graph is a file the plan\u2019s work writes.');
    [
      ['depth cap', String(read.depth_cap)],
      ['nodes in this run', String(live ? countNodes(live) : 0)],
      ['past runs', String(history.length)],
      ['graph nodes', String(graph ? graph.nodes.length : 0)],
      ['graph edges', String(graph ? graph.edges.length : 0)]
    ].forEach(function (pair) {
      var line = node('p', 'derivation');
      line.appendChild(node('b', null, pair[0] + ' \u00b7 '));
      line.appendChild(document.createTextNode(pair[1]));
      summary.appendChild(line);
    });
    var rail = [summary];
    if ((read.refused || []).length) rail.push(refusedCard(read.refused));
    el.delegationBoard.appendChild(surfaceRail(rail));

    if (graph && graph.nodes.length) {
      el.delegationBoard.appendChild(graphCard(graph, cardWide(el.delegationBoard, 12)));
      el.delegationBoard.appendChild(unlockCard(graph));
      el.delegationBoard.appendChild(graphTiles(graph));
    } else {
      var empty = boardCard(12, 'the dependency graph', null, null);
      empty.appendChild(node('p', 'board-empty',
        'This home has declared no dependency graph. The graph is a file the plan\u2019s work ' +
        'writes; nothing here is inferred from what a run did.'));
      el.delegationBoard.appendChild(empty);
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
  // ---- the memory and skills surfaces -------------------------------------
  // Both read like the atlas: a compact instrument in the page head, a
  // figures strip, a dense left-hand table whose rows carry their own
  // actions, and a narrow rail for the day's activity or the decisions
  // waiting. The cards that carry rows are full-bleed, so a row's
  // hairline runs the card the way a ledger's does.

  function badge(text, tone) {
    var el0 = node('span', 'badge', String(text));
    el0.title = String(text);
    if (tone) el0.dataset.tone = tone;
    return el0;
  }

  function surfaceHead(eyebrow, title, note, instrument) {
    var head = node('header', 'surface-head');
    var main = node('div', 'surface-main');
    main.appendChild(node('p', 'page-eyebrow', eyebrow));
    main.appendChild(node('h1', 'surface-title', title));
    main.appendChild(node('p', 'page-note', note));
    head.appendChild(main);
    if (instrument) head.appendChild(instrument);
    return head;
  }

  function surfaceStrip(items, end) {
    var strip = node('div', 'surface-strip');
    items.forEach(function (item) {
      var span = node('span', item[2] === 'down' ? 'surface-strip-down' : null);
      span.appendChild(node('b', null, String(item[0])));
      span.appendChild(document.createTextNode(' ' + item[1]));
      strip.appendChild(span);
    });
    if (end) strip.appendChild(node('span', 'surface-strip-end', end));
    return strip;
  }

  function surfaceInstrument(title, count, meta, actions) {
    var box = node('div', 'surface-tool');
    var line = node('div', 'surface-tool-line');
    line.appendChild(node('span', 'surface-live'));
    line.appendChild(node('b', null, title));
    if (count != null) line.appendChild(badge(count, null));
    box.appendChild(line);
    if (meta) box.appendChild(node('p', 'surface-tool-meta', meta));
    if (actions && actions.length) {
      var act = node('div', 'surface-tool-act');
      actions.forEach(function (action) {
        var button = node('button', 'surface-refresh', action.label);
        button.type = 'button';
        button.addEventListener('click', action.run);
        act.appendChild(button);
      });
      box.appendChild(act);
    }
    return box;
  }

  // The rail: the cards that sit beside a table, stacked in one column.
  function surfaceRail(cards) {
    var rail = node('div', 'surface-rail');
    cards.forEach(function (card) { rail.appendChild(card); });
    return rail;
  }

  function rowAction(label, title, run, primary) {
    var button = node('button', 'row-action' + (primary ? ' row-action-primary' : ''), label);
    button.type = 'button';
    button.title = title || label;
    button.addEventListener('click', function (event) {
      event.stopPropagation();
      run();
    });
    return button;
  }

  // A path opens in the pane, with the code tab in front.
  function openPath(path) {
    if (!path) return;
    openPane();
    openFile(path);
  }

  function paintMemory() {
    var read = face.memory;
    el.memoryBoard.textContent = '';
    if (!read) {
      el.memoryBoard.appendChild(surfaceHead('Knowledge', 'Memory',
        'One fact per file: what this run recalled, and what the corpus already knows.', null));
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
    var shown = facts.filter(function (fact) {
      return memoryFilterMatch(fact, filter) && matchesQuery(fact);
    });
    var instrument = surfaceInstrument('the corpus', facts.length + ' facts',
      read.root + ' \u00b7 ' + hits.length + ' hits on ' + read.day,
      [
        {
          label: 'ask what fleet knows',
          run: function () {
            ask('summarise what the memories know about my current work and cite the facts');
          }
        },
        {
          label: 'copy the path',
          run: function () {
            if (navigator.clipboard) navigator.clipboard.writeText(read.root).catch(function () { });
            notice('the corpus path is copied', read.root);
          }
        }
      ]);
    el.memoryBoard.appendChild(surfaceHead('Knowledge', 'Memory',
      'One fact per file: what this run recalled, and what the corpus already knows.', instrument));
    el.memoryBoard.appendChild(surfaceStrip([
      [facts.length, 'facts'],
      [live.length, 'live'],
      [protectedFacts.length, 'protected'],
      [retired.length, 'retired', retired.length ? 'down' : null],
      [hits.length, 'hits ' + read.day],
      [recalls.length, 'recalls this run']
    ], 'the corpus at ' + read.root));
    el.memoryBoard.appendChild(memCorpus(shown, filter, facts.length));
    el.memoryBoard.appendChild(surfaceRail([memHits(read, hits, recalls), memMix(facts)]));
    if (read.refused && read.refused.length) {
      el.memoryBoard.appendChild(refusedCard(read.refused));
    }
  }

  function memCorpus(shown, filter, total) {
    var card = boardCard(8, 'the corpus',
      shown.length + (filter === 'all' ? ' of ' + total : ' ' + filter),
      'a fact is one file. A row opens it with its whole body and its edit, written through ' +
      'the substrate\u2019s own rewriter; the row\u2019s own buttons do the three things you ' +
      'reach for while reading.');
    card.classList.add('lib-card');
    var tools = node('div', 'lib-tools');
    tools.appendChild(memorySearch());
    card.appendChild(tools);
    var facts = (face.memory && face.memory.facts) || [];
    var live = facts.filter(function (fact) { return !fact.retired; });
    card.appendChild(filterBar([
      { value: 'all', label: 'all', count: facts.length },
      { value: 'live', label: 'live', count: live.length },
      { value: 'protected', label: 'protected', count: live.filter(function (fact) { return fact.protected; }).length },
      { value: 'retired', label: 'retired', count: facts.filter(function (fact) { return fact.retired; }).length }
    ], filter, function (value) {
      face.memoryFilter = value;
      paintMemory();
    }));
    var cols = node('div', 'mem-cols');
    ['fact', 'type', 'basis', 'recalls', 'state', 'actions'].forEach(function (label) {
      cols.appendChild(node('span', null, label));
    });
    card.appendChild(cols);
    shown.forEach(function (fact) { card.appendChild(memRow(fact)); });
    if (!shown.length) {
      card.appendChild(node('p', 'board-empty',
        facts.length
          ? 'No fact matches' + (face.query ? ' \u201c' + face.query + '\u201d' : ' this filter') +
            '. The search reads the name, the sentence, the type, the shape tags and the body.'
          : 'This home holds no facts. The agent writes them through the substrate\u2019s own writer.'));
    }
    return card;
  }

  function memRow(fact) {
    var row = node('div', 'mem-row');
    row.tabIndex = 0;
    row.setAttribute('role', 'button');
    row.title = 'open ' + fact.name + ' and its edit';
    var main = node('div', 'mem-fact');
    main.appendChild(node('b', null, fact.name));
    main.appendChild(node('p', null, fact.description));
    row.appendChild(main);
    row.appendChild(badge(fact.fact_type, null));
    var basis = node('div', 'mem-basis');
    basis.appendChild(node('span', null, fact.basis));
    basis.appendChild(node('span', 'mem-tier', fact.tier));
    row.appendChild(basis);
    row.appendChild(node('span', 'mem-num', String(fact.recall_count)));
    var state = node('div', 'mem-state');
    if (fact.retired) state.appendChild(badge('retired', 'down'));
    else if (fact.protected) state.appendChild(badge('protected', 'accent'));
    else state.appendChild(node('span', 'mem-live', 'live'));
    if (fact.valid_until) state.appendChild(node('span', 'mem-tier', 'until ' + fact.valid_until));
    row.appendChild(state);
    var acts = node('div', 'row-actions');
    acts.appendChild(rowAction('edit', 'open the fact and its edit', function () {
      inspectFact(fact);
    }, true));
    acts.appendChild(rowAction('ask', 'ask fleet about this fact', function () {
      ask('what do the memories say about ' + fact.name + '? cite the fact and its evidence.');
    }));
    acts.appendChild(rowAction('file', 'open the fact file', function () {
      if (fact.relative) {
        openPath(fact.relative);
        return;
      }
      if (navigator.clipboard) navigator.clipboard.writeText(fact.path).catch(function () { });
      notice('the fact is outside the working directory', 'its path is copied: ' + fact.path);
    }));
    row.appendChild(acts);
    var open = function () { inspectFact(fact); };
    row.addEventListener('click', open);
    row.addEventListener('keydown', function (event) {
      if (event.key === 'Enter' || event.key === ' ') {
        event.preventDefault();
        open();
      }
    });
    return row;
  }

  function memHits(read, hits, recalls) {
    var card = boardCard(4, 'today\u2019s recall', hits.length + ' hits',
      'one row per fact a recall surfaced, plus this run\u2019s own recalls from the event log.');
    card.classList.add('lib-card');
    var list = node('div', 'rail-list');
    if (recalls.length) {
      var run = node('div', 'rail-group');
      run.appendChild(node('p', 'rail-label', 'this run \u00b7 ' + recalls.length));
      recalls.slice(-6).forEach(function (event) {
        var row = node('div', 'rail-row');
        var body = node('div');
        body.appendChild(node('b', null, (event.shape ? event.shape + ' \u00b7 ' : '') + event.facts + ' facts'));
        body.appendChild(node('p', null, 'recalled for this run'));
        row.appendChild(body);
        list.appendChild(row);
      });
      card.appendChild(run);
    }
    if (hits.length) {
      var day = node('div', 'rail-group');
      day.appendChild(node('p', 'rail-label', read.day));
      hits.slice(0, 12).forEach(function (hit) {
        var row = node('div', 'rail-row');
        row.tabIndex = 0;
        row.setAttribute('role', 'button');
        var body = node('div');
        body.appendChild(node('b', null, hit.fact));
        body.appendChild(node('p', null, (hit.shape ? hit.shape + ' \u00b7 ' : '') + clock(hit.ts)));
        row.appendChild(body);
        row.appendChild(node('span', 'rail-em', hit.score.toFixed(2)));
        row.addEventListener('click', function () {
          openDrawer('recall \u00b7 ' + hit.fact, [
            ['the record', node('pre', 'raw', JSON.stringify(hit, null, 2))]
          ]);
        });
        list.appendChild(row);
      });
      if (hits.length > 12) {
        list.appendChild(node('p', 'rail-more', (hits.length - 12) + ' more hits in the day\u2019s stream'));
      }
    }
    if (!recalls.length && !hits.length) {
      list.appendChild(node('p', 'board-empty',
        'Nothing has been recalled today. A hit is one calibration row per fact a recall surfaced.'));
    }
    card.appendChild(list);
    return card;
  }

  function memMix(facts) {
    var types = {};
    var bases = {};
    facts.forEach(function (fact) {
      types[fact.fact_type] = (types[fact.fact_type] || 0) + 1;
      bases[fact.basis] = (bases[fact.basis] || 0) + 1;
    });
    var card = boardCard(4, 'the corpus mix', facts.length + ' facts', null);
    card.classList.add('lib-card');
    var list = node('div', 'rail-list');
    var max = Object.keys(types).reduce(function (value, key) { return Math.max(value, types[key]); }, 0);
    Object.keys(types).sort(function (left, right) { return types[right] - types[left]; })
      .forEach(function (type) {
        var row = node('div', 'mix-row');
        row.appendChild(node('span', null, type));
        var bar = node('span', 'mix-bar');
        var fill = node('i');
        fill.style.width = Math.round(100 * types[type] / max) + '%';
        bar.appendChild(fill);
        row.appendChild(bar);
        row.appendChild(node('span', 'rail-em', String(types[type])));
        list.appendChild(row);
      });
    Object.keys(bases).sort().forEach(function (basis) {
      var row = node('div', 'mix-row');
      row.appendChild(node('span', null, basis));
      row.appendChild(node('span'));
      row.appendChild(node('span', 'rail-em', String(bases[basis])));
      list.appendChild(row);
    });
    card.appendChild(list);
    return card;
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
      el.skillsBoard.appendChild(surfaceHead('Knowledge', 'Skills',
        'The library the model can reach, the names the loader declined, the proposals waiting for a decision, and what the owner retired.', null));
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
    var query = face.skillQuery || '';
    var shown = skills
      .filter(function (skill) {
        return skillFilterMatch(skill, filter) && skillMatches(skill, query);
      })
      .sort(function (left, right) {
        return right.uses - left.uses || left.name.localeCompare(right.name);
      });
    var instrument = surfaceInstrument('the library', skills.length + ' skills',
      (read.canonical || 'no library') + ' \u00b7 ' + read.sessions + ' sessions scanned',
      [
        {
          label: 'ask the curator',
          run: function () {
            ask('review the skill library and propose the next curation change');
          }
        },
        {
          label: 'copy the path',
          run: function () {
            if (read.canonical && navigator.clipboard) {
              navigator.clipboard.writeText(read.canonical).catch(function () { });
            }
            notice('the library path is copied', read.canonical || 'no library resolved');
          }
        }
      ]);
    el.skillsBoard.appendChild(surfaceHead('Knowledge', 'Skills',
      'The library the model can reach, the names the loader declined, the proposals waiting for a decision, and what the owner retired.',
      instrument));
    el.skillsBoard.appendChild(surfaceStrip([
      [skills.length, 'loaded'],
      [used, 'used'],
      [read.unused, 'never selected'],
      [proposals.length, 'proposals', proposals.length ? 'live' : null],
      [skips.length, 'skipped', skips.length ? 'down' : null],
      [read.retired.length, 'retired', read.retired.length ? 'down' : null],
      [read.sessions, 'sessions scanned']
    ], 'the library at ' + (read.canonical || '\u2014')));
    el.skillsBoard.appendChild(skillLibrary(read, shown, filter));
    el.skillsBoard.appendChild(surfaceRail([
      skillsDecisions(proposals),
      skillsSkips(skips),
      skillsCuration(read),
      skillsRetired(read)
    ]));
    if (read.refused && read.refused.length) {
      el.skillsBoard.appendChild(refusedCard(read.refused));
    }
  }

  function skillFilterMatch(skill, filter) {
    if (filter === 'used') return skill.uses > 0;
    if (filter === 'unused') return !skill.uses;
    return true;
  }

  function skillMatches(skill, query) {
    var asked = (query || '').trim().toLowerCase();
    if (!asked) return true;
    return (skill.name + ' ' + skill.description).toLowerCase().indexOf(asked) !== -1;
  }

  function skillSearch() {
    var wrap = node('div', 'panel-search');
    var box = node('input', 'panel-search-box');
    box.type = 'search';
    box.placeholder = 'search the library \u2014 name or sentence';
    box.value = face.skillQuery || '';
    box.setAttribute('aria-label', 'search the library');
    box.addEventListener('input', function () {
      face.skillQuery = box.value;
      paintSkills();
      var again = el.skillsBoard.querySelector('.panel-search-box');
      if (again) {
        again.focus();
        again.setSelectionRange(again.value.length, again.value.length);
      }
    });
    wrap.appendChild(box);
    return wrap;
  }

  function skillLibrary(read, shown, filter) {
    var skills = read.skills || [];
    var used = skills.filter(function (skill) { return skill.uses > 0; }).length;
    var card = boardCard(8, 'the library', shown.length + ' of ' + skills.length,
      'one row per skill the loader accepted. A row opens the skill; its buttons open the file, ' +
      'ask the curator, or copy the name.');
    card.classList.add('lib-card');
    var tools = node('div', 'lib-tools');
    tools.appendChild(skillSearch());
    card.appendChild(tools);
    card.appendChild(filterBar([
      { value: 'all', label: 'all', count: skills.length },
      { value: 'used', label: 'used', count: used },
      { value: 'unused', label: 'never used', count: skills.length - used }
    ], filter, function (value) {
      face.skillsFilter = value;
      face.skillShowAll = false;
      paintSkills();
    }));
    var cols = node('div', 'skill-cols');
    ['skill', 'uses', 'modified', 'actions'].forEach(function (label) {
      cols.appendChild(node('span', null, label));
    });
    card.appendChild(cols);
    var limit = face.skillShowAll || face.skillQuery
      ? shown.length : Math.min(30, shown.length);
    shown.slice(0, limit).forEach(function (skill) { card.appendChild(skillRow(skill)); });
    if (!shown.length) {
      card.appendChild(node('p', 'board-empty',
        'No skill matches' + (face.skillQuery ? ' \u201c' + face.skillQuery + '\u201d' : ' this filter') +
        '. The search reads the name and the one sentence.'));
    }
    var foot = node('div', 'lib-foot');
    foot.appendChild(node('span', null,
      'showing ' + limit + ' of ' + shown.length + (face.skillQuery ? ' matches' : '')));
    if (!face.skillShowAll && !face.skillQuery && shown.length > limit) {
      var more = node('button', 'row-action', 'show all ' + shown.length);
      more.type = 'button';
      more.addEventListener('click', function () { face.skillShowAll = true; paintSkills(); });
      foot.appendChild(more);
    } else if (face.skillShowAll && shown.length > 30) {
      var less = node('button', 'row-action', 'show the first 30');
      less.type = 'button';
      less.addEventListener('click', function () { face.skillShowAll = false; paintSkills(); });
      foot.appendChild(less);
    }
    card.appendChild(foot);
    return card;
  }

  // Retire a skill from every run's prompt, or return it: the owner's
  // click, written fleet-locally, the library untouched (KI-215). The
  // in-flight and refusal states mirror the decisions queue's approve.
  function retireSkill(name, button, retiring) {
    var path = retiring ? '/api/skill/retire' : '/api/skill/unretire';
    button.disabled = true;
    button.textContent = retiring ? 'retiring\u2026' : 'returning\u2026';
    post(path, { name: name }).then(function (reply) {
      if (reply.status === 200) {
        notice(retiring ? 'the skill is retired' : 'the skill is back',
          name + (retiring
            ? ' leaves every run\u2019s prompt; the library keeps it'
            : ' is declared again'));
        readSkills(true).then(paintSkills);
        return;
      }
      button.disabled = false;
      button.textContent = retiring ? 'retire' : 'unretire';
      notice(retiring ? 'the retire was refused' : 'the return was refused',
        reply.body.error || 'the face said no without a reason');
    }).catch(function () {
      button.disabled = false;
      button.textContent = retiring ? 'retire' : 'unretire';
      notice(retiring ? 'the retire did not cross' : 'the return did not cross',
        'the server did not answer');
    });
  }

  // The retired section: the names that left the prompt, one click
  // from returning. The library is never written — a retirement is
  // fleet-local and reversible (KI-215's shape, the owner's ruling).
  function skillsRetired(read) {
    var retired = read.retired || [];
    var card = boardCard(12, 'retired', retired.length + ' retired',
      'a retired skill leaves every run\u2019s prompt and stays in the library; ' +
      'returning one is the same click.');
    card.classList.add('lib-card');
    if (!retired.length) {
      card.appendChild(node('p', 'board-empty',
        'Nothing is retired. Retire one from a row above when the scan says it has ' +
        'never been selected.'));
      return card;
    }
    var list = node('div', 'rail-list');
    retired.forEach(function (name) {
      var row = node('div', 'rail-row');
      row.tabIndex = 0;
      row.setAttribute('role', 'button');
      row.title = 'the retired skill ' + name;
      row.appendChild(node('b', null, name));
      var back = rowAction('unretire', 'declare this skill again', function () {
        retireSkill(name, back, false);
      });
      var acts = node('div', 'row-actions');
      acts.appendChild(back);
      row.appendChild(acts);
      list.appendChild(row);
    });
    card.appendChild(list);
    return card;
  }

  function skillRow(skill) {
    var row = node('div', 'skill-row');
    row.tabIndex = 0;
    row.setAttribute('role', 'button');
    row.title = 'open ' + skill.name;
    var main = node('div', 'mem-fact');
    main.appendChild(node('b', null, skill.name));
    main.appendChild(node('p', null, skill.description));
    row.appendChild(main);
    row.appendChild(node('span', 'mem-num', String(skill.uses)));
    row.appendChild(node('span', 'skill-mod', (skill.modified || '').slice(0, 10) || '\u2014'));
    var acts = node('div', 'row-actions');
    acts.appendChild(rowAction('open', 'open the skill', function () {
      inspectSkill(skill);
    }, true));
    acts.appendChild(rowAction('path', 'copy the skill path', function () {
      if (navigator.clipboard) navigator.clipboard.writeText(skill.path).catch(function () { });
      notice('the skill path is copied', skill.path);
    }));
    acts.appendChild(rowAction('ask', 'ask the curator about this skill', function () {
      ask('review the ' + skill.name + ' skill against the sessions that used it and say whether it should change');
    }));
    acts.appendChild(rowAction('copy', 'copy the skill name', function () {
      if (navigator.clipboard) navigator.clipboard.writeText(skill.name).catch(function () { });
      notice('the skill name is copied', skill.name);
    }));
    var retire = rowAction('retire', 'retire this skill from every run\u2019s prompt', function () {
      retireSkill(skill.name, retire, true);
    });
    acts.appendChild(retire);
    row.appendChild(acts);
    var open = function () { inspectSkill(skill); };
    row.addEventListener('click', open);
    row.addEventListener('keydown', function (event) {
      if (event.key === 'Enter' || event.key === ' ') {
        event.preventDefault();
        open();
      }
    });
    return row;
  }

  function skillsDecisions(proposals) {
    var card = boardCard(12, 'the decisions queue',
      proposals.length + (proposals.length === 1 ? ' proposal' : ' proposals'),
      'a proposal is a drafted change; an approve rides the same apply the CLI rides.');
    card.classList.add('lib-card');
    var list = node('div', 'rail-list');
    proposals.forEach(function (proposal) {
      var row = node('div', 'rail-row');
      row.tabIndex = 0;
      row.setAttribute('role', 'button');
      row.title = 'read the proposal';
      var body = node('div');
      body.appendChild(node('b', null, proposal.name));
      body.appendChild(node('p', null, proposal.kind + ' \u00b7 ' + clip(proposal.evidence, 130)));
      row.appendChild(body);
      var acts = node('div', 'row-actions');
      var approve = node('button', 'row-action row-action-primary', 'approve');
      approve.type = 'button';
      approve.title = 'apply this proposal';
      approve.addEventListener('click', function (event) {
        event.stopPropagation();
        approve.disabled = true;
        approve.textContent = 'applying\u2026';
        post('/api/skill', { name: proposal.name }).then(function (reply) {
          if (reply.status === 200) {
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
      acts.appendChild(approve);
      acts.appendChild(rowAction('read', 'read the proposal', function () {
        inspectProposal(proposal);
      }));
      row.appendChild(acts);
      row.addEventListener('click', function () { inspectProposal(proposal); });
      row.addEventListener('keydown', function (event) {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault();
          inspectProposal(proposal);
        }
      });
      list.appendChild(row);
    });
    if (!proposals.length) {
      list.appendChild(node('p', 'board-empty',
        'No proposal is awaiting a decision. An approve here rides the same apply the CLI rides.'));
    }
    card.appendChild(list);
    return card;
  }

  function skillsSkips(skips) {
    var card = boardCard(12, 'declined', skips.length + ' skipped',
      'a skip is the loader\u2019s own report, in its own words.');
    card.classList.add('lib-card');
    var list = node('div', 'rail-list');
    skips.forEach(function (skip) {
      var row = node('div', 'rail-row');
      row.tabIndex = 0;
      row.setAttribute('role', 'button');
      row.title = skip.reason;
      var body = node('div');
      body.appendChild(node('b', null, skip.name));
      body.appendChild(node('p', null, clip(skip.reason, 150)));
      row.appendChild(body);
      row.addEventListener('click', function () {
        openDrawer('skip \u00b7 ' + skip.name, [
          ['why the loader declined it', node('p', 'derivation', skip.reason)]
        ]);
      });
      row.addEventListener('keydown', function (event) {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault();
          row.click();
        }
      });
      list.appendChild(row);
    });
    if (!skips.length) list.appendChild(node('p', 'rail-note', 'The loader accepted every directory it read.'));
    card.appendChild(list);
    return card;
  }

  function skillsCuration(read) {
    var merges = read.merges || [];
    var card = boardCard(12, 'curation', merges.length + ' candidates', read.curation_note);
    card.classList.add('lib-card');
    card.appendChild(node('p', 'rail-note',
      read.unused + ' of ' + ((read.skills || []).length) + ' skills were never selected across ' +
      read.sessions + ' scanned sessions. A merge candidate is a pair scoring at or above ' +
      read.threshold + ' on the substrate\u2019s own description similarity.'));
    var list = node('div', 'rail-list');
    merges.forEach(function (merge) {
      var row = node('div', 'rail-row');
      row.tabIndex = 0;
      row.setAttribute('role', 'button');
      var body = node('div');
      body.appendChild(node('b', null, merge.survivor + ' would absorb ' + merge.absorbed));
      body.appendChild(node('p', null, 'similarity ' + merge.similarity.toFixed(2)));
      row.appendChild(body);
      row.addEventListener('click', function () {
        openDrawer('merge candidate', [
          ['the pair', node('p', 'derivation', merge.survivor + ' would absorb ' + merge.absorbed)],
          ['the measure', node('p', 'derivation',
            'description similarity ' + merge.similarity.toFixed(2) +
            ' \u00b7 the threshold is ' + read.threshold)]
        ]);
      });
      list.appendChild(row);
    });
    if (!merges.length) {
      list.appendChild(node('p', 'rail-note', 'No two descriptions read as near-identical at this threshold.'));
    }
    card.appendChild(list);
    return card;
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
  // The draft is the conversation's, not the face's (D2): one key per
  // session, swapped on every conversation change, so two conversations
  // never share the sentence being typed. The legacy single-key draft
  // migrates into the visiting conversation on first read.
  function draftKey(session) {
    return 'fleet-draft-' + (session || '');
  }

  function saveDraft() {
    try {
      window.localStorage.setItem(draftKey(face.session), el.task.value);
    } catch (error) { }
  }

  function clearDraft() {
    try { window.localStorage.removeItem(draftKey(face.session)); } catch (error) { }
  }

  function loadDraft() {
    try {
      var key = draftKey(face.session);
      var legacy = window.localStorage.getItem('fleet-draft');
      if (legacy != null && window.localStorage.getItem(key) == null) {
        window.localStorage.setItem(key, legacy);
        window.localStorage.removeItem('fleet-draft');
      }
      var draft = window.localStorage.getItem(key);
      el.task.value = draft || '';
      grow();
    } catch (error) { }
  }

  // Swap on a conversation change: the outgoing conversation keeps its
  // sentence, the incoming one gets its own back.
  function swapDraft(fromSession, toSession) {
    if (fromSession === toSession) return;
    try {
      if (el.task.value) {
        window.localStorage.setItem(draftKey(fromSession), el.task.value);
      } else {
        window.localStorage.removeItem(draftKey(fromSession));
      }
    } catch (error) { }
    face.session = toSession;
    loadDraft();
  }

  function ask(text) {
    if (window.location.hash !== '#/chat') window.location.hash = '#/chat';
    el.task.value = text;
    saveDraft();
    grow();
    el.task.focus();
    el.task.setSelectionRange(text.length, text.length);
  }

  function send() {
    // While a run is in flight the composer queues: one slot, sent the
    // moment the run ends. The queue is the face's own state — a reload
    // drops it, and the chip says so.
    if (face.running || face.seat ||
        (face.driving && face.driving !== face.session)) {
      var pending = el.task.value.trim();
      if (pending) {
        face.queued.push(pending);
        el.task.value = '';
        grow();
        clearDraft();
        paintQueue();
      }
      return;
    }
    if (face.planMode) { planSend(); return; }
    // An edit in flight (D3): rewind to the turn before the original
    // run, then post the edit as the next run. The confirm is the
    // rewind's own grammar — the tree's files go back with it.
    var edit = face.editing || null;
    face.editing = null;
    if (edit && edit.turn != null) {
      var edited = el.task.value.trim();
      if (!edited) return;
      if (!window.confirm('rewind to turn ' + edit.turn +
        ' and re-run with your edit? The files the runs after it changed go back, ' +
        'and the original message stays in the log.')) {
        return;
      }
      post('/api/rewind', { turn: edit.turn }).then(function (reply) {
        if (reply.status !== 200) {
          notice('the rewind was refused',
            (reply.body && reply.body.error) || 'the face said no without a reason');
          el.task.value = edited;
          saveDraft();
          return;
        }
        dispatchTask();
      }).catch(function () {
        el.task.value = edited;
        saveDraft();
      });
      return;
    }
    dispatchTask();
  }

  // The run dispatch itself — the composer's text to /api/run, with the
  // face's standing choices. Split from send() so the edit path can
  // rewind first and land here unchanged.
  function dispatchTask() {
    var task = el.task.value.trim();
    if (!task) return;
    el.task.value = '';
    grow();
    clearDraft();
    el.empty.hidden = true;
    el.log.hidden = false;
    var before = face.events.length;
    // A folder change in flight lands first: the run reads the stored
    // choice, and a run that overtook its own folder would work in the
    // old one.
    Promise.resolve(folderSync).then(function () {
      // The run names the session the face is showing: after opening a
      // session from the archive, a session-less run would go to the
      // launch session — a different conversation, or a refusal when
      // that one is mid-run — which reads as "the conversation will not
      // let me work in it".
      var runPath = '/api/run' + (face.session
        ? '?session=' + encodeURIComponent(face.session) : '');
      return post(runPath, {
        task: task,
        mode: face.permissions,
        pin: face.providerPin || null,
        attachments: face.attachments.map(function (file) {
          return {
          name: file.name,
          text: file.text,
          parser: file.parser || '',
          pages: file.pages == null ? null : file.pages,
          units: file.units == null ? null : file.units,
          job_id: file.job_id || '',
          reason: file.reason || '',
          truncated: file.truncated || null
        };
        })
      });
    }).then(function (reply) {
      if (reply.status === 200) {
        face.running = true;
        face.driving = face.session;
        face.stop = null;
        face.attachments = [];
        paintAttachments();
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
    // The seat names the session it belongs to, and the answer has to
    // name it too: the bridge keeps one seat per session, and a
    // session-less post is answered against the face's launch session —
    // which is a different session the moment a channel peer's run is
    // driving, and the seat then stands with the run hung behind it.
    var seat = face.seat || {};
    var path = '/api/approve' + (seat.session ? '?session=' + encodeURIComponent(seat.session) : '');
    post(path, { decision: decision }).then(function (reply) {
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
      if (document.body.dataset.aside !== 'open') face.asideBeforeObject = true;
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
      // An object that opened the dock closes it again. A dock the
      // owner had open before the object stays open: the column is
      // theirs, not the object's.
      if (face.asideBeforeObject) {
        face.asideBeforeObject = false;
        setAside(false);
      }
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
    // The draft swaps when the replay anchors a different conversation
    // (the boot's launch session, or a resume): same session is a no-op.
    swapDraft(face.session, state.session || '');
    // The return anchor's divider (D6): the first event after the last
    // one the owner saw at the bottom, marked while the history paints.
    face.eventSeq = 0;
    // The stamp is seeded ONCE per conversation per visit: the stream's
    // onopen re-load replays a second time, after the boot's own
    // scroll-to-bottom has already marked everything seen — a fresh
    // seed on that pass would erase the divider the first one drew.
    if (face.seedSession !== face.session) {
      face.seedSession = face.session;
      var seen = seenStamp();
      face.dividerAt = seen != null && !isNaN(Number(seen)) ? Number(seen) : null;
    }
    face.dividerShown = false;
    face.model = state.model || '';
    face.workspace = state.workspace || '';
    face.folder = state.folder || null;
    face.budget = Number(state.budget) || 0;
    face.turnBudget = Number(state.turn_budget) || 0;
    face.running = !!state.running;
    // The session the bridge is executing, which is not always this one:
    // a run in another conversation is a fact about that row, not a
    // reason to disable this composer.
    face.driving = state.driving || '';
    face.dropped = Number(state.dropped) || 0;
    face.rules = state.rules || [];
    face.feedback = state.feedback || {};
    face.seat = state.approval || null;
    if (face.seat) face.seatSince = face.seat.since || Date.now();
    face.events = [];
    face.rows = [];
    face.assistant = null;
    face.strip = null;
    face.stripEl = null;
    face.stripLog = [];
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
    // The back-to-live control lives in the log and must survive its
    // own clearing — sticky at the foot, always the last child.
    if (el.toLive) el.log.appendChild(el.toLive);
    // The empty state is the home when nothing has been said yet. A
    // boot-time skills event is not a conversation, so it must not push
    // the owner into a transcript with no words in it. The shapes are
    // the ones both paths emit — a message carries its role, it is not
    // its own type — and a predicate naming types that never occur hid
    // every resumed history behind the empty state (the log was
    // replayed and invisible; only `send` ever unhid it).
    var talked = (state.events || []).some(function (event) {
      return event.type === 'message' || event.type === 'turn' ||
        event.type === 'tool_call' || event.type === 'tool_result';
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
    paintRunbar();
  }

  // An event recorded between the snapshot and the rebuild rides the
  // stream into a log the rebuild then wipes. The count is the tell: a
  // rebuild short of what the page already held has lost something, and
  // one more read closes the gap rather than leaving the face quietly
  // missing a turn. Three is where it stops asking.
  function load(pass) {
    pass = pass || 0;
    // The read names the session the face is showing: after a resume the
    // launch session's document holds the boot events, not the history,
    // and a session-less read would replay those instead (the launch
    // itself stays the face's own session — a phone that names nothing
    // still gets what this face booted with).
    var asked = face.session
      ? '/api/state?session=' + encodeURIComponent(face.session)
      : '/api/state';
    return fetch(asked, { headers: { accept: 'application/json' } })
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

  // The draft survives a reload, per conversation (D2). A twelve-hour
  // desk should not lose the sentence being typed because the face was
  // refreshed, and two conversations should never share one.
  loadDraft();

  // The conversation's text size survives a reload like the draft does.
  // The stored value is a step index, clamped to the scale in case an
  // older build wrote a longer one.
  try {
    var savedSize = parseInt(window.localStorage.getItem('fleet-chat-size'), 10);
    if (!isNaN(savedSize)) {
      face.chatStep = Math.min(CHAT_SCALES.length - 1, Math.max(0, savedSize));
    }
  } catch (error) { }
  paintChatScale();

  try {
    var savedAside = window.localStorage.getItem('fleet-aside');
    // The dock is the owner's to open, at every width: the work column
    // carries the run's state in the list and the strip carries it in
    // the conversation, so a third column nobody asked for is space
    // taken from the work.
    setAside(savedAside === 'open');
  } catch (error) {
    setAside(false);
  }

  // The list's fold is the owner's too, and read before the first route
  // paints so the page opens in the shape they left it in.
  try {
    face.workFolded = window.localStorage.getItem('fleet-work') === 'collapsed';
  } catch (error) { }
  if (el.asideToggle) {
    el.asideToggle.addEventListener('click', function () {
      // A dock opened by hand is the owner's: closing an object later
      // must not fold it.
      face.asideBeforeObject = false;
      setAside(document.body.dataset.aside === 'closed');
      if (face.panel === 'sessions') paintSessions();
    });
  }
  if (el.dockFold) {
    el.dockFold.addEventListener('click', function () {
      face.asideBeforeObject = false;
      setAside(false);
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
  if (el.convBack) {
    el.convBack.addEventListener('click', function () {
      // The way out: the work list itself where it is a column, and the
      // sheet's close where it is an overlay. A folded list is still the
      // list, so the back control unfolds it; a sheet already closed is
      // opened, because the head's word is "work", not "away".
      if (window.innerWidth <= 1180) {
        if (workFolded()) foldWork(false);
        else if (workOpen()) setWork(false);
        else setWork(true);
        return;
      }
      openWorkSurface();
    });
  }
  if (el.convTitle) {
    el.convTitle.addEventListener('click', function () { toggleWork(); });
  }
  if (el.convNew) {
    el.convNew.addEventListener('click', function () { newConversation(); });
  }
  if (el.convRename) {
    el.convRename.addEventListener('click', function () {
      if (!face.session) return;
      var row = workSelected();
      var asked = window.prompt('name this conversation', (row && row.title) || '');
      if (asked == null) return;
      post('/api/session/title?session=' + encodeURIComponent(face.session), { title: asked })
        .then(function (reply) {
          if (reply.status !== 200) {
            notice('the name was refused',
              (reply.body && reply.body.error) || 'the face said no without a reason');
            return;
          }
          face.sessions = null;
          readSessions(true);
        }).catch(function () { });
    });
  }
  if (el.convArchive) {
    el.convArchive.addEventListener('click', function () {
      if (!face.session) return;
      var row = workSelected();
      var archivedNow = !!(row && row.state === 'archived');
      archiveSession(face.session, !archivedNow);
    });
  }
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

  wireGoalPop();
  setGoalPop(false);

  el.composer.addEventListener('submit', function (event) {
    event.preventDefault();
    // Enter while a run is in flight is not a stop: the task is empty
    // and the intent is unclear, so send() declines. The button is the
    // explicit control.
    send();
  });
  // The conversation's text size: two presses in its head, and the same
  // two keys from the composer — the owner is reading the reply while
  // typing, so the keys live where the hands are. Alt+arrows are the
  // browser's own history keys, so this claims them only inside the
  // composer.
  if (el.chatSmaller) {
    el.chatSmaller.addEventListener('click', function () { stepChatScale(-1); });
  }
  if (el.chatBigger) {
    el.chatBigger.addEventListener('click', function () { stepChatScale(1); });
  }
  el.composer.addEventListener('keydown', function (event) {
    if (!event.altKey || event.metaKey || event.ctrlKey) return;
    if (event.key === 'ArrowUp') {
      event.preventDefault();
      stepChatScale(1);
    } else if (event.key === 'ArrowDown') {
      event.preventDefault();
      stepChatScale(-1);
    }
  });
  if (el.send) {
    el.send.addEventListener('click', function (event) {
      event.preventDefault();
      if (face.running || !!face.seat) {
        stopRun();
        return;
      }
      send();
    });
  }

  // The composer's run policy: the permissions control and the route
  // picker. A chat's choices live on the face and ride its next run;
  // "auto" is the empty pin, and the destructive class still asks under
  // autonomous.
  function paintRunbar() {
    if (!el.runMode && !el.runProvider) return;
    // Full access is a grant only a working folder can bound: the option
    // is disabled without one, and a mode left on full from before the
    // folder was cleared falls back to ask rather than arming a run the
    // server would refuse.
    var full = el.runMode ? el.runMode.querySelector('option[value="full"]') : null;
    if (full) full.disabled = !face.folder;
    if (!face.folder && face.permissions === 'full') face.permissions = 'ask';
    if (el.runMode) el.runMode.value = face.permissions;
    var modes = {
      ask: 'every tool call asks, except the rules you have remembered',
      auto: 'everything runs except the destructive class, which still asks',
      full: 'nothing asks except the always-ask class \u2014 a standing risk'
    };
    if (el.runPerms) {
      el.runPerms.dataset.mode = face.permissions;
      el.runPerms.title = modes[face.permissions] || 'how much runs before asking';
      if (!face.folder) el.runPerms.title += '; full access needs a working folder';
    }
    if (el.runProvider) {
      el.runProvider.value = face.providerPin || '';
      el.runProvider.disabled = !face.providers;
      // With the field collapsed to its icon the pin still has to show,
      // so the icon carries it.
      var route = el.runProvider.closest('.runbar-field');
      if (route) route.dataset.active = face.providerPin ? 'true' : 'false';
    }
    // The turn budget, while a run is live: the honest remaining count
    // against the budget the state document named (D4). Nothing renders
    // when there is no budget or no live run - the field is run
    // telemetry, not decoration.
    if (el.runTurns) {
      var live = face.running && face.turn > 0 && face.turnBudget > 0;
      el.runTurns.hidden = !live;
      if (live) {
        var left = Math.max(0, face.turnBudget - face.turn);
        el.runTurns.textContent = left + ' turns left';
        el.runTurns.dataset.low = left <= 10 ? 'true' : 'false';
      }
    }
    paintFolder();
    paintGoal();
  }

  // The session's standing objective, always present (D1): unset is the
  // quiet affordance that teaches goals exist; set is one ellipsized
  // line with the whole text on hover. The popover carries the write.
  function paintGoal() {
    if (!el.runGoal || !el.runGoalBtn) return;
    el.runGoal.hidden = false;
    if (!face.goal) {
      el.runGoalBtn.textContent = 'goal';
      el.runGoalBtn.classList.add('unset');
      el.runGoalBtn.title = 'set this conversation\u2019s goal \u2014 it rides every run';
      return;
    }
    el.runGoalBtn.classList.remove('unset');
    el.runGoalBtn.textContent = face.goal;
    el.runGoalBtn.title = 'the session\u2019s goal: ' + face.goal +
      ' \u00b7 /goal clear drops it';
  }

  // The goal popover (D1): the full text, and Set/Replace/Clear riding
  // /api/goal — the same route /goal rides, so the command and the chip
  // agree because they write through one seam.
  function goalPost(text, onDone) {
    var path = '/api/goal' + (face.session ? '?session=' + encodeURIComponent(face.session) : '');
    post(path, { text: text }).then(function (reply) {
      if (reply.status === 200) {
        face.goal = reply.body.text || null;
        paintGoal();
        if (onDone) onDone();
        return;
      }
      notice('the goal was refused',
        (reply.body && reply.body.error) || 'the face said no without a reason');
    }).catch(function () {
      notice('the goal did not cross', 'the server did not answer');
    });
  }

  function setGoalPop(open) {
    if (!el.goalPop) return;
    el.goalPop.hidden = !open;
    if (el.runGoalBtn) {
      el.runGoalBtn.setAttribute('aria-expanded', open ? 'true' : 'false');
    }
    if (!open) return;
    if (el.goalPopState) {
      el.goalPopState.textContent = face.goal ? 'set' : 'not set';
    }
    if (el.goalPopInput) {
      el.goalPopInput.value = face.goal || '';
    }
    if (el.goalPopClear) el.goalPopClear.hidden = !face.goal;
    if (el.goalPopSet) {
      el.goalPopSet.textContent = face.goal ? 'replace it' : 'set the goal';
    }
    if (el.goalPopInput && el.goalPopInput.focus) el.goalPopInput.focus();
  }

  function wireGoalPop() {
    if (el.runGoalBtn) {
      el.runGoalBtn.addEventListener('click', function () {
        setGoalPop(el.goalPop && el.goalPop.hidden);
      });
    }
    if (el.goalPopSet) {
      el.goalPopSet.addEventListener('click', function () {
        var text = (el.goalPopInput.value || '').trim();
        if (!text) {
          notice('the goal is a sentence', 'one line \u2014 what is this conversation for?');
          return;
        }
        goalPost(text, function () { setGoalPop(false); });
      });
    }
    if (el.goalPopClear) {
      el.goalPopClear.addEventListener('click', function () {
        goalPost('', function () { setGoalPop(false); });
      });
    }
    if (el.goalPopClose) {
      el.goalPopClose.addEventListener('click', function () { setGoalPop(false); });
    }
  }

  // The working folder: its basename when the chat has one, "no folder"
  // when it does not. The full path rides the title, because the field
  // is one value wide in a crowded row.
  function paintFolder() {
    if (!el.runFolder) return;
    el.runFolder.textContent = face.folder ? face.folder.name : 'no folder';
    el.runFolder.dataset.state = face.folder ? 'set' : 'none';
    el.runFolder.title = face.folder
      ? 'this chat works in ' + face.folder.path
      : 'no working folder: the filesystem tools are unavailable until one is set';
    var field = el.runFolder.closest('.runbar-field');
    if (field) field.dataset.active = face.folder ? 'true' : 'false';
  }

  // The pin's choices are the roster's enabled providers, read once off
  // the settings catalog and cached on the face.
  function readProviders(force) {
    var now = Date.now();
    if (!force && face.providers && now - face.providersAt < 60000) return Promise.resolve();
    face.providersAt = now;
    return fetch('/api/settings', { headers: { accept: 'application/json' } })
      .then(function (reply) { return reply.json(); })
      .then(function (doc) {
        face.providers = ((doc.catalog && doc.catalog.providers) || [])
          .filter(function (provider) { return provider.enabled; });
        var select = el.runProvider;
        while (select.options.length > 1) select.remove(1);
        face.providers.forEach(function (provider) {
          var option = node('option', null, provider.id);
          option.value = provider.id;
          select.appendChild(option);
        });
        if (face.providerPin && !face.providers.some(function (provider) {
          return provider.id === face.providerPin;
        })) {
          face.providerPin = '';
        }
        paintRunbar();
      })
      .catch(function () { });
  }

  if (el.runMode) {
    el.runMode.addEventListener('change', function () {
      face.permissions = el.runMode.value;
      paintRunbar();
    });
  }
  if (el.runProvider) {
    el.runProvider.addEventListener('change', function () {
      face.providerPin = el.runProvider.value;
      paintRunbar();
    });
  }
  paintRunbar();
  readProviders(false);

  // The folder picker. It browses the whole hierarchy through the
  // picker's own absolute listing — not the pane's contained tree —
  // opening at the chat's folder, ascending to the filesystem root, and
  // taking a typed path whose existing subfolders are the suggestions.
  // The choice applies to the next run only: a run in flight already
  // accepted the folder it is working in. A change is one line in the
  // transcript, recorded by the server and replayed like any other event.
  var folderBrowse = { path: '' };
  // The change in flight, so a send that follows a click waits for the
  // server to have the choice rather than racing it on a second
  // connection.
  var folderSync = Promise.resolve();

  // `folder` is the pane's rooted form of the grant, null removes it,
  // and the launch directory itself is sent by its own path because
  // choosing it is a grant like any other.
  function chooseFolder(folder) {
    var body = { folder: folder || null };
    if (face.session) body.session = face.session;
    folderSync = post('/api/folder', body).then(function (reply) {
      if (reply.status !== 200) {
        notice('the folder was refused',
          (reply.body && reply.body.error) || 'the face said no without a reason');
        return;
      }
      // The recorded event rides the stream and is the authority; this
      // reply is the same line, used at once so the button does not
      // wait for the stream to catch up.
      if (reply.body && reply.body.type === 'folder') {
        face.folder = reply.body.path
          ? { path: reply.body.path, name: reply.body.name, rel: reply.body.rel || '' }
          : null;
      }
      paintFolder();
      toggleFolderPop(false);
    }).catch(function () {
      notice('the folder was refused', 'the face did not answer');
    });
    return folderSync;
  }

  function parentOf(path) {
    if (!path || path === '/') return '/';
    var trimmed = path.replace(/\/+$/, '');
    var cut = trimmed.lastIndexOf('/');
    return cut <= 0 ? '/' : trimmed.slice(0, cut);
  }

  function joinPath(dir, name) {
    return dir === '/' ? '/' + name : dir.replace(/\/+$/, '') + '/' + name;
  }

  function listFolders(path) {
    return fetch('/api/tree?abs=1&path=' + encodeURIComponent(path), {
      headers: { accept: 'application/json' }
    }).then(function (reply) {
      return reply.json();
    });
  }

  // Paint one listing: where it is, the way up, and its folders — only
  // those starting with `prefix` when completion asked for a subset.
  function paintFolderList(doc, prefix) {
    var list = el.folderPopList;
    list.textContent = '';
    var here = doc.path || folderBrowse.path || '/';
    folderBrowse.path = here;
    el.folderPopPath.textContent = here;
    var up = node('button', 'folder-entry', '\u2191 up');
    up.type = 'button';
    up.disabled = here === '/';
    up.addEventListener('click', function () {
      browseFolder(parentOf(here));
    });
    list.appendChild(up);
    if (doc.refused) {
      list.appendChild(node('span', 'folder-empty', (doc.refused || []).join('; ')));
      return;
    }
    var wanted = (prefix || '').toLowerCase();
    var dirs = (doc.entries || []).filter(function (entry) {
      return entry.kind === 'dir' && (!wanted || entry.name.toLowerCase().indexOf(wanted) === 0);
    });
    dirs.forEach(function (entry) {
      var button = node('button', 'folder-entry', entry.name + '/');
      button.type = 'button';
      button.addEventListener('click', function () {
        browseFolder(joinPath(here, entry.name));
      });
      list.appendChild(button);
    });
    if (!dirs.length) {
      list.appendChild(node('span', 'folder-empty',
        wanted ? 'no folder here starts with that' : 'no subfolders'));
    }
  }

  function browseFolder(path) {
    var target = path || face.workspace || '/';
    folderBrowse.path = target;
    el.folderPopPath.textContent = target;
    if (el.folderPopInput) {
      el.folderPopInput.value = target === '/' ? '/' : target.replace(/\/+$/, '') + '/';
    }
    listFolders(target).then(function (doc) {
      paintFolderList(doc, '');
    }).catch(function () {
      notice('the folder could not be read', 'the face did not answer');
    });
  }

  // Completion from what was typed: the typed path's own subfolders when
  // it exists (a trailing slash asks for exactly that), else the nearest
  // existing ancestor's subfolders that start with the first segment the
  // disk does not have. One listing per keystroke at most — the fetch is
  // debounced and every keystroke re-arms it (KI-244's lesson), and a
  // listing that lands after a newer keystroke is dropped.
  var folderTypeTimer = null;
  var folderTypeSeq = 0;
  function suggestFolders(typed) {
    var seq = ++folderTypeSeq;
    var text = (typed || '').trim();
    if (!text) {
      browseFolder('/');
      return;
    }
    if (text.charAt(0) !== '/') text = '/' + text;
    var exact = text.length > 1 && /\/$/.test(text);
    var base = exact ? (text.replace(/\/+$/, '') || '/') : parentOf(text);
    var prefix = exact ? '' : text.slice(text.lastIndexOf('/') + 1);
    (function attempt(dir, want, depth) {
      listFolders(dir).then(function (doc) {
        if (seq !== folderTypeSeq) return;
        if (doc.refused && dir !== '/' && depth < 16) {
          attempt(parentOf(dir), dir.slice(dir.lastIndexOf('/') + 1), depth + 1);
          return;
        }
        paintFolderList(doc, want);
      }).catch(function () { });
    })(base, prefix, 0);
  }

  function toggleFolderPop(open) {
    if (!el.folderPop) return;
    var showing = open === undefined ? el.folderPop.hidden : open;
    el.folderPop.hidden = !showing;
    el.runFolder.setAttribute('aria-expanded', showing ? 'true' : 'false');
    if (showing) {
      browseFolder(face.folder && face.folder.path ? face.folder.path : (face.workspace || '/'));
      if (el.folderPopInput) el.folderPopInput.focus();
    }
  }

  // The typed path wins when there is one; a trailing slash is the
  // browsing form, not part of the folder's name.
  function useTypedFolder() {
    var typed = el.folderPopInput ? el.folderPopInput.value.trim() : '';
    var chosen = typed ? (typed === '/' ? '/' : typed.replace(/\/+$/, '')) : folderBrowse.path;
    chooseFolder(chosen || face.workspace);
  }

  if (el.runGoalBtn) {
    el.runGoalBtn.addEventListener('click', function () {
      notice('this session\u2019s goal', face.goal
        ? face.goal + ' \u00b7 /goal clear drops it'
        : 'no goal is set \u00b7 /goal <text> sets one');
    });
  }
  if (el.runFolder) {
    el.runFolder.addEventListener('click', function (event) {
      event.preventDefault();
      toggleFolderPop();
    });
  }
  if (el.folderPopInput) {
    el.folderPopInput.addEventListener('input', function () {
      if (folderTypeTimer) clearTimeout(folderTypeTimer);
      var typed = el.folderPopInput.value;
      folderTypeTimer = setTimeout(function () {
        suggestFolders(typed);
      }, 200);
    });
    el.folderPopInput.addEventListener('keydown', function (event) {
      if (event.key === 'Enter') {
        event.preventDefault();
        useTypedFolder();
      }
    });
  }
  if (el.folderUse) {
    el.folderUse.addEventListener('click', useTypedFolder);
  }
  if (el.folderReset) {
    el.folderReset.addEventListener('click', function () { chooseFolder(null); });
  }
  document.addEventListener('keydown', function (event) {
    if (event.key === 'Escape' && el.folderPop && !el.folderPop.hidden) toggleFolderPop(false);
  });
  paintFolder();

  // The document picker: the workspace tree, files a parser owns, one
  // level at a time. A clean Office file is read locally; a scan or a
  // media file answers with what would be sent and asks first.
  var docBrowse = { rel: '' };
  var docPending = null;
  var DOC_RE = /\.(docx?|docm|pptx?|pptm|xlsx?|xlsm|csv|html?|pdf|png|jpe?g|tiff?|webp|bmp|eml|msg|oft|mp3|wav|m4a|flac|mpga|oga|ogg|mp4|mov|avi|mkv|vsdx?|vsd|parquet)$/i;

  function docConsent(open) {
    if (!el.docConsentSend) return;
    el.docConsentSend.hidden = !open;
    el.docConsentCancel.hidden = !open;
  }

  function toggleDocPop(open) {
    if (!el.docPop) return;
    var showing = open === undefined ? el.docPop.hidden : open;
    el.docPop.hidden = !showing;
    el.docOpen.setAttribute('aria-expanded', showing ? 'true' : 'false');
    if (showing) {
      docConsent(false);
      el.docPopNote.hidden = true;
      browseDocs(face.folder ? (face.folder.rel || '') : '');
    }
  }

  function docPath(rel, name) {
    return rel ? rel + '/' + name : name;
  }

  function browseDocs(rel) {
    docBrowse.rel = rel || '';
    el.docPopPath.textContent = docBrowse.rel || 'the workspace root';
    fetch('/api/tree?path=' + encodeURIComponent(docBrowse.rel), {
      headers: { accept: 'application/json' }
    }).then(function (reply) {
      return reply.json();
    }).then(function (doc) {
      var list = el.docPopList;
      list.textContent = '';
      var up = node('button', 'folder-entry', '\u2191 up');
      up.type = 'button';
      up.disabled = !docBrowse.rel;
      up.addEventListener('click', function () {
        var parts = docBrowse.rel.split('/').filter(Boolean);
        parts.pop();
        browseDocs(parts.join('/'));
      });
      list.appendChild(up);
      if (doc.refused) {
        list.appendChild(node('span', 'folder-empty', (doc.refused || []).join('; ')));
        return;
      }
      var entries = doc.entries || [];
      entries.filter(function (entry) { return entry.kind === 'dir'; }).forEach(function (entry) {
        var button = node('button', 'folder-entry', entry.name + '/');
        button.type = 'button';
        button.addEventListener('click', function () { browseDocs(docPath(docBrowse.rel, entry.name)); });
        list.appendChild(button);
      });
      entries.filter(function (entry) {
        return entry.kind === 'file' && DOC_RE.test(entry.name);
      }).forEach(function (entry) {
        var button = node('button', 'folder-entry file', entry.name);
        button.type = 'button';
        button.title = 'attach ' + entry.name;
        button.addEventListener('click', function () {
          attachDocument(docPath(docBrowse.rel, entry.name));
        });
        list.appendChild(button);
      });
      if (!entries.some(function (entry) {
        return entry.kind === 'dir' || (entry.kind === 'file' && DOC_RE.test(entry.name));
      })) {
        list.appendChild(node('span', 'folder-empty', 'no documents or subfolders here'));
      }
    }).catch(function () {
      notice('the workspace could not be read', 'the face did not answer');
    });
  }

  // What the local lane will do with this path, named before the read:
  // the picker states the parser, not only the chip after it.
  function docParserHint(rel) {
    var name = String(rel || '').split('/').pop().toLowerCase();
    var ext = name.indexOf('.') === -1 ? '' : name.split('.').pop();
    if (ext === 'docx' || ext === 'docm') {
      return 'reading locally \u00b7 OOXML document (parser: local:docx)';
    }
    if (ext === 'pptx' || ext === 'pptm') {
      return 'reading locally \u00b7 OOXML slides (parser: local:pptx)';
    }
    if (ext === 'xlsx' || ext === 'xlsm') {
      return 'reading locally \u00b7 OOXML sheets (parser: local:xlsx)';
    }
    if (ext === 'csv') return 'reading locally \u00b7 csv (parser: local:csv)';
    if (ext === 'html' || ext === 'htm') return 'reading locally \u00b7 html (parser: local:html)';
    if (ext === 'doc' || ext === 'xls' || ext === 'ppt') {
      return 'reading locally with LibreOffice, safe mode (parser: local:libreoffice)';
    }
    if (ext === 'pdf') {
      return 'reading locally with pdftotext (parser: local:pdftotext) \u00b7 if it is a scan, ' +
        'Nace will be asked before anything is sent';
    }
    if (ext === 'md' || ext === 'txt' || ext === 'log' || ext === 'xml' || ext === 'json') {
      return 'reading locally (parser: local:text)';
    }
    return 'this format needs Nace perception; you will be asked before it is sent';
  }

  function attachDocument(rel, consent) {
    if (face.attachments.length >= 5) {
      notice('at most five attachments', 'remove one before adding another');
      return;
    }
    docConsent(false);
    el.docPopNote.hidden = false;
    el.docPopNote.textContent = consent
      ? 'reading with Nace perception\u2026'
      : docParserHint(rel);
    var run = function () {
      post('/api/attach', { path: rel, consent: !!consent }).then(function (reply) {
        if (reply.status !== 200) {
          notice('the document was refused',
            (reply.body && reply.body.error) || 'the face said no without a reason');
          el.docPopNote.hidden = true;
          return;
        }
        var doc = reply.body || {};
        if (doc.type === 'attach_needs') {
          docPending = rel;
          el.docPopNote.hidden = false;
          el.docPopNote.textContent = (doc.reason || 'this file needs a hosted parser') +
            ' \u2014 sending it to Nace perception sends the document to a second party.';
          docConsent(true);
          return;
        }
        if (doc.type === 'attachment') {
          face.attachments.push({
            name: doc.name,
            text: doc.text,
            parser: doc.parser || '',
            pages: doc.pages,
            units: doc.units,
            job_id: doc.job_id || '',
            reason: doc.reason || '',
            truncated: doc.truncated || null
          });
          paintAttachments();
          toggleDocPop(false);
        }
      }).catch(function () {
        notice('the document was refused', 'the face did not answer');
        el.docPopNote.hidden = true;
      });
    };
    // The parser statement is painted before the read starts; starting in
    // the same task would show it only after the answer.
    window.requestAnimationFrame(function () { window.setTimeout(run, 0); });
  }

  if (el.docOpen) {
    el.docOpen.addEventListener('click', function (event) {
      event.preventDefault();
      toggleDocPop();
    });
  }
  if (el.docConsentSend) {
    el.docConsentSend.addEventListener('click', function () {
      if (docPending) attachDocument(docPending, true);
    });
  }
  if (el.docConsentCancel) {
    el.docConsentCancel.addEventListener('click', function () {
      docPending = null;
      docConsent(false);
      el.docPopNote.hidden = true;
    });
  }
  document.addEventListener('keydown', function (event) {
    if (event.key === 'Escape' && el.docPop && !el.docPop.hidden) toggleDocPop(false);
  });

  // Attachments: text only, five at most, each 256 KB — the same bounds
  // the server enforces, said here so a refused file is named before it
  // is read rather than after the run is refused.
  function paintAttachments() {
    if (!el.attachChips) return;
    el.attachChips.textContent = '';
    el.attachChips.hidden = !face.attachments.length;
    face.attachments.forEach(function (file, index) {
      var chip = node('span', 'attach-chip');
      var meta = [bytes(file.text.length)];
      if (file.parser) meta.unshift(file.parser);
      if (file.pages) meta.push(file.pages + ' pages');
      if (file.units) meta.push(file.units + ' units');
      if (file.job_id) meta.push('job ' + file.job_id);
      if (file.truncated) {
        meta.push('truncated \u00b7 retained ' + file.truncated.retained +
          ' of ' + file.truncated.original);
      }
      chip.appendChild(node('span', null, file.name + ' \u00b7 ' + meta.join(' \u00b7 ')));
      var drop = node('button', 'attach-drop', '\u00d7');
      drop.type = 'button';
      drop.title = 'remove ' + file.name;
      drop.addEventListener('click', function () {
        face.attachments.splice(index, 1);
        paintAttachments();
      });
      chip.appendChild(drop);
      el.attachChips.appendChild(chip);
    });
  }

  function attachFiles(files) {
    Array.prototype.forEach.call(files, function (file) {
      if (face.attachments.length >= 5) {
        notice('at most five files can be attached', file.name + ' was not added');
        return;
      }
      if (file.size > 256 * 1024) {
        notice('the file is larger than the 256 KB bound', file.name + ' was not added');
        return;
      }
      if (!/\.(md|txt|rs|js|json|toml|yaml|yml|css|html|py|sh|ts|tsx|jsx)$/i.test(file.name)) {
        notice('only text files can be attached', file.name + ' was not added');
        return;
      }
      var reader = new FileReader();
      reader.onload = function () {
        face.attachments.push({ name: file.name, text: String(reader.result || '') });
        paintAttachments();
      };
      reader.readAsText(file);
    });
  }

  if (el.attach) {
    el.attach.addEventListener('click', function () { el.attachInput.click(); });
  }
  if (el.attachInput) {
    el.attachInput.addEventListener('change', function () {
      attachFiles(el.attachInput.files);
      el.attachInput.value = '';
    });
  }
  // A paste carrying files is an attach (D4): the same pipeline the
  // machine-file picker rides — one pipeline, one set of bounds (five
  // files, 256 KB, text). A paste carrying text inserts text, as
  // always; an image names its path honestly rather than vanishing.
  if (el.task) {
    el.task.addEventListener('paste', function (event) {
      var clipboard = event.clipboardData;
      if (!clipboard) return;
      var files = clipboard.files;
      if (files && files.length) {
        var carried = 0;
        Array.prototype.forEach.call(files, function (file) { carried += 1; });
        if (carried) {
          event.preventDefault();
          attachFiles(files);
        }
        return;
      }
      var items = clipboard.items;
      if (items) {
        var image = Array.prototype.filter.call(items, function (item) {
          return item.kind === 'file' && /^image\//.test(item.type);
        })[0];
        if (image) {
          event.preventDefault();
          notice('an image cannot ride the composer yet',
            'paste attaches text files through the same bounds as the picker; an image goes \
             through the document surface, which escalates it to Perception');
        }
      }
    });
  }
  paintAttachments();

  el.task.addEventListener('input', function () {
    grow();
    saveDraft();
    typeAhead();
  });
  el.task.addEventListener('keydown', function (event) {
    // The type-ahead owns the keys while it is open, so a command is one
    // keystroke from anywhere in the composer. Enter runs the option the
    // screen is highlighting — the owner's ruling on KI-245: the selected
    // row is what Enter sends, whether the owner typed a command's name
    // or the intent that ranked it. Shift+Enter stays a newline.
    if (face.referenceOpen) {
      if (event.key === 'ArrowDown') { event.preventDefault(); moveReference(1); return; }
      if (event.key === 'ArrowUp') { event.preventDefault(); moveReference(-1); return; }
      if ((event.key === 'Enter' || event.key === 'Tab') && !event.shiftKey) {
        event.preventDefault();
        insertReference(face.referenceAt);
        return;
      }
      if (event.key === 'Escape') { event.preventDefault(); closeReferences(); el.commands.hidden = true; return; }
    }
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
    if (value.charAt(0) !== '/') {
      // The second sigil (D5): a trailing @token opens the reference
      // listbox over the reads the face already makes. Anything else
      // closes both listboxes.
      var at = value.match(/(^|\s)@([^\s@]*)$/);
      if (at) { openReferences(at[2]); return; }
      closeReferences();
      closeCommands();
      return;
    }
    closeReferences();
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
    // A tool card has its own detail — what ran, how long, what came
    // back — and a raw-record button inside it. The generic inspector
    // would be a second, less useful answer to the same press.
    if (opened.classList.contains('tool')) return;
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
    if (face.stick) clearUnread();
  });
  if (el.toLive) {
    el.toLive.addEventListener('click', function () {
      face.stick = true;
      el.log.scrollTop = el.log.scrollHeight;
      clearUnread();
    });
  }

  // ------------------------------------------------- the hash addresses

  // ------------------------------------------------------------ the work surface

  // The archive is the work list now: every conversation and background
  // run under this home, grouped by what needs the owner rather than by
  // where it was born. The state words are the bridge's own derivation —
  // "needs-approval", "paused", "working", "active", "completed",
  // "capped", "stopped", "interrupted", "new" — so the page cannot
  // disagree with the archive about what a row is.
  var WORK_STATE_WORD = {
    'needs-approval': 'approval waiting',
    paused: 'paused by you',
    working: 'working',
    active: 'active',
    completed: 'completed',
    capped: 'capped',
    stopped: 'stopped by you',
    'handed back': 'handed back',
    archived: 'archived',
    interrupted: 'interrupted',
    new: 'new'
  };

  // The states that mean the owner is the one holding the work up.
  function workNeeds(state) {
    return state === 'needs-approval' || state === 'paused';
  }

  function workStateWord(state) {
    return WORK_STATE_WORD[state] || state || 'unknown';
  }

  // The archive: the owner's one end for a session. The loop's stops
  // (a handback, a cap) are pauses; this is the verb that ends one.
  function archiveSession(id, archived) {
    post('/api/session/archive', { session: id, archived: !!archived })
      .then(function (reply) {
        if (reply.status !== 200) {
          notice('the archive was refused',
            (reply.body && reply.body.error) || 'the face said no without a reason');
          return;
        }
        face.sessions = null;
        readSessions(true);
      }).catch(function () { });
  }

  // The archive read: the log heads as summaries, newest first, with the
  // session this face anchors named and every row's derived state.
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
        if (face.panel === 'sessions') paintWork();
        paintConvHead();
      })
      .catch(function () { });
  }

  function workRows() {
    return (face.sessions && face.sessions.sessions) || [];
  }

  function conversationName(id) {
    var row = workRows().filter(function (row) { return row.id === id; })[0];
    return (row && row.title) || sessionLabel(id);
  }

  function workSelected() {
    return workRows().filter(function (row) { return row.id === face.session; })[0] || null;
  }

  function workAge(row) {
    var stamp = row.created;
    if (!stamp) return 'unknown';
    var then = Date.parse(stamp);
    if (isNaN(then)) return sessionShortDate(stamp);
    var span = Math.max(0, Date.now() - then);
    var minutes = Math.floor(span / 60000);
    if (minutes < 1) return 'just now';
    if (minutes < 60) return minutes + 'm';
    var hours = Math.floor(minutes / 60);
    if (hours < 24) return hours + 'h';
    return Math.floor(hours / 24) + 'd';
  }

  // The project a run was born in: the workspace's last segment. A run
  // with no workspace is its own group, named rather than hidden.
  function sessionProject(workspace) {
    if (!workspace) return 'no project';
    var segments = String(workspace).split('/');
    return segments[segments.length - 1] || 'no project';
  }

  function sessionLabel(id) {
    var text = String(id || '');
    return text.length > 13 ? text.slice(0, 13) + '\u2026' : text;
  }

  function sessionShortDate(stamp) {
    var text = String(stamp || '');
    return text.length >= 16 ? text.slice(5, 10) + ' ' + text.slice(11, 16) : text;
  }

  function sessionGroups(rows) {
    var map = {};
    rows.forEach(function (row) {
      var key = sessionProject(row.workspace);
      (map[key] = map[key] || []).push(row);
    });
    var groups = Object.keys(map).map(function (name) {
      var grouped = map[name].slice().sort(function (left, right) {
        return String(right.created || '').localeCompare(String(left.created || ''));
      });
      return {
        name: name,
        rows: grouped,
        newest: grouped.length ? String(grouped[0].created || '') : ''
      };
    });
    groups.sort(function (left, right) {
      if (left.name === 'no project') return 1;
      if (right.name === 'no project') return -1;
      return right.newest.localeCompare(left.newest) || left.name.localeCompare(right.name);
    });
    return groups;
  }

  function sessionMatchesQuery(row) {
    var asked = (face.sessionQuery || '').trim().toLowerCase();
    if (!asked) return true;
    var channel = (row.channel || []).join(' ');
    return [row.id, row.title, row.workspace, row.model, row.kind, row.job,
      row.class, row.state, channel]
      .join(' ').toLowerCase().indexOf(asked) !== -1;
  }

  // The folded list: a strip that names itself, carries the archive's own
  // count of what needs the owner, and holds the one control that brings
  // the rows back. No row width, no second meaning — the rows are one
  // press away and the conversation holds the rest of the page.
  function workStrip(counts) {
    var expand = node('button', 'work-expand');
    expand.type = 'button';
    expand.setAttribute('aria-expanded', 'false');
    expand.setAttribute('aria-label', 'expand the work list');
    expand.title = 'expand the work list';
    expand.textContent = '\u203a';
    expand.addEventListener('click', function () { foldWork(false); });
    el.workHead.appendChild(expand);
    el.workHead.appendChild(node('span', 'work-folded-label', 'work'));
    var needs = counts ? counts.needs : 0;
    if (needs) {
      var badge = node('span', 'work-folded-badge', String(needs));
      badge.title = needs + (needs === 1 ? ' conversation needs you' : ' conversations need you');
      el.workHead.appendChild(badge);
    }
  }

  // The list, painted: four groups in fixed order, because the owner's
  // question is "what needs me" before "what exists".
  function paintWork() {
    if (!el.workList) return;
    el.workHead.textContent = '';
    el.workList.textContent = '';
    var read = face.sessions;
    // The counts are the document's own (the bridge derives them from the
    // rows' state words); the page never re-derives what the archive
    // already counted. Without a read yet there is nothing honest to
    // show, so the head says so rather than inventing a zero.
    var counts = read && read.counts ? read.counts : null;
    // Folded, the column is a strip: the way back and how much is waiting,
    // and nothing that would need a row's width.
    if (workFolded()) {
      workStrip(counts);
      return;
    }
    var all = workRows();
    var head = node('div', 'work-title');
    var title = node('h2', null, 'work');
    head.appendChild(title);
    var countText = 'reading\u2026';
    if (counts) {
      var mine = counts.yours == null ? counts.total : counts.yours;
      countText = counts.needs
        ? counts.needs + (counts.needs === 1 ? ' needs you' : ' need you')
        : mine + (mine === 1 ? ' conversation' : ' conversations');
    }
    head.appendChild(node('span', 'work-count', countText));
    var fresh = node('button', 'work-new');
    fresh.type = 'button';
    fresh.appendChild(node('span', null, '+'));
    fresh.appendChild(node('span', null, 'New'));
    fresh.title = 'a new conversation';
    fresh.addEventListener('click', function () { newConversation(); });
    head.appendChild(fresh);
    var fold = node('button', 'work-fold');
    fold.type = 'button';
    fold.setAttribute('aria-expanded', 'true');
    fold.setAttribute('aria-label', 'collapse the work list');
    fold.title = 'collapse the work list';
    fold.textContent = '\u2039';
    fold.addEventListener('click', function () { foldWork(true); });
    head.appendChild(fold);
    el.workHead.appendChild(head);
    var search = node('div', 'work-search');
    var box = node('input', null);
    box.type = 'search';
    box.placeholder = 'id, project, model, job';
    box.value = face.sessionQuery || '';
    box.setAttribute('aria-label', 'search work');
    box.addEventListener('input', function () {
      face.sessionQuery = box.value;
      paintWork();
      var again = el.workList && el.workHead.querySelector('.work-search input');
      if (again) {
        again.focus();
        again.setSelectionRange(again.value.length, again.value.length);
      }
    });
    search.appendChild(box);
    el.workHead.appendChild(search);

    if (!read) {
      el.workList.appendChild(node('p', 'board-empty', 'Reading the archive\u2026'));
      return;
    }
    if (read.refused && read.refused.length) {
      el.workList.appendChild(node('p', 'board-empty',
        'The archive could not be read: ' + read.refused.join('; ')));
      return;
    }
    var shown = all.filter(sessionMatchesQuery);
    var groups = {
      needs: shown.filter(function (row) { return workNeeds(row.state); }),
      working: shown.filter(function (row) { return row.state === 'working'; }),
      archived: shown.filter(function (row) { return row.state === 'archived'; }),
      background: shown.filter(function (row) {
        return row.class === 'background' || row.class === 'delegated' || row.class === 'channel';
      }),
      earlier: shown.filter(function (row) {
        return !workNeeds(row.state) && row.state !== 'working' && row.state !== 'archived' &&
          row.class !== 'background' && row.class !== 'delegated' && row.class !== 'channel';
      })
    };

    var groupCount = function (key, rows) {
      return counts ? counts[key] : rows.length;
    };
    if (groups.needs.length) {
      el.workList.appendChild(workGroupHead('needs you', groupCount('needs', groups.needs)));
      groups.needs.forEach(function (row) { el.workList.appendChild(workRow(row)); });
    }
    if (groups.working.length) {
      el.workList.appendChild(workGroupHead('working', groupCount('working', groups.working)));
      groups.working.forEach(function (row) { el.workList.appendChild(workRow(row)); });
    }
    if (groups.background.length) {
      // One quiet line, closed by default: the app's own work is a fact
      // about the home, not a row the owner has to read past.
      var backgroundCount = groupCount('background', groups.background);
      var opened = !!face.showBackground;
      var toggle = node('button', 'work-background');
      toggle.type = 'button';
      toggle.setAttribute('aria-expanded', opened ? 'true' : 'false');
      toggle.title = opened ? 'hide the app\u2019s own runs' : 'show the app\u2019s own runs';
      toggle.appendChild(node('span', null, opened ? '\u25be' : '\u25b8'));
      var label = node('span');
      label.appendChild(node('b', null, 'background'));
      label.appendChild(document.createTextNode(' \u00b7 ' + backgroundCount +
        (backgroundCount === 1 ? ' run' : ' runs') + (opened ? '' : ' \u00b7 hidden')));
      toggle.appendChild(label);
      toggle.addEventListener('click', function () {
        face.showBackground = !face.showBackground;
        paintWork();
      });
      el.workList.appendChild(toggle);
      if (opened) {
        familyLines(groups.background).forEach(function (family) {
          el.workList.appendChild(workFamily(family));
        });
      }
    }
    if (groups.earlier.length) {
      el.workList.appendChild(workGroupHead('earlier', groupCount('earlier', groups.earlier)));
      sessionGroups(groups.earlier).forEach(function (group) {
        var folded = !face.sessionQuery && !!face.sessionFolded[group.name];
        var head = node('button', 'work-group');
        head.type = 'button';
        head.setAttribute('aria-expanded', folded ? 'false' : 'true');
        head.appendChild(node('span', null, (folded ? '\u25b8 ' : '\u25be ') + group.name));
        head.appendChild(node('span', 'work-group-why', String(group.rows.length)));
        head.addEventListener('click', function () {
          face.sessionFolded[group.name] = !face.sessionFolded[group.name];
          paintWork();
        });
        el.workList.appendChild(head);
        if (!folded) group.rows.forEach(function (row) { el.workList.appendChild(workRow(row)); });
      });
    }
    if (groups.archived.length) {
      // One quiet line, closed by default, like the background runs: the
      // archive is the one end a session comes to, and its rows stay
      // findable — never invisible.
      var archivedCount = groupCount('archived', groups.archived);
      var archivedOpen = !!face.showArchived;
      var archivedToggle = node('button', 'work-background');
      archivedToggle.type = 'button';
      archivedToggle.setAttribute('aria-expanded', archivedOpen ? 'true' : 'false');
      archivedToggle.title = archivedOpen ? 'hide archived sessions' : 'show archived sessions';
      archivedToggle.appendChild(node('span', null, archivedOpen ? '\u25be' : '\u25b8'));
      var archivedLabel = node('span');
      archivedLabel.appendChild(node('b', null, 'archived'));
      archivedLabel.appendChild(document.createTextNode(' \u00b7 ' + archivedCount +
        (archivedCount === 1 ? ' session' : ' sessions')));
      archivedToggle.appendChild(archivedLabel);
      archivedToggle.addEventListener('click', function () {
        face.showArchived = !face.showArchived;
        paintWork();
      });
      el.workList.appendChild(archivedToggle);
      if (archivedOpen) {
        groups.archived.forEach(function (row) { el.workList.appendChild(workRow(row)); });
      }
    }
    if (!shown.length) {
      el.workList.appendChild(node('p', 'board-empty',
        'Nothing under this home matches' +
        (face.sessionQuery ? ' \u201c' + face.sessionQuery + '\u201d' : '') + '.'));
    }
  }

  // paintSessions is the name every existing trigger calls; one painter,
  // so a stale name cannot paint a second list.
  function paintSessions() { paintWork(); }

  function workGroupHead(name, count) {
    var head = node('div', 'work-group');
    head.appendChild(node('span', null, name));
    head.appendChild(node('span', 'work-group-why', String(count)));
    return head;
  }

  // One row: the state in words, the age, the project, and — for
  // background work — the job it belongs to. Every row opens the same
  // way, whatever its state: the conversation beside it, with a composer.
  function workRow(row) {
    var button = node('button', 'work-row');
    button.type = 'button';
    button.dataset.state = row.state || '';
    // Whose work the row is, in the DOM: the list groups by it, so a
    // test (and an audit) can read the grouping rather than infer it
    // from a title.
    button.dataset.class = row.class || 'yours';
    if (workNeeds(row.state)) button.dataset.attention = 'true';
    if (face.session === row.id) button.setAttribute('aria-current', 'true');
    button.appendChild(node('span', 'work-bar'));
    var body = node('span', 'work-body');
    body.appendChild(node('span', 'work-name', row.title || sessionLabel(row.id)));
    // The content match's line, when a query drove the row (D9): the
    // row names why it matched, the whole line clipped — never a
    // summary a model wrote.
    if (row.matched) {
      body.appendChild(node('span', 'work-matched', row.matched));
    }
    var meta = node('span', 'work-meta');
    var state = node('span', 'work-state', workStateWord(row.state));
    state.dataset.state = row.state || '';
    meta.appendChild(state);
    meta.appendChild(node('span', null, '\u00b7'));
    meta.appendChild(node('span', null, sessionProject(row.workspace)));
    meta.appendChild(node('span', null, '\u00b7'));
    meta.appendChild(node('span', null, workAge(row)));
    if (row.job) meta.appendChild(node('span', 'work-badge', row.job));
    else if (row.kind === 'subagent') meta.appendChild(node('span', 'work-badge', 'child'));
    else if (row.channel && row.channel.length) {
      meta.appendChild(node('span', 'work-badge', row.channel[0]));
    }
    body.appendChild(meta);
    // The owner's end: archive on any live row, unarchive on an
    // archived one. A chip, not a state change of the row's own click —
    // opening the conversation stays the row's primary act.
    var archive = node('span', 'work-archive',
      row.state === 'archived' ? 'unarchive' : 'archive');
    archive.setAttribute('role', 'button');
    archive.tabIndex = 0;
    var archiveAsk = function (event) {
      event.stopPropagation();
      archiveSession(row.id, row.state !== 'archived');
    };
    archive.addEventListener('click', archiveAsk);
    archive.addEventListener('keydown', function (event) {
      if (event.key === 'Enter' || event.key === ' ') {
        event.preventDefault();
        archiveAsk(event);
      }
    });
    body.appendChild(archive);
    button.appendChild(body);
    button.addEventListener('click', function () { openWork(row.id); });
    return button;
  }

  // Background work is one line per family until asked about: fourteen
  // nightly jobs are not fourteen things the owner must read.
  function familyLines(rows) {
    var map = {};
    rows.forEach(function (row) {
      var name = row.job
        || (row.class === 'delegated' ? 'subagents'
          : row.class === 'channel' ? 'channel \u00b7 ' + ((row.channel && row.channel[0]) || 'peer')
            : 'runs');
      (map[name] = map[name] || []).push(row);
    });
    return Object.keys(map).map(function (name) {
      var family = map[name];
      var newest = family.slice().sort(function (left, right) {
        return String(right.created || '').localeCompare(String(left.created || ''));
      })[0];
      return {
        name: name,
        rows: family,
        newest: newest,
        open: family.filter(function (row) {
          return row.state === 'working' || row.state === 'active' || workNeeds(row.state);
        }).length
      };
    }).sort(function (left, right) {
      return String(right.newest.created || '').localeCompare(String(left.newest.created || ''));
    });
  }

  function workFamily(family) {
    var expanded = !!face.workExpanded[family.name];
    var wrap = node('div', 'work-family-wrap');
    var head = node('button', 'work-family');
    head.type = 'button';
    head.setAttribute('aria-expanded', expanded ? 'true' : 'false');
    head.appendChild(node('span', null, expanded ? '\u25be' : '\u25b8'));
    var label = node('span');
    label.appendChild(node('b', null, family.name));
    label.appendChild(document.createTextNode(' \u00b7 ' + family.rows.length +
      (family.rows.length === 1 ? ' run' : ' runs') +
      (family.open ? ' \u00b7 ' + family.open + ' active' : '') +
      ' \u00b7 last ' + workStateWord(family.newest.state)));
    head.appendChild(label);
    head.addEventListener('click', function () {
      face.workExpanded[family.name] = !face.workExpanded[family.name];
      paintWork();
    });
    wrap.appendChild(head);
    if (expanded) {
      var list = node('div', 'work-family-rows');
      family.rows.forEach(function (row) { list.appendChild(workRow(row)); });
      wrap.appendChild(list);
    }
    return wrap;
  }

  // The conversation's head: the way out, the way to another, the state
  // in words, and New. Shown whenever a conversation is loaded, so the
  // face never reads as a one-way door.
  function paintConvHead() {
    if (!el.convHead) return;
    var row = workSelected();
    el.convHead.hidden = !face.session;
    if (!face.session) return;
    el.convName.textContent = (row && row.title) || sessionLabel(face.session);
    el.convTitle.title = (row && row.title) || face.session;
    if (row) {
      el.convState.hidden = false;
      el.convState.textContent = workStateWord(row.state);
      el.convState.dataset.state = row.state || '';
    } else {
      el.convState.hidden = true;
    }
  }

  function workOpen() {
    return document.body.dataset.work === 'open';
  }

  // Folded is still there: the strip is shown, so the control that brings
  // the rows back never leaves the page the owner is reading them from.
  function workFolded() {
    return document.body.dataset.work === 'collapsed';
  }

  function setWork(open) {
    // A shown list takes the mode the owner last chose. The width the
    // conversation gives up is declared by that mode, so the two can never
    // disagree about how much of the page the list holds.
    var mode = open ? (face.workFolded ? 'collapsed' : 'open') : '';
    document.body.dataset.work = mode;
    if (el.workCol) el.workCol.hidden = !mode;
    if (mode) paintWork();
  }

  // Fold or unfold the list and keep the choice. The head's own control and
  // the strip's are the same press in two places, so either direction is
  // always one click away.
  function foldWork(folded) {
    face.workFolded = folded;
    try {
      window.localStorage.setItem('fleet-work', folded ? 'collapsed' : 'open');
    } catch (error) { }
    setWork(true);
  }

  function toggleWork() {
    // Folded, the title brings the list back before it can become a
    // switcher. Where the list is a column it is already the switcher: the
    // title brings it to the front rather than closing the place the owner
    // is switching from. Where it is a sheet, the title opens and closes it.
    if (workFolded()) { foldWork(false); return; }
    if (window.innerWidth > 1180) { setWork(true); return; }
    setWork(!workOpen());
  }

  // Open a work item in place: resume its log into the conversation
  // beside the list, or — when it is already the open one — just keep
  // the place. One verb, whatever the row's state; the resume refuses
  // only a run in flight in that same session, which is the one case
  // where reading would race the writer.
  function openWork(id, opts) {
    if (!id) return Promise.resolve();
    var options = opts || {};
    var same = face.session === id;
    // The draft swaps with the conversation (D2): the outgoing one keeps
    // its sentence, the incoming one restores its own.
    swapDraft(face.session, id);
    face.focus = { panel: 'sessions', id: id };
    if (!options.keepHash) {
      try {
        window.history.replaceState(null, '', '#/sessions/' + encodeURIComponent(id));
      } catch (error) { }
    }
    face.panel = 'sessions';
    setWork(true);
    if (window.innerWidth <= 1180) setWork(false);
    var driven = face.driving === id && face.running;
    if (same || driven) {
      paintWork();
      paintConvHead();
      return Promise.resolve();
    }
    return post('/api/resume', { session: id }).then(function (reply) {
      if (reply.status !== 200) {
        notice('that conversation would not open',
          (reply.body && reply.body.error) || 'the face said no without a reason');
        return;
      }
      face.sessions = null;
      return Promise.resolve(load(true)).then(function () {
        face.stick = true;
        var toLast = function () {
          var rows = face.stream ? face.stream.children : [];
          var last = rows.length ? rows[rows.length - 1] : null;
          if (last && last.scrollIntoView) last.scrollIntoView({ block: 'end' });
          else if (el.log) el.log.scrollTop = el.log.scrollHeight;
        };
        toLast();
        window.setTimeout(toLast, 80);
        paintWork();
        paintConvHead();
        readSessions(true);
      });
    }).catch(function () { });
  }

  // New is a real conversation: the route writes the log, so the work
  // list holds it before its first message and the next run lands in it.
  function newConversation() {
    return post('/api/session/new', {}).then(function (reply) {
      if (reply.status !== 200 || !reply.body || !reply.body.id) {
        notice('a new conversation was refused',
          (reply.body && reply.body.error) || 'the face said no without a reason');
        return;
      }
      face.sessions = null;
      setWork(true);
      return openWork(reply.body.id);
    }).catch(function () { });
  }

  // A branch's log reads in the drawer, over the live conversation
  // rather than instead of it: the owner keeps watching the run and
  // still reads the child's records.
  function showSessionObject(replay, current) {
    var ex = exchangesIn(replay.cards || []);
    var summary = (replay.model || 'routing\u2026') +
      (replay.kind ? ' \u00b7 ' + replay.kind : '') +
      ' \u00b7 ' + (replay.ended
          ? 'ended: ' + replay.stop_reason +
            (ex ? ' \u00b7 ' + ex + (ex === 1 ? ' exchange' : ' exchanges')
              : replay.turns ? ' \u00b7 ' + replay.turns + ' turns' : '')
        : 'no end record \u2014 the run was interrupted');
    var actions = node('div', 'replay-actions');
    var copy = node('button', 'replay-button');
    copy.type = 'button';
    copy.textContent = 'copy id';
    copy.addEventListener('click', function () {
      if (navigator.clipboard) navigator.clipboard.writeText(replay.id).catch(function () { });
      copy.textContent = 'copied';
      window.setTimeout(function () { copy.textContent = 'copy id'; }, 1200);
    });
    actions.appendChild(copy);
    var open = node('button', 'replay-button replay-resume');
    open.type = 'button';
    open.textContent = replay.id === current
      ? 'this is the open conversation'
      : 'open in the conversation';
    if (replay.id !== current) {
      open.addEventListener('click', function () {
        closeDrawer();
        openWork(replay.id);
      });
    } else {
      open.disabled = true;
    }
    actions.appendChild(open);
    openDrawer('session \u00b7 ' + sessionLabel(replay.id), [
      ['the run', node('p', 'derivation', summary)],
      ['related', relatedFor('session', replay.id)],
      ['the replay', replayBody(replay)],
      ['what you can do', actions]
    ]);
  }

  function replayBody(replay) {
    var wrap = node('div', 'replay-body');
    if (replay.omitted) {
      wrap.appendChild(node('p', 'board-empty',
        replay.omitted + ' earlier card' + (replay.omitted === 1 ? ' is' : 's are') +
        ' left out of this preview. The log holds them.'));
    }
    var lines = node('div', 'replay-lines');
    (replay.cards || []).forEach(function (entry) { lines.appendChild(replayLine(entry)); });
    if (!(replay.cards || []).length) {
      lines.appendChild(node('p', 'board-empty', 'This log carries no messages yet.'));
    }
    wrap.appendChild(lines);
    return wrap;
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
    } else if (entry.type === 'lane') {
      line.appendChild(node('span', 'replay-who', 'lane \u00b7 ' + entry.lane));
      line.appendChild(node('span', 'replay-text', entry.tools + ' tools joined the run'));
    } else {
      line.appendChild(node('span', 'replay-who', entry.type));
    }
    return line;
  }

  // The switcher is the same list, so a row cannot mean two things.
  function openWorkSurface() {
    if (window.location.hash.indexOf('#/sessions') === 0) return;
    window.location.hash = '#/sessions' + (face.session
      ? '/' + encodeURIComponent(face.session) : '');
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
    var instrument = surfaceInstrument('the stream', read.rows_read + ' rows',
      read.day + ' \u00b7 failover gate ' +
      (read.gate && read.gate.informs_failover ? 'on' : 'off'),
      [{
        label: 'run a gate review',
        run: function () {
          ask('use the sys1 lane to grade today\u2019s route rows and report the fit');
        }
      }]);
    el.calibrationBoard.appendChild(surfaceHead('System', 'Calibration',
      'The day\u2019s stream, the judged quality of what served, and the gate that decides whether the failover may learn from it.',
      instrument));
    el.calibrationBoard.appendChild(surfaceStrip([
      [read.rows_read, 'rows, the day\u2019s last ' + read.cap],
      [judged, 'judged'],
      [cells.length, 'fit cells'],
      [fit, 'fit', fit ? 'live' : null],
      [informing, 'informing the failover'],
      [read.gate && read.gate.informs_failover ? 'on' : 'off', 'failover gate']
    ], read.day));
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
          (row.execution ? ' \u00b7 ' + row.execution.label : '') +
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

  // The tool plugin's read. The state the panel renders and the pipeline
  // that produces it are two reads because they fail differently: the
  // files can be read while the tool's server is down, and the plugin
  // says so with the address it tried. A running job polls once a
  // second; a job that just finished reloads the state it rebuilt.
  function readPlugin(force) {
    var now = Date.now();
    if (!force && face.plugin && now - face.pluginAt < 2000) return Promise.resolve();
    face.pluginAt = now;
    return fetch('/api/second-brain/plugin', { headers: { accept: 'application/json' } })
      .then(function (reply) {
        if (reply.status === 401) throw new Error('token');
        return reply.json();
      })
      .then(function (doc) {
        var was = face.pluginWasRunning;
        face.plugin = doc;
        var running = !!(doc.pipeline && doc.pipeline.running);
        face.pluginWasRunning = running;
        if (face.panel === 'second-brain') paintBrain();
        if (running && face.panel === 'second-brain') {
          window.setTimeout(function () {
            if (face.panel === 'second-brain') readPlugin(true);
          }, 1000);
        } else if (was) {
          // The job finished under the panel: the derived state it
          // rebuilt is what the sections below should now show.
          readBrain(true);
        }
      })
      .catch(function () { });
  }

  // The second brain surface, as an atlas: the tool's instrument in the
  // page head, the project landscape as the page itself, the queue as the
  // column beside it, and the method band under both. The old surface put
  // six equal cards in a stack; the eye had nowhere to land. Here the
  // weight is the landscape, and the method telemetry is a ruled band.
  var BRAIN_CLASSES = [
    ['drift', 'ledger drift', 'warn'],
    ['stale', 'stale plans', 'warn'],
    ['handoff', 'live handoffs', 'acc'],
    ['conflicts', 'claims vs evidence', 'down']
  ];

  function brainHead(read) {
    var head = node('header', 'surface-head');
    var main = node('div', 'surface-main');
    main.appendChild(node('p', 'page-eyebrow', 'Knowledge'));
    main.appendChild(node('h1', 'surface-title', 'Second brain'));
    main.appendChild(node('p', 'page-note',
      'The project landscape, read as an atlas: every project\u2019s six-week pulse and the ' +
      'one line its last session left behind, with the decisions that need a person kept beside them.'));
    head.appendChild(main);
    head.appendChild(brainTool(read));
    return head;
  }

  // The tool instrument: the plugin's facts and pipeline, in the page
  // head rather than in a card. A card at the top competed with the
  // landscape; this is the page's own status line. The job picker keeps
  // its choice on the face, because the instrument repaints on every
  // poll and a reset mid-choice would start a different job.
  function brainTool(read) {
    var doc = face.plugin || {};
    var plugin = doc.plugin || {};
    var pipeline = doc.pipeline;
    var running = !!(pipeline && pipeline.running);
    var live = !!plugin.live;
    var wrap = node('div', 'surface-tool');
    var line = node('div', 'surface-tool-line');
    line.appendChild(node('span', live ? 'surface-live' : 'surface-dead'));
    line.appendChild(node('b', null, 'the tool'));
    var built = read.state_at || (pipeline && pipeline.built_at) || '';
    var age = brainAge(built);
    line.appendChild(badge(
      built ? (age >= 2 ? age + ' days behind' : 'fresh') : 'unread',
      built ? (age >= 2 ? 'warn' : 'ok') : null));
    wrap.appendChild(line);
    var meta = node('p', 'surface-tool-meta');
    meta.appendChild(node('span', null, built ? 'state built ' + built : 'the state names no build'));
    if (read.events_since != null) {
      meta.appendChild(node('span', null, ' \u00b7 ' + read.events_since + ' events since'));
    }
    if (!running && pipeline && pipeline.last_ok) {
      meta.appendChild(node('span', null, ' \u00b7 last run ' + pipeline.last_ok));
    }
    wrap.appendChild(meta);

    var controls = node('div', 'surface-tool-act');
    var select = node('select', 'surface-jobs');
    select.setAttribute('aria-label', 'the job to refresh');
    (plugin.jobs || []).forEach(function (job) {
      var option = node('option', null, job.label);
      option.value = job.name;
      select.appendChild(option);
    });
    var known = (plugin.jobs || []).some(function (job) { return job.name === face.pluginJob; });
    select.value = known ? face.pluginJob : ((plugin.jobs || [])[0] || {}).name || '';
    select.addEventListener('change', function () { face.pluginJob = select.value; });
    var button = node('button', 'surface-refresh', running ? 'running\u2026' : 'refresh');
    button.type = 'button';
    button.disabled = !live || running;
    if (!live) button.title = 'the tool is not answering at its address';
    button.addEventListener('click', function () { brainRefresh(select, button); });
    controls.appendChild(select);
    controls.appendChild(button);
    wrap.appendChild(controls);

    // The steps belong to a run in flight or to the failure that ended
    // the last one. A finished run's steps are history: the instrument
    // says when it last ran and keeps the head compact, so the page's
    // title is not pushed down by a list nobody is watching.
    if (pipeline && (running || pipeline.error)) {
      var run = node('div', 'brain-run');
      (pipeline.steps || []).slice(0, 6).forEach(function (step) {
        var row = node('div', 'brain-step');
        row.appendChild(node('span', 'brain-step-label', step.label));
        var tone = step.status === 'failed' ? 'down' : (step.status === 'done' ? 'ok' : null);
        row.appendChild(badge(step.status, tone));
        if (step.ms) row.appendChild(node('span', 'brain-step-ms', Math.round(step.ms) + ' ms'));
        run.appendChild(row);
      });
      if ((pipeline.steps || []).length > 6) {
        run.appendChild(node('p', 'brain-step-more',
          (pipeline.steps.length - 6) + ' more steps'));
      }
      if (pipeline.error) run.appendChild(node('p', 'brain-run-error', pipeline.error));
      var tail = (pipeline.log || []).slice(-3);
      if (tail.length) run.appendChild(node('pre', 'brain-run-log', tail.join('\n')));
      wrap.appendChild(run);
    }
    if (!live) {
      wrap.appendChild(node('p', 'brain-run-error',
        'The tool is not answering at ' + (plugin.address || 'its address') +
        '. The data below is the last state it published.'));
    }
    return wrap;
  }

  function brainRefresh(select, button) {
    button.disabled = true;
    button.textContent = 'starting\u2026';
    fetch('/api/second-brain/act', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ action: 'refresh', job: select.value })
    }).then(function (reply) {
      return reply.json().then(function (doc) { return { ok: reply.ok, doc: doc }; });
    }).then(function (result) {
      if (!result.ok) {
        notice(result.doc.error || 'the tool refused the refresh');
        button.disabled = false;
        button.textContent = 'refresh';
        return;
      }
      readPlugin(true);
    }).catch(function () {
      notice('could not reach fleet');
      button.disabled = false;
      button.textContent = 'refresh';
    });
  }

  function brainStrip(read) {
    var projects = read.projects || [];
    var attention = read.attention || {};
    var counts = attention.counts || {};
    var strip = node('div', 'surface-strip');
    var active = projects.filter(function (row) { return row.lifecycle === 'active'; }).length;
    var dirty = projects.filter(function (row) { return row.dirty > 0; }).length;
    var commits = projects.reduce(function (sum, row) { return sum + (row.commits_30d || 0); }, 0);
    var open = projects.reduce(function (sum, row) { return sum + (row.open_plans || 0); }, 0);
    [
      [projects.length, 'projects'],
      [active, 'active'],
      [dirty, 'dirty'],
      [commits, 'commits / 30d'],
      [open, 'open plans'],
      [counts.conflicts || 0, 'conflicts', 'down'],
      [attention.total || 0, 'need attention']
    ].forEach(function (item) {
      var span = node('span', item[2] === 'down' ? 'surface-strip-down' : null);
      span.appendChild(node('b', null, String(item[0])));
      span.appendChild(document.createTextNode(' ' + item[1]));
      strip.appendChild(span);
    });
    var end = node('span', 'surface-strip-end');
    end.textContent = read.state_at
      ? 'the tool\u2019s view, built ' + read.state_at +
        (read.events_since != null ? ' \u00b7 ' + read.events_since + ' events since' : '')
      : 'the tool names no build';
    strip.appendChild(end);
    return strip;
  }

  function brainAtlas(read) {
    var projects = read.projects || [];
    var card = boardCard(8, 'project atlas', projects.length + ' projects',
      'the six-week pulse, the plans each project keeps open, and the one line its last ' +
      'session left behind. A row opens the project\u2019s own page.');
    card.classList.add('atlas-card');
    var maxWeek = 1;
    projects.forEach(function (row) {
      (row.weeks || []).forEach(function (value) { maxWeek = Math.max(maxWeek, value); });
    });
    var columns = node('div', 'atlas-cols');
    ['project', 'six weeks', '30d', 'open', 'left on the table'].forEach(function (label) {
      columns.appendChild(node('span', null, label));
    });
    card.appendChild(columns);
    var shown = face.brainShowAll ? projects : projects.slice(0, 16);
    shown.forEach(function (row) { card.appendChild(atlasRow(row, maxWeek)); });
    var reports = projects.reduce(function (sum, row) {
      return sum + (row.reports || []).reduce(function (a, b) { return a + b; }, 0);
    }, 0);
    var filing = projects.filter(function (row) {
      return (row.reports || []).some(function (value) { return value > 0; });
    }).length;
    var foot = node('div', 'atlas-foot');
    foot.appendChild(node('span', null,
      reports + ' reports filed in the same six weeks by ' + filing + ' of ' +
      projects.length + ' projects'));
    if (!face.brainShowAll && projects.length > shown.length) {
      var more = node('button', 'atlas-more', 'show all ' + projects.length);
      more.type = 'button';
      more.addEventListener('click', function () { face.brainShowAll = true; paintBrain(); });
      foot.appendChild(more);
    } else if (face.brainShowAll && projects.length > 16) {
      var less = node('button', 'atlas-more', 'show the busiest 16');
      less.type = 'button';
      less.addEventListener('click', function () { face.brainShowAll = false; paintBrain(); });
      foot.appendChild(less);
    }
    card.appendChild(foot);
    return card;
  }

  function atlasRow(project, maxWeek) {
    var row = node('div', 'atlas-row');
    row.tabIndex = 0;
    row.setAttribute('role', 'button');
    row.title = 'open ' + project.name;
    var name = node('div', 'atlas-name');
    name.appendChild(node('b', null, project.name));
    if (project.lifecycle && project.lifecycle !== 'active') {
      name.appendChild(node('span', 'atlas-life',
        project.lifecycle === 'exploration' ? 'exploring' : project.lifecycle));
    }
    var marks = node('span', 'atlas-marks');
    if (project.conflicts) {
      marks.appendChild(node('span', 'atlas-mark down')).title =
        project.conflicts + ' open conflicts';
    }
    if (project.stale_plans) {
      marks.appendChild(node('span', 'atlas-mark warn')).title =
        project.stale_plans + ' stale plans';
    }
    if (project.handoff && project.handoff.score >= 0.6) {
      marks.appendChild(node('span', 'atlas-mark acc')).title = 'a live handoff';
    }
    name.appendChild(marks);
    row.appendChild(name);
    var bars = node('span', 'atlas-bars');
    (project.weeks || []).forEach(function (value) {
      var bar = node('span', value ? '' : 'zero');
      if (value) bar.style.height = Math.max(2, Math.round(24 * Math.sqrt(value / maxWeek))) + 'px';
      bars.appendChild(bar);
    });
    row.appendChild(bars);
    row.appendChild(node('span', 'atlas-num', String(project.commits_30d || 0)));
    row.appendChild(node('span', 'atlas-num', (project.open_plans || 0) + ' open'));
    var next = project.handoff && project.handoff.score >= 0.5
      ? brainClean((project.handoff.next || [])[0] || '')
      : '';
    row.appendChild(node('div', next ? 'atlas-next' : 'atlas-next none',
      next || 'nothing left on the table'));
    var open = function () { brainProjectDrawer(project); };
    row.addEventListener('click', open);
    row.addEventListener('keydown', function (event) {
      if (event.key === 'Enter' || event.key === ' ') {
        event.preventDefault();
        open();
      }
    });
    return row;
  }

  function brainAttention(read) {
    var attention = read.attention || {};
    var counts = attention.counts || {};
    var card = boardCard(4, 'needs attention', null, null);
    card.classList.add('attn-card');
    var total = node('div', 'attn-total');
    total.appendChild(node('b', null, String(attention.total || 0)));
    total.appendChild(node('span', null,
      'decisions waiting across ' + ((read.projects || []).length) + ' projects'));
    card.appendChild(total);

    var selected = face.brainClass || '';
    BRAIN_CLASSES.forEach(function (entry) {
      var count = counts[entry[0]] || 0;
      var row = node('button', 'attn-class');
      row.type = 'button';
      row.setAttribute('aria-pressed', selected === entry[0] ? 'true' : 'false');
      row.appendChild(node('span', null, entry[1]));
      var bar = node('span', 'attn-bar');
      var fill = node('i');
      fill.dataset.tone = entry[2];
      fill.style.width = (attention.total
        ? Math.round(100 * count / attention.total) : 0) + '%';
      bar.appendChild(fill);
      row.appendChild(bar);
      row.appendChild(node('em', null, String(count)));
      row.addEventListener('click', function () {
        face.brainClass = face.brainClass === entry[0] ? '' : entry[0];
        paintBrain();
      });
      card.appendChild(row);
    });

    var list = node('div', 'attn-list');
    var rows = [];
    if (selected) {
      var klass = BRAIN_CLASSES.filter(function (entry) { return entry[0] === selected; })[0];
      rows = (attention[selected] || []).slice(0, 12).map(function (row) {
        return { row: row, tone: klass[2], klass: selected };
      });
    } else {
      BRAIN_CLASSES.forEach(function (entry) {
        (attention[entry[0]] || []).slice(0, entry[0] === 'conflicts' ? 3 : 2)
          .forEach(function (row) {
            rows.push({ row: row, tone: entry[2], klass: entry[0] });
          });
      });
    }
    rows.forEach(function (entry) {
      list.appendChild(attentionRow(entry.row, entry.tone, entry.klass));
    });
    if (!rows.length) list.appendChild(node('p', 'board-empty', 'Nothing in this class.'));
    card.appendChild(list);
    return card;
  }

  function attentionRow(row, tone, klass) {
    var line = node('div', 'attn-row');
    line.tabIndex = 0;
    line.setAttribute('role', 'button');
    line.dataset.tone = tone;
    line.appendChild(node('span', 'attn-glyph',
      tone === 'down' ? '\u2715' : (tone === 'acc' ? '\u25b6' : '\u26a0')));
    var body = node('div', 'attn-body');
    body.appendChild(node('b', null, row.project));
    body.appendChild(node('p', null, brainClean(row.what)));
    line.appendChild(body);
    line.appendChild(node('span', 'attn-ev', brainEvidence(row, klass)));
    var open = function () {
      if (klass === 'conflicts') brainConflictDrawer(row);
      else brainProjectFromName(row.project);
    };
    line.addEventListener('click', open);
    line.addEventListener('keydown', function (event) {
      if (event.key === 'Enter' || event.key === ' ') {
        event.preventDefault();
        open();
      }
    });
    return line;
  }

  function brainEvidence(row, klass) {
    if (klass === 'conflicts') {
      var age = brainRel(String(row.when || '').slice(0, 10));
      return row.evidence + (age ? ' \u00b7 ' + age : '');
    }
    if (!row.when) return row.evidence;
    var relative = brainRel(String(row.when).slice(0, 10));
    return relative ? row.evidence.replace(row.when, relative) : row.evidence;
  }

  function brainRibbon(read) {
    var days = (read.commits || []).slice();
    var first = -1;
    days.forEach(function (row, index) { if (first < 0 && row.count > 0) first = index; });
    if (first > 0) days = days.slice(first);
    var max = days.reduce(function (value, row) { return Math.max(value, row.count); }, 0);
    var card = boardCard(12, 'recent commits',
      days.length ? (days[0].day + ' \u2014 ' + days[days.length - 1].day) : null,
      'one bar per day. The tool keeps its newest 400 timeline rows, so the left edge is ' +
      'the cap, not a quiet month.');
    card.classList.add('brain-ribbon');
    var strip = node('div', 'commit-bars');
    days.forEach(function (row) {
      var bar = node('span', row.count ? (row.count === max ? 'max' : '') : 'zero');
      if (row.count) {
        bar.style.height = Math.max(1, Math.round(52 * Math.sqrt(row.count / max))) + 'px';
      }
      bar.title = row.day + ': ' + row.count + ' commits';
      strip.appendChild(bar);
    });
    card.appendChild(strip);
    var axis = node('div', 'commit-axis');
    [0, Math.floor(days.length / 3), Math.floor((2 * days.length) / 3), days.length - 1]
      .forEach(function (index) {
        if (index >= 0 && days[index]) {
          axis.appendChild(node('span', null, days[index].day.slice(5)));
        }
      });
    card.appendChild(axis);
    return card;
  }

  // The evidence the retired tower console held that the atlas did not:
  // the report feed, newest first, and each harness's claim precision.
  // The scan, the ranked projects, and the activity chart are covered by
  // the atlas itself, so they did not move.
  function brainEvidence(read) {
    var reports = read.reports || {};
    var feed = reports.feed || [];
    var trust = reports.trust || [];
    if (!feed.length && !trust.length) return null;
    var card = boardCard(12, 'the report feed', feed.length + ' filed',
      'every report filed to the tower\u2019s ledger, newest first \u2014 the evidence the atlas '
      + 'reads its conflicts and its coverage from.');
    card.classList.add('lib-card');
    var list = node('div', 'rail-list');
    feed.slice(0, 12).forEach(function (row) {
      var line = node('div', 'rail-row');
      var body = node('div');
      body.appendChild(node('b', null, row.project + ' \u00b7 ' + row.kind));
      body.appendChild(node('p', null, row.summary));
      line.appendChild(body);
      line.appendChild(node('span', 'rail-em', String(row.ts || '').slice(5, 16)));
      list.appendChild(line);
    });
    card.appendChild(list);
    if (feed.length > 12) {
      card.appendChild(node('p', 'rail-more',
        (feed.length - 12) + ' more filings in the ledger'));
    }
    trust.forEach(function (row) {
      card.appendChild(node('p', 'rail-note',
        row.harness + ' \u00b7 ' + row.evidenced + ' of ' + row.claimed +
        ' claims carried evidence \u00b7 precision ' + Math.round(row.precision * 100) + '%'));
    });
    return card;
  }

  function brainMethod(read) {
    var band = node('section', 'brain-method');
    band.dataset.span = '12';

    var gates = node('div');
    gates.appendChild(node('h3', null, 'Gates \u00b7 earned autonomy'));
    var board = (read.gates && read.gates.board) || [];
    board.forEach(function (row) {
      var line = node('div', 'brain-gate');
      line.appendChild(node('span', 'brain-gate-name', row.gate));
      var track = node('span', 'brain-gate-track');
      var fill = node('span', 'brain-gate-fill');
      // The state's phrase is the authority: `3/20` is the graded count
      // the gate needs before it may block. The row's numeric field is a
      // convenience the tool does not always carry.
      var graded = String(row.graded_progress || '').split('/');
      var filled = parseInt(graded[0], 10);
      if (!isFinite(filled)) filled = row.graded_rows || 0;
      fill.style.width = Math.round(Math.min(1, filled / 20) * 100) + '%';
      track.appendChild(fill);
      line.appendChild(track);
      line.appendChild(node('span', 'brain-gate-meta',
        row.graded_progress +
        (row.override_rate ? ' \u00b7 ' + Math.round(row.override_rate * 100) + '%' : '')));
      gates.appendChild(line);
    });
    if (!board.length) gates.appendChild(node('p', 'brain-method-empty', 'the gates state has not landed'));
    band.appendChild(gates);

    var loops = node('div');
    loops.appendChild(node('h3', null, 'Loops & harness'));
    var rows = (read.loops && read.loops.loops) || [];
    var iterations = rows.reduce(function (sum, row) { return sum + (row.iterations || 0); }, 0);
    var pending = rows.reduce(function (sum, row) { return sum + (row.pending_review || 0); }, 0);
    var stats = node('p', 'brain-loops-stats');
    [
      [rows.length, 'loops'],
      [iterations, 'iterations'],
      [pending, 'pending review']
    ].forEach(function (item) {
      var chunk = node('span');
      chunk.appendChild(node('b', null, String(item[0])));
      chunk.appendChild(document.createTextNode(' ' + item[1]));
      stats.appendChild(chunk);
    });
    loops.appendChild(stats);
    rows.slice()
      .sort(function (left, right) {
        return (right.pending_review || 0) - (left.pending_review || 0) ||
          (right.iterations || 0) - (left.iterations || 0);
      })
      .slice(0, 4)
      .forEach(function (row) {
        var line = node('div', 'brain-loop');
        line.appendChild(node('span', null, row.name));
        line.appendChild(node('span', null,
          row.iterations + ' it' + (row.pending_review ? ' \u00b7 ' + row.pending_review + ' pending' : '')));
        loops.appendChild(line);
      });
    var harness = (read.harness && read.harness.loops) || [];
    if (harness.length) {
      loops.appendChild(node('p', 'brain-harness',
        'harness \u2014 ' + harness[0].converged + ' of ' + harness[0].runs +
        ' ' + harness[0].name + ' runs converged'));
    }
    band.appendChild(loops);

    var maturity = node('div');
    maturity.appendChild(node('h3', null, 'Maturity \u00b7 14 weeks'));
    if (read.maturity) {
      var weeks = (read.maturity.weeks || []).slice(-14);
      var series = (read.maturity.series || []).map(function (row) {
        return { name: row.name, points: (row.points || []).slice(-14) };
      });
      maturity.appendChild(weekGrid(series, weeks));
      var milestones = (read.maturity.milestones || []).slice(-1);
      if (milestones.length) {
        maturity.appendChild(node('p', 'brain-milestone',
          milestones[0].when + ' \u00b7 ' + milestones[0].what));
      }
    } else {
      maturity.appendChild(node('p', 'brain-method-empty', 'the maturity state has not landed'));
    }
    band.appendChild(maturity);
    return band;
  }

  function brainProjectDrawer(project) {
    var sections = [];
    sections.push(['the six weeks', node('p', 'derivation',
      project.total + ' commits in six weeks \u00b7 ' + project.commits_30d + ' in 30 days \u00b7 ' +
      (project.dirty ? project.dirty + ' dirty files' : 'clean worktree'))]);
    sections.push(['the plans', node('p', 'derivation',
      project.open_plans
        ? project.open_plans + ' open \u00b7 ' + project.stale_plans + ' untouched for three weeks'
        : 'the derived state names no open plan')]);
    if (project.handoff) {
      var handoff = node('div');
      handoff.appendChild(node('p', 'derivation',
        project.handoff.date + ' \u00b7 ' + Math.round((project.handoff.score || 0) * 100) +
        '% next-step score'));
      (project.handoff.next || []).forEach(function (line) {
        handoff.appendChild(node('p', 'derivation', brainClean(line)));
      });
      sections.push(['the handoff', handoff]);
    }
    if (project.conflicts) {
      sections.push(['claims vs evidence', node('p', 'derivation',
        project.conflicts + ' open conflicts \u2014 the tool\u2019s conflicts board carries the claims')]);
    }
    openDrawer('project \u00b7 ' + project.name, sections);
  }

  function brainConflictDrawer(row) {
    openDrawer('claim \u00b7 ' + row.project, [
      ['the claim', node('p', 'derivation', brainClean(row.what))],
      ['the evidence', node('p', 'derivation', brainClean(row.detail) || 'the tool names no reality')],
      ['filed', node('p', 'derivation', row.when || 'the tool names no time')]
    ]);
  }

  function brainProjectFromName(name) {
    var projects = (face.brain && face.brain.projects) || [];
    var project = projects.filter(function (row) { return row.name === name; })[0];
    if (project) {
      brainProjectDrawer(project);
      return;
    }
    openDrawer('project \u00b7 ' + name, [['the state', node('p', 'derivation',
      'the atlas names no row for this project')]]);
  }

  function brainClean(text) {
    return String(text == null ? '' : text)
      .replace(/\*\*/g, '')
      .replace(/~~/g, '')
      .replace(/`/g, '')
      .replace(/\s+/g, ' ')
      .trim();
  }

  function brainAge(stamp) {
    var date = String(stamp || '').slice(0, 10);
    if (!date) return null;
    var then = Date.parse(date + 'T00:00:00Z');
    if (!isFinite(then)) return null;
    var now = new Date();
    var today = Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), now.getUTCDate());
    return Math.max(0, Math.round((today - then) / 86400000));
  }

  function brainRel(date) {
    if (!date) return '';
    var days = brainAge(date);
    if (days == null) return '';
    if (days === 0) return 'today';
    if (days < 30) return days + 'd';
    if (days < 60) return Math.round(days / 7) + 'w';
    return Math.round(days / 30) + 'mo';
  }

  function paintBrain() {
    var read = face.brain;
    el.brainBoard.textContent = '';
    if (!read) {
      el.brainBoard.appendChild(brainHead({}));
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
    el.brainBoard.appendChild(brainHead(read));
    el.brainBoard.appendChild(brainStrip(read));
    if (!read.projects) {
      var empty = boardCard(12, 'the atlas', null, null);
      empty.appendChild(node('p', 'board-empty',
        'The tower data has not landed, so the atlas has no projects to place. The reason ' +
        'is beside the sections that did land.'));
      el.brainBoard.appendChild(empty);
      return;
    }
    el.brainBoard.appendChild(brainAtlas(read));
    el.brainBoard.appendChild(brainAttention(read));
    if ((read.commits || []).length) el.brainBoard.appendChild(brainRibbon(read));
    var evidence = brainEvidence(read);
    if (evidence) el.brainBoard.appendChild(evidence);
    el.brainBoard.appendChild(brainMethod(read));
  }

  // ------------------------------------------------- the workspace-history panel

  // The substrate's tree, as the face's own knowledge surface: the
  // History pane draws the DAG (main's trunk, each child branch at its
  // fork point, the merge edges back), the Work pane carries each
  // child's lane with its session and state, the Files pane browses a
  // branch, and Storage carries the Store's figures. The read is the
  // substrate's own, so a machine that has not provisioned — or a
  // folder no delegation has registered — is named where the tree would
  // be, never drawn as an empty one.
  function readWorkspaceHistory(force) {
    var now = Date.now();
    if (!force && face.history && now - face.historyAt < 5000) return Promise.resolve();
    face.historyAt = now;
    // The refresh rides the version the last read named: a substrate
    // that still holds it current answers unchanged in one flag and the
    // view stands — no repaint, no re-read of the files pane.
    var since = (face.history && face.history.version)
      ? '?since=' + encodeURIComponent(face.history.version) : '';
    return fetch('/api/workspace-history' + since, { headers: { accept: 'application/json' } })
      .then(function (reply) {
        if (reply.status === 401) throw new Error('token');
        return reply.json();
      })
      .then(function (doc) {
        if (doc.unchanged) return;
        face.history = doc;
        // The Files pane's branch follows the tree: the first branch
        // that is not main, or main itself, so a fresh read lands
        // somewhere real rather than on an empty selector.
        var branches = doc.branches || [];
        if (!face.historyBranch || !branches.some(function (b) { return b.name === face.historyBranch; })) {
          var child = branches.filter(function (b) { return b.name !== 'main'; })[0];
          face.historyBranch = (child || branches[0] || {}).name || 'main';
        }
        if (face.panel === 'workspace-history') {
          paintWorkspaceHistory();
          readWorkspaceFiles(face.historyBranch, face.historyPath || '', true);
        }
      })
      .catch(function () { });
  }

  // The live feed's page: one read per refresh, riding `since` so a
  // poll answers only what moved. Traces off is a named absence.
  function readWorkspaceCalls(force) {
    var now = Date.now();
    if (!force && face.calls && now - face.callsAt < 5000) return;
    face.callsAt = now;
    var since = (face.callsLastStart)
      ? '?since=' + encodeURIComponent(face.callsLastStart) : '';
    return fetch('/api/workspace-history/calls' + since, { headers: { accept: 'application/json' } })
      .then(function (reply) {
        if (reply.status === 401) throw new Error('token');
        return reply.json();
      })
      .then(function (doc) {
        var calls = doc.calls || [];
        if (calls.length) {
          face.callsLastStart = calls[calls.length - 1].start || face.callsLastStart;
          face.calls = face.calls && !force
            ? { type: 'workspace-calls', calls: (face.calls.calls || []).concat(calls).slice(-200), more: doc.more, note: doc.note, refused: doc.refused }
            : doc;
        } else if (!face.calls || force) {
          face.calls = doc;
        }
        if (face.panel === 'workspace-history') paintWorkspaceHistory();
      })
      .catch(function () { });
  }

  // The Files pane's page. One read per (branch, path, page), cached on
  // the face so a repaint does not re-ask for the listing the owner is
  // reading.
  function readWorkspaceFiles(branch, path, force) {
    if (!branch) return Promise.resolve();
    var key = branch + '\u0000' + path + '\u0000' + (face.historyPage || 0);
    var now = Date.now();
    if (!force && face.historyFiles && face.historyFilesKey === key &&
        now - face.historyFilesAt < 5000) {
      return Promise.resolve();
    }
    face.historyFilesAt = now;
    var asked = '/api/workspace-history/files?branch=' + encodeURIComponent(branch) +
      '&path=' + encodeURIComponent(path || '/') +
      '&page=' + (face.historyPage || 0);
    return fetch(asked, { headers: { accept: 'application/json' } })
      .then(function (reply) {
        if (reply.status === 401) throw new Error('token');
        return reply.json();
      })
      .then(function (doc) {
        face.historyFiles = doc;
        face.historyFilesKey = key;
        if (face.panel === 'workspace-history') paintWorkspaceHistory();
      })
      .catch(function () { });
  }

  function paintWorkspaceHistory() {
    var read = face.history;
    el.workspaceHistoryBoard.textContent = '';
    if (!read) {
      el.workspaceHistoryBoard.appendChild(surfaceHead('Knowledge', 'Workspace history',
        'One Store, every branch a delegated child worked: the tree as it stands, and the ' +
        'figures behind it.', null));
      var waiting = boardCard(12, 'reading', null, null);
      waiting.appendChild(node('p', 'board-empty',
        'The substrate read has not landed yet. It reads the LayerFS Store through the ' +
        'same composed environment a delegation spawns with.'));
      el.workspaceHistoryBoard.appendChild(waiting);
      return;
    }
    var branches = read.branches || [];
    var layers = read.layers || [];
    var commits = (read.commits || []).length;
    var storage = read.storage || null;
    var instrument = surfaceInstrument('the store',
      read.registered === false ? 'no project' : (read.project || 'the project'),
      read.registered === false
        ? 'the launch folder has no LayerFS project yet'
        : (branches.length + (branches.length === 1 ? ' branch' : ' branches') +
           ' \u00b7 ' + commits + ' commits \u00b7 ' + layers.length +
           (layers.length === 1 ? ' layer' : ' layers')),
      [{
        label: 'refresh',
        run: function () {
          readWorkspaceHistory(true).then(paintWorkspaceHistory);
          readWorkspaceCalls(true);
          readWorkspaceFiles(face.historyBranch, face.historyPath || '', true);
        }
      }]);
    el.workspaceHistoryBoard.appendChild(surfaceHead('Knowledge', 'Workspace history',
      'One Store, every branch a delegated child worked: the tree as it stands, and the ' +
      'figures behind it.', instrument));
    el.workspaceHistoryBoard.appendChild(surfaceStrip([
      [branches.length, 'branches'],
      [commits, 'commits'],
      [layers.length, 'layers'],
      [storage ? Math.round(storage.stored / 1048576) : 0, 'MB stored'],
      [storage ? Math.round(storage.dedupe * 100) / 100 : 0, 'dedupe'],
      [(read.refused || []).length, 'reads refused', (read.refused || []).length ? 'down' : null]
    ], read.project ? 'project ' + read.project : 'no project registered'));

    if (read.registered === false) {
      var none = boardCard(12, 'the tree', null, null);
      none.appendChild(node('p', 'board-empty', read.note ||
        'This folder has no LayerFS project. The first delegation launched here registers ' +
        'one, and the tree it worked appears on this surface.'));
      none.appendChild(node('p', 'board-empty',
        'The substrate is the isolation a delegated run works in: the child\u2019s writes ' +
        'land on its own branch, and nothing reaches the working tree until a merge is ' +
        'published. A folder that has never delegated has no history to show.'));
      el.workspaceHistoryBoard.appendChild(none);
      return;
    }
    if ((read.refused || []).length) {
      el.workspaceHistoryBoard.appendChild(refusedCard(read.refused));
    }
    el.workspaceHistoryBoard.appendChild(historyDag(read));
    el.workspaceHistoryBoard.appendChild(historyWork(read));
    el.workspaceHistoryBoard.appendChild(face.calls ? historyCalls(face.calls) : historyCalls({ calls: [] }));
    el.workspaceHistoryBoard.appendChild(historyFiles(read));
    el.workspaceHistoryBoard.appendChild(historyStorage(read));
  }

  // The tree: the substrate's history as the reference draws it —
  // main's spine carries a BAND for every Layer, each branch with work
  // is a lane sprouting from a fork node at its actual base, every
  // Commit is a leaf on its lane, a publish grafts a leaf back to the
  // band it made with a dashed stem and a ring, and a branch nobody
  // worked is one idle chip — never eighty empty columns. Time is the
  // order of events, not the wall clock; sideways by default, upright
  // on the toggle; zoom 25\u2013200% + fit; pan by drag or scroll; a
  // minimap with the visible window; a hover card per node; clicking a
  // node points the Files pane at that ref.
  function historyLayout(read) {
    var layers = (read.layers || []).slice().reverse(); // the answer is newest first
    var layerIndex = {};
    layers.forEach(function (layer, index) { layerIndex[layer.id] = index; });
    var commits = (read.commits || []).slice().sort(function (a, b) {
      return (a.time || 0) - (b.time || 0);
    });
    var ordinal = {};
    commits.forEach(function (commit, index) { ordinal[commit.id] = index + 1; });
    // Commits group under the Layer they forked from; a commit whose
    // base is not a Layer id rides at the end, named in its hover.
    var byBase = {};
    commits.forEach(function (commit) {
      var base = (commit.base && layerIndex[commit.base] !== undefined) ? commit.base : null;
      (byBase[base] = byBase[base] || []).push(commit);
    });
    // Slots: the layer chain is the backbone; each layer's forked
    // commits sit between it and the next layer, in time order.
    var slot = {}, next = 0;
    layers.forEach(function (layer) {
      slot[layer.id] = next++;
      (byBase[layer.id] || []).forEach(function (commit) { slot[commit.id] = next++; });
    });
    commits.forEach(function (commit) {
      if (slot[commit.id] === undefined) slot[commit.id] = next++;
    });
    var totalSlots = Math.max(1, next);
    // The branches with work, in fork order, one lane each — the free
    // lane nearest the spine, alternating sides.
    var worked = (read.branches || []).filter(function (branch) {
      return branch.name !== 'main' && branch.head;
    });
    worked.sort(function (a, b) {
      var ai = (a.base && layerIndex[a.base] !== undefined) ? layerIndex[a.base] : 999;
      var bi = (b.base && layerIndex[b.base] !== undefined) ? layerIndex[b.base] : 999;
      return ai - bi || (a.name < b.name ? -1 : 1);
    });
    var lane = {}, up = 0, down = 0;
    worked.forEach(function (branch, index) {
      lane[branch.name] = (index % 2 === 0) ? (++down) : (-(++up));
    });
    // The grafts: every published Layer names the Commit it was made
    // from — the dashed stem back to the band.
    var grafts = layers.filter(function (layer) {
      return layer.source_commit && slot[layer.source_commit] !== undefined;
    });
    var idle = (read.branches || []).filter(function (branch) {
      return branch.name !== 'main' && !branch.head;
    });
    return {
      layers: layers, layerIndex: layerIndex, commits: commits, ordinal: ordinal,
      slot: slot, totalSlots: totalSlots, worked: worked, lane: lane,
      grafts: grafts, idle: idle,
      totalNodes: layers.length + commits.length
    };
  }

  // One branch's lane colour: the categorical palette, by lane order —
  // the semantic tones (accent/ok/down/warn) stay semantic and are
  // never spent on lanes.
  function historyLaneColor(index) {
    return 'var(--lane-' + (index % 6) + ')';
  }

  function historyStateWord(state) {
    return { merged: 'merged to main', open: 'open \u00b7 not merged', idle: 'forked, no work' }[state] || state;
  }

  function historyDag(read) {
    var card = boardCard(12, 'the tree', null,
      'main is the trunk: a band for every Layer, a lane for every branch that has work, ' +
      'a leaf for every commit, a dashed stem where a publish grafted work back. Click a node ' +
      'to read Files there; drag or scroll to pan; a branch nobody worked is one idle chip.');
    var layout = historyLayout(read);
    if (!layout.totalNodes) {
      card.appendChild(node('p', 'board-empty',
        'The Store holds its first layer and no commits yet.'));
      return card;
    }
    var wide = cardWide(el.workspaceHistoryBoard, 12);
    var fit = Math.max(10, Math.min(64, (wide - 200) / layout.totalSlots));
    var zoom = face.historyZoom || 1;
    var px = Math.max(3, Math.round(fit * zoom));
    var mode = face.historyUpright ? 'upright' : 'sideways';
    var plot = historyPlot(read, layout, px, mode);
    // The controls: zoom out / fit / zoom in, and the orientation
    // toggle. The zoom is a factor over fit, held across repaints.
    var controls = node('div', 'dag-controls');
    var note = node('span', 'dag-zoom-note',
      Math.round(zoom * 100) + '% \u00b7 ' + (mode === 'upright' ? 'upright' : 'sideways'));
    function repaint() { paintWorkspaceHistory(); }
    var zoomOut = node('button', 'dag-zoom', '\u2212');
    zoomOut.type = 'button';
    zoomOut.title = 'zoom out (25% floor)';
    zoomOut.addEventListener('click', function () {
      face.historyZoom = Math.max(0.25, (face.historyZoom || 1) * 0.75);
      repaint();
    });
    var fitBtn = node('button', 'dag-zoom', 'fit');
    fitBtn.type = 'button';
    fitBtn.title = 'fit the whole run to the card';
    fitBtn.addEventListener('click', function () {
      face.historyZoom = 1;
      repaint();
    });
    var zoomIn = node('button', 'dag-zoom', '+');
    zoomIn.type = 'button';
    zoomIn.title = 'zoom in (200% ceiling)';
    zoomIn.addEventListener('click', function () {
      face.historyZoom = Math.min(2, (face.historyZoom || 1) * 1.3334);
      repaint();
    });
    var orient = node('button', 'dag-zoom', mode === 'upright' ? 'sideways' : 'upright');
    orient.type = 'button';
    orient.title = 'time runs right (sideways) or up (upright)';
    orient.addEventListener('click', function () {
      face.historyUpright = !face.historyUpright;
      repaint();
    });
    [zoomOut, fitBtn, zoomIn, orient, note].forEach(function (piece) {
      controls.appendChild(piece);
    });
    // The plot in its scroll: pan by wheel or drag; the minimap jumps.
    var scroll = node('div', 'dag-scroll');
    scroll.appendChild(plot.svg);
    var dragging = null;
    scroll.addEventListener('pointerdown', function (event) {
      if (event.target.closest('.dag-node')) return;
      dragging = { x: event.clientX, left: scroll.scrollLeft };
      scroll.classList.add('is-dragging');
    });
    scroll.addEventListener('pointermove', function (event) {
      if (!dragging) return;
      scroll.scrollLeft = dragging.left - (event.clientX - dragging.x);
    });
    ['pointerup', 'pointerleave'].forEach(function (name) {
      scroll.addEventListener(name, function () {
        dragging = null;
        scroll.classList.remove('is-dragging');
      });
    });
    var minimap = historyMinimap(layout);
    scroll.addEventListener('scroll', function () {
      historyMinimapWindow(scroll, minimap);
    });
    var wrap = node('div', 'chart dag-wrap');
    wrap.appendChild(controls);
    wrap.appendChild(scroll);
    wrap.appendChild(minimap);
    // The hover card, one per plot, filled by the nodes.
    var hover = node('div', 'dag-hover');
    hover.hidden = true;
    wrap.appendChild(hover);
    plot.bindHover(scroll, wrap, hover);
    // The idle branches: one chip, never one column each.
    if (layout.idle.length) {
      var idle = node('div', 'dag-idle');
      var chip = node('button', 'chip',
        layout.idle.length + ' idle \u00b7 forked, no work');
      chip.type = 'button';
      chip.title = 'branches that were forked and never worked';
      idle.appendChild(chip);
      if (face.historyIdleOpen) {
        var names = node('p', 'board-note', layout.idle.map(function (branch) {
          return branch.name;
        }).join(' \u00b7 '));
        idle.appendChild(names);
      }
      chip.addEventListener('click', function () {
        face.historyIdleOpen = !face.historyIdleOpen;
        repaint();
      });
      wrap.appendChild(idle);
    }
    if (read.truncated) {
      wrap.appendChild(node('p', 'dag-truncated', 'showing ' + read.truncated));
    }
    card.appendChild(wrap);
    // Restore the pan the owner had, then show the newest end.
    if (face.historyScroll) scroll.scrollLeft = face.historyScroll;
    else scroll.scrollLeft = scroll.scrollWidth;
    scroll.addEventListener('scroll', function () {
      face.historyScroll = scroll.scrollLeft;
    });
    return card;
  }

  // The plot: one svg whose viewBox is the CONTENT's size — the card
  // scrolls, nothing clips. Builds nodes and edges from the layout;
  // `mode` picks the orientation (time right, or time up).
  function historyPlot(read, layout, px, mode) {
    var ROW = 34, COL = 44, PAD = 26, TAG = 150;
    var rowsUp = Math.max(0, -Math.min.apply(null, [0].concat(layout.worked.map(function (b) {
      return layout.lane[b.name];
    }))));
    var rowsDown = Math.max(0, Math.max.apply(null, [0].concat(layout.worked.map(function (b) {
      return layout.lane[b.name];
    }))));
    var sideways = mode !== 'upright';
    var w, h, spineAt;
    if (sideways) {
      w = PAD + layout.totalSlots * px + TAG;
      h = PAD * 2 + (rowsUp + rowsDown + 1) * ROW;
      spineAt = PAD + rowsUp * ROW + ROW / 2;
    } else {
      w = PAD * 2 + (rowsUp + rowsDown + 1) * COL + 120;
      h = PAD + layout.totalSlots * px + 56;
      spineAt = PAD + rowsUp * COL + COL / 2;
    }
    function pos(t, lane) {
      if (sideways) return { x: PAD + t * px, y: spineAt + lane * ROW };
      return { x: spineAt + lane * COL, y: h - 56 - t * px };
    }
    var plot = svg('svg', {
      width: Math.max(w, 320), height: h,
      viewBox: '0 0 ' + Math.max(w, 320) + ' ' + h,
      role: 'img',
      'aria-label': 'the workspace history tree'
    });
    // The spine: main's own line, a little thicker than the rest.
    var first = pos(0, 0), last = pos(layout.totalSlots - 1, 0);
    if (sideways) {
      plot.appendChild(svg('line', {
        x1: first.x - 14, y1: first.y, x2: last.x + 14, y2: last.y, class: 'dag-spine'
      }));
      plot.appendChild(label(first.x - 18, first.y + 3, 'dag-lane-name', 'main', 'end'));
    } else {
      plot.appendChild(svg('line', {
        x1: first.x, y1: first.y + 14, x2: last.x, y2: last.y - 14, class: 'dag-spine'
      }));
      plot.appendChild(label(first.x + 8, first.y + 20, 'dag-lane-name', 'main'));
    }
    // The bands: one per Layer, coloured by the branch that published
    // it (the import's own layer stays neutral).
    var laneColor = {};
    layout.worked.forEach(function (branch, index) {
      laneColor[branch.name] = historyLaneColor(index);
      var merged = 'merge-' + branch.name;
      laneColor[merged] = historyLaneColor(index);
    });
    var nodes = [];
    layout.layers.forEach(function (layer, index) {
      var at = pos(layout.slot[layer.id], 0);
      var band = svg('rect', {
        x: sideways ? at.x - 5 : at.x - 9,
        y: sideways ? at.y - 9 : at.y - 5,
        width: sideways ? 10 : 18,
        height: sideways ? 18 : 10,
        rx: 2, class: 'dag-band',
        fill: laneColor[layer.source_branch] || 'var(--dsw-alias-surface-raised)'
      });
      bindNode(band, nodes, {
        kind: 'layer', id: layer.id, at: at,
        title: 'Layer ' + layer.id.slice(0, 12) +
          (layer.source_branch ? ' \u00b7 published from ' + layer.source_branch.replace(/^merge-/, '') : ' \u00b7 the import\u2019s own layer'),
        lines: [
          'layer ' + layer.id.slice(0, 12) + (index === 0 ? ' \u00b7 the root' : ''),
          bytes(layer.logical) + ' logical' +
            (layer.source_commit ? ' \u00b7 from commit ' + layer.source_commit.slice(0, 12) : '')
        ],
        ref: layer.id, label: 'layer ' + layer.id.slice(0, 12)
      });
      plot.appendChild(band);
    });
    // The lanes and their leaves.
    layout.worked.forEach(function (branch) {
      var lane = layout.lane[branch.name], color = laneColor[branch.name];
      var mine = layout.commits.filter(function (commit) {
        return commit.branch === branch.name;
      });
      var baseSlot = (branch.base && layout.slot[branch.base] !== undefined)
        ? layout.slot[branch.base] : 0;
      var fork = pos(baseSlot, 0);
      var headAt = mine.length
        ? pos(layout.slot[mine[mine.length - 1].id], lane)
        : fork;
      // The sprout: from the fork node on the spine out to the lane.
      plot.appendChild(svg('path', {
        d: bend(fork, headAt, sideways), class: 'dag-lane', stroke: color
      }));
      // The lane's body through its commits.
      if (mine.length > 1) {
        var points = mine.map(function (commit) {
          var at = pos(layout.slot[commit.id], lane);
          return at.x + ' ' + at.y;
        }).join(' L ');
        var start = pos(layout.slot[mine[0].id], lane);
        plot.appendChild(svg('path', {
          d: 'M ' + start.x + ' ' + start.y + ' L ' + points, class: 'dag-lane', stroke: color
        }));
      }
      // The fork node at the branch's actual base.
      var forkNode = svg('rect', {
        x: fork.x - (sideways ? 3 : 4), y: fork.y - (sideways ? 4 : 3),
        width: sideways ? 6 : 8, height: sideways ? 8 : 6,
        rx: 1.5, class: 'dag-fork'
      });
      bindNode(forkNode, nodes, {
        kind: 'fork', id: branch.name, at: fork,
        title: 'fork \u2190 ' + branch.name,
        lines: ['fork: ' + branch.name, 'from ' + (branch.base || 'main').slice(0, 12) +
          (branch.child ? ' \u00b7 child session ' + branch.child.session : '')],
        ref: branch.name, label: branch.name
      });
      plot.appendChild(forkNode);
      // The leaves.
      mine.forEach(function (commit) {
        var at = pos(layout.slot[commit.id], lane);
        var leaf = svg('circle', {
          cx: at.x, cy: at.y, r: 3.5, class: 'dag-leaf', fill: color
        });
        bindNode(leaf, nodes, {
          kind: 'commit', id: commit.id, at: at,
          title: 'C' + layout.ordinal[commit.id] + ' \u00b7 ' + branch.name,
          lines: [
            'C' + layout.ordinal[commit.id] + ' \u00b7 ' + (commit.message || '(no message)'),
            commit.id.slice(0, 12) + ' \u00b7 via ' + (commit.via || '?') +
              ' \u00b7 ' + historyTime(commit.time),
            bytes(commit.logical) + ' tree \u00b7 +' + bytes(commit.new_stored) + ' stored'
          ],
          ref: commit.id, label: 'commit ' + commit.id.slice(0, 12)
        });
        plot.appendChild(leaf);
      });
      // The head: the state dot and the name tag.
      var headDot = svg('circle', {
        cx: headAt.x, cy: headAt.y, r: 5, class: 'dag-head-dot', 'data-state': branch.state,
        fill: color
      });
      plot.appendChild(headDot);
      var tagAt = sideways
        ? { x: headAt.x + 9, y: headAt.y + 3 }
        : { x: headAt.x + 9, y: headAt.y + 3 };
      var tag = label(tagAt.x, tagAt.y, 'dag-tag', clip(branch.name, 18));
      tag.setAttribute('fill', color);
      bindNode(tag, nodes, {
        kind: 'branch', id: branch.name, at: headAt,
        title: branch.name + ' \u00b7 ' + historyStateWord(branch.state) +
          (branch.child ? ' \u00b7 child session ' + branch.child.session + ' \u00b7 ' + branch.child.outcome : ''),
        lines: [branch.name, historyStateWord(branch.state) +
          (branch.child ? ' \u00b7 ' + branch.child.session + ' \u00b7 ' + branch.child.outcome : '')],
        ref: branch.name, label: branch.name
      });
      plot.appendChild(tag);
    });
    // The grafts: a dashed stem from the leaf a publish carried up to
    // the band it made, with a ring at the stem's foot.
    layout.grafts.forEach(function (layer) {
      var from = pos(layout.slot[layer.source_commit], layout.lane[layer.source_branch.replace(/^merge-/, '')] || 0);
      var to = pos(layout.slot[layer.id], 0);
      plot.appendChild(svg('path', {
        d: bend(from, to, sideways), class: 'dag-graft', stroke: laneColor[layer.source_branch] || 'var(--dsw-alias-ink-tertiary)'
      }));
      plot.appendChild(svg('circle', {
        cx: from.x, cy: from.y, r: 6, class: 'dag-ring',
        stroke: laneColor[layer.source_branch] || 'var(--dsw-alias-ink-tertiary)'
      }));
    });
    if (!layout.layers.length && !layout.commits.length) {
      plot.appendChild(label(PAD, h / 2, 'dag-empty',
        'The Store holds its first layer and no commits yet.'));
    }
    // A node is anything clickable; the hover card and the click
    //through hang off this list.
    function bindNode(elt, list, fact) {
      elt.classList.add('dag-node');
      elt.style.cursor = 'pointer';
      var said = svg('title', null, fact.title);
      elt.appendChild(said);
      elt.addEventListener('click', function () {
        historyBrowse(fact.ref, fact.label);
      });
      list.push({ elt: elt, fact: fact });
    }
    function bindHover(scroll, wrap, hover) {
      nodes.forEach(function (one) {
        one.elt.addEventListener('mouseenter', function () {
          hover.textContent = '';
          var title = node('div', null, one.fact.title);
          hover.appendChild(title);
          one.fact.lines.forEach(function (line) {
            hover.appendChild(node('div', 'dag-hover-id', line));
          });
          hover.hidden = false;
          var host = scroll.getBoundingClientRect();
          var box = one.elt.getBoundingClientRect();
          var left = box.left - host.left + scroll.scrollLeft + 14;
          hover.style.left = Math.min(left, host.width - 240) + 'px';
          hover.style.top = (box.top - host.top + scroll.scrollTop - 8) + 'px';
        });
        one.elt.addEventListener('mouseleave', function () {
          hover.hidden = true;
        });
      });
    }
    return {
      svg: plot, width: w, height: h,
      bindHover: bindHover
    };
  }

  // A bend that runs about as far along time as the rows it crosses
  // (near 45\u00b0), so a sprout or a graft stays where its event is.
  function bend(from, to, sideways) {
    var run = Math.abs(sideways ? (to.x - from.x) : (to.y - from.y));
    var rise = Math.abs(sideways ? (to.y - from.y) : (to.x - from.x));
    var reach = Math.min(run, Math.max(14, rise * 1.1));
    if (sideways) {
      var dx = Math.sign(to.x - from.x) * reach;
      return 'M' + from.x + ' ' + from.y +
        ' C' + (from.x + dx) + ' ' + from.y + ', ' +
        (to.x - dx) + ' ' + to.y + ', ' + to.x + ' ' + to.y;
    }
    var dy = Math.sign(to.y - from.y) * reach;
    return 'M' + from.x + ' ' + from.y +
      ' C' + from.x + ' ' + (from.y + dy) + ', ' +
      to.x + ' ' + (to.y - dy) + ', ' + to.x + ' ' + to.y;
  }

  // The minimap: the whole run compressed, one bar per slot (its height
  // the nodes at that slot), and the visible window. Click or drag
  // jumps; scrolling moves the window.
  function historyMinimap(layout) {
    var minimap = node('div', 'dag-minimap');
    minimap.setAttribute('role', 'scrollbar');
    minimap.setAttribute('aria-label', 'the whole run; click to jump');
    for (var t = 0; t < layout.totalSlots; t++) (function (t) {
      var bar = node('span', 'dag-minimap-bar');
      bar.style.left = (t / layout.totalSlots * 100) + '%';
      bar.style.height = Math.min(20, 4 + 5 * (countsBySlot(layout)[t] || 0)) + 'px';
      minimap.appendChild(bar);
    })(t);
    var window_ = node('span', 'dag-minimap-window');
    minimap.appendChild(window_);
    minimap.addEventListener('pointerdown', function (event) {
      var jump = function (event) {
        var box = minimap.getBoundingClientRect();
        var fraction = Math.max(0, Math.min(1, (event.clientX - box.left) / box.width));
        var scroll = minimap.parentNode.querySelector('.dag-scroll');
        if (scroll) scroll.scrollLeft = fraction * scroll.scrollWidth - scroll.clientWidth / 2;
      };
      jump(event);
      var move = function (event) { jump(event); };
      var up = function () {
        document.removeEventListener('pointermove', move);
        document.removeEventListener('pointerup', up);
      };
      document.addEventListener('pointermove', move);
      document.addEventListener('pointerup', up);
    });
    minimap._window = window_;
    return minimap;
  }

  function countsBySlot(layout) {
    if (layout._counts) return layout._counts;
    var counts = {};
    layout.layers.forEach(function (layer) {
      counts[layout.slot[layer.id]] = (counts[layout.slot[layer.id]] || 0) + 1;
    });
    layout.commits.forEach(function (commit) {
      counts[layout.slot[commit.id]] = (counts[layout.slot[commit.id]] || 0) + 1;
    });
    layout._counts = counts;
    return counts;
  }

  function historyMinimapWindow(scroll, minimap) {
    var win = minimap._window;
    if (!win) return;
    var left = scroll.scrollLeft / Math.max(1, scroll.scrollWidth);
    var width = scroll.clientWidth / Math.max(1, scroll.scrollWidth);
    win.style.left = (left * 100) + '%';
    win.style.width = (Math.min(1, width) * 100) + '%';
  }

  function historyTime(unixMillis) {
    if (!unixMillis) return 'no time recorded';
    var made = new Date(unixMillis);
    return made.toISOString().slice(0, 16).replace('T', ' ');
  }

  function historyBrowse(ref, label) {
    face.historyBranch = ref;
    face.historyRefLabel = label || ref;
    face.historyPath = '';
    face.historyPage = 0;
    readWorkspaceFiles(ref, '', true);
    paintWorkspaceHistory();
  }

  // The Work pane: every branch with work as one row — its head
  // commit's ordinal (C<n>), the layer it forked from (↓ L<k>), the
  // state the tree carries, its last activity, and the child session
  // the delegation rows join. Sorted by activity; the idle branches
  // live in the tree's chip. The session link opens the run in the
  // work surface.
  function historyWork(read) {
    var card = boardCard(12, 'the work', null,
      'Every branch a delegated child worked, against the tree: its head commit, the layer ' +
      'it forked from, its state, and whether the parent merged it back.');
    var layers = (read.layers || []).slice(); // newest first
    var ordinal = {};
    (read.commits || []).slice().sort(function (a, b) {
      return (a.time || 0) - (b.time || 0);
    }).forEach(function (commit, index) { ordinal[commit.id] = index + 1; });
    var layerAt = {};
    layers.forEach(function (layer, index) { layerAt[layer.id] = index; });
    var rows = (read.branches || []).filter(function (branch) {
      return branch.name !== 'main' && branch.head;
    });
    rows.forEach(function (branch) {
      var base = layerAt[branch.base || branch.forked_from];
      branch.layerBase = base !== undefined ? layers.length - base : null;
      var mine = (read.commits || []).filter(function (commit) {
        return commit.branch === branch.name;
      });
      branch.lastAt = mine.reduce(function (most, commit) {
        return Math.max(most, commit.time || 0);
      }, 0);
      branch.commitCount = mine.length;
    });
    rows.sort(function (a, b) { return (b.lastAt || 0) - (a.lastAt || 0); });
    if (!rows.length) {
      card.appendChild(node('p', 'board-empty',
        'No delegated child has worked a branch on this Store yet. A delegation that ran ' +
        'inside the substrate lands here with its branch, its session and its merge state.'));
      return card;
    }
    var table = node('table', 'board-table');
    var head = node('tr');
    ['branch', 'head', 'forked from', 'state', 'last activity', 'child session'].forEach(function (name) {
      head.appendChild(node('th', null, name));
    });
    table.appendChild(head);
    rows.forEach(function (branch) {
      var line = node('tr');
      var branchCell = node('td');
      var link = node('button', 'link', clip(branch.name, 40));
      link.type = 'button';
      link.addEventListener('click', function () { historyBrowse(branch.name); });
      branchCell.appendChild(link);
      line.appendChild(branchCell);
      line.appendChild(node('td', null,
        'C' + (ordinal[branch.head] || '?') + ' \u00b7 ' + branch.commitCount +
        (branch.commitCount === 1 ? ' commit' : ' commits')));
      line.appendChild(node('td', null,
        branch.layerBase ? ('\u2193 L' + branch.layerBase) : '\u2193 ?'));
      var state = node('td');
      state.appendChild(node('span', 'chip' + (branch.state === 'merged' ? ' is-ok' : ''),
        historyStateWord(branch.state)));
      line.appendChild(state);
      line.appendChild(node('td', null, historyTime(branch.lastAt)));
      line.appendChild(node('td', null,
        branch.child ? clip(branch.child.session, 26) + ' \u00b7 ' + (branch.child.outcome || '?') : '\u2014'));
      table.appendChild(line);
    });
    card.appendChild(table);
    return card;
  }

  // The live feed: the substrate's own call trace, tailed from the
  // machine's traces — the actor (the trace file, one per agent), the
  // call, its cost, whether it landed, and the commit it made. Traces
  // off is named, never read as "no activity".
  function historyCalls(read) {
    var card = boardCard(12, 'the calls', null,
      'What the substrate is doing right now: every call an agent or the backend made, ' +
      'as the trace recorded it. A delegation\u2019s child traces to its own file, so its ' +
      'work shows under its session.');
    if (read.note) {
      card.appendChild(node('p', 'board-note', read.note));
    }
    if ((read.refused || []).length) {
      card.appendChild(refusedCard(read.refused));
      return card;
    }
    var calls = read.calls || [];
    if (!calls.length) {
      if (!read.note) {
        card.appendChild(node('p', 'board-empty',
          'No calls recorded in this window yet. The feed moves as the substrate is used.'));
      }
      return card;
    }
    if (read.more) {
      card.appendChild(node('p', 'board-note', 'showing the newest 200 ' + (read.note || '')));
    }
    var table = node('table', 'board-table');
    var head = node('tr');
    ['when', 'actor', 'call', 'ms', 'made'].forEach(function (name) {
      head.appendChild(node('th', null, name));
    });
    table.appendChild(head);
    calls.forEach(function (call) {
      var line = node('tr');
      line.appendChild(node('td', null, clip(String(call.start || '').replace('T', ' ').slice(11, 19), 8)));
      line.appendChild(node('td', null, clip(String(call.actor || ''), 24)));
      var method = node('td');
      method.appendChild(node('span', 'board-path', String(call.method || '?')));
      line.appendChild(method);
      line.appendChild(node('td', null,
        call.ms != null ? Math.round(call.ms * 10) / 10 + ' ms' : '\u2014'));
      var made = node('td');
      if (call.ok === false) {
        made.appendChild(node('span', 'chip', 'refused'));
      } else if (call.commit) {
        made.appendChild(node('span', 'board-path', clip(String(call.commit), 12)));
      } else if (call.kind) {
        made.appendChild(node('span', null, String(call.kind)));
      } else {
        made.appendChild(node('span', null, call.ok === true ? 'ok' : '\u2014'));
      }
      line.appendChild(made);
      table.appendChild(line);
    });
    card.appendChild(table);
    return card;
  }

  // The Files instrument: a ref's tree at a path, and the file's own
  // instruments beside it — text (the file's text, a window at a time),
  // blame (every line with the Commit that last changed it), revisions
  // (the file's history), and diff (the changed set between two
  // commits, or one changed file's hunks). A ref is a branch name, a
  // commit id or a layer id — clicking a tree node reads there.
  function historyFiles(read) {
    if (!face.historyMode) face.historyMode = 'text';
    var card = boardCard(12, 'the files', null,
      'A ref\u2019s tree at a path \u2014 branch name, commit id or layer id \u2014 and, once a ' +
      'file is open: its text, its blame, its revisions, and the diff it last took.');
    var branches = (read.branches || []).map(function (b) { return b.name; });
    if (!branches.length) return card;
    // The tabs: every branch, plus the ref a tree node clicked (when it
    // is not itself a branch — a commit or a layer).
    var tabs = node('div', 'board-tabs');
    var at = face.historyBranch || 'main';
    branches.forEach(function (name) {
      var tab = node('button', 'board-tab' + (name === at ? ' is-on' : ''), name);
      tab.type = 'button';
      tab.addEventListener('click', function () { historyBrowse(name); });
      tabs.appendChild(tab);
    });
    if (face.historyRefLabel && branches.indexOf(at) < 0) {
      var nodeTab = node('button', 'board-tab is-on',
        '\u25c6 ' + face.historyRefLabel);
      nodeTab.type = 'button';
      tabs.appendChild(nodeTab);
    }
    card.appendChild(tabs);
    // The modes: the file's instruments. Diff stands on the ref's own
    // ends (the commit's parent or base), so it is offered only when
    // the read knows them.
    var modes = node('div', 'board-tabs');
    var ends = historyDiffEnds(read, at);
    [['text', 'text'], ['blame', 'blame'], ['revisions', 'revisions'],
     ['diff', 'diff']].forEach(function (pair) {
      var mode = pair[0];
      var tab = node('button', 'board-tab' + (mode === face.historyMode ? ' is-on' : ''),
        pair[1]);
      tab.type = 'button';
      if (mode === 'diff' && !ends) {
        tab.disabled = true;
        tab.title = 'a diff needs the ref\u2019s own parent or base';
      }
      tab.addEventListener('click', function () {
        face.historyMode = mode;
        face.historyDiffPath = '';
        face.historyFilePage = 0;
        face.historyPath = mode === 'diff' ? '' : (face.historyPath || '');
        if (mode !== 'diff' && !face.historyPath) {
          // The file's instruments stand on an open file; the listing
          // stays the landing.
          return paintWorkspaceHistory();
        }
        readWorkspaceFile(true);
      });
      modes.appendChild(tab);
    });
    card.appendChild(modes);
    var where = node('p', 'board-path',
      (face.historyPath || '/') + '  \u00b7  ' +
      (face.historyRefLabel || face.historyBranch));
    card.appendChild(where);
    // The listing: navigate folders, read files.
    var doc = face.historyFiles;
    if (!doc || doc.branch !== face.historyBranch) {
      card.appendChild(node('p', 'board-empty', 'reading ' + face.historyBranch + ' \u2026'));
      return card;
    }
    if ((doc.refused || []).length) {
      card.appendChild(refusedCard(doc.refused));
      return card;
    }
    var items = doc.items || [];
    var table = node('table', 'board-table');
    items.forEach(function (item) {
      var line = node('tr');
      var name = item.name || item.path || '(no name)';
      var isDir = !!(item.dir || item.kind === 'dir' || item.type === 'dir' || item.type === 'directory');
      var cell = node('td');
      var button = node('button', 'link', (isDir ? '\u{1f4c2} ' : '') + name);
      button.type = 'button';
      button.addEventListener('click', function () {
        if (isDir) {
          var next = (face.historyPath || '/').replace(/\/$/, '') + '/' + name;
          face.historyPath = next;
          face.historyPage = 0;
          readWorkspaceFiles(face.historyBranch, next, true);
          paintWorkspaceHistory();
        } else {
          face.historyPath = (face.historyPath || '/').replace(/\/$/, '') + '/' + name;
          face.historyMode = 'text';
          face.historyFilePage = 0;
          face.historyDiffPath = '';
          readWorkspaceFile(true);
          paintWorkspaceHistory();
        }
      });
      cell.appendChild(button);
      line.appendChild(cell);
      line.appendChild(node('td', null, isDir ? 'dir' : (item.size != null ? bytes(item.size) : '')));
      table.appendChild(line);
    });
    if (items.length) card.appendChild(table);
    if (doc.has_more) {
      card.appendChild(node('p', 'board-note', 'more entries below \u2014 the page is cut at 200'));
    }
    // The instrument: the active mode's answer.
    if (face.historyMode !== 'diff' && !face.historyPath) return card;
    var instrument = face.historyFile;
    if (!instrument || instrument.key !== historyFileKey()) {
      card.appendChild(node('p', 'board-empty',
        'reading ' + face.historyMode + ' of ' + (face.historyPath || face.historyRefLabel) + ' \u2026'));
      return card;
    }
    if ((instrument.refused || []).length) {
      card.appendChild(refusedCard(instrument.refused));
      return card;
    }
    if (instrument.note) {
      card.appendChild(node('p', 'board-note', instrument.note));
    }
    if (instrument.mode === 'text') {
      if (instrument.binary) {
        card.appendChild(node('p', 'board-empty', instrument.note));
        return card;
      }
      var pre = node('pre', 'history-code');
      var numbered = String(instrument.text || '').split('\n').map(function (line, index) {
        return String(instrument.start_line + index) + '  ' + line;
      }).join('\n');
      pre.textContent = numbered;
      card.appendChild(pre);
      var window_ = node('div', 'board-tabs');
      if (face.historyFilePage > 0) {
        var back = node('button', 'board-tab', '\u2190 window');
        back.type = 'button';
        back.addEventListener('click', function () {
          face.historyFilePage -= 1;
          readWorkspaceFile(true);
          paintWorkspaceHistory();
        });
        window_.appendChild(back);
      }
      if (instrument.next_line) {
        var next = node('button', 'board-tab', 'window \u2192');
        next.type = 'button';
        next.addEventListener('click', function () {
          face.historyFilePage += 1;
          readWorkspaceFile(true);
          paintWorkspaceHistory();
        });
        window_.appendChild(next);
      }
      if (window_.children.length) card.appendChild(window_);
      return card;
    }
    if (instrument.mode === 'blame') {
      var blame = node('table', 'board-table');
      (instrument.lines || []).forEach(function (row) {
        var line = node('tr');
        line.appendChild(node('td', null, String(row.n)));
        var commitCell = node('td');
        var commitLink = node('button', 'link', clip(String(row.commit || '\u2014'), 12));
        commitLink.type = 'button';
        commitLink.addEventListener('click', function () {
          historyBrowse(row.commit, 'commit ' + clip(String(row.commit), 12));
        });
        commitCell.appendChild(commitLink);
        line.appendChild(commitCell);
        var text = node('td', 'history-blame-text', row.text == null ? '' : row.text);
        line.appendChild(text);
        blame.appendChild(line);
      });
      card.appendChild(blame);
      return card;
    }
    if (instrument.mode === 'revisions') {
      var revisions = node('table', 'board-table');
      (instrument.items || []).forEach(function (row) {
        var line = node('tr');
        var cell = node('td');
        var link = node('button', 'link', clip(String(row.commit || row.object || '\u2014'), 12));
        link.type = 'button';
        link.addEventListener('click', function () {
          historyBrowse(row.commit, 'commit ' + clip(String(row.commit || ''), 12));
        });
        cell.appendChild(link);
        line.appendChild(cell);
        line.appendChild(node('td', null, row.change || '\u2014'));
        line.appendChild(node('td', null, row.message || '\u2014'));
        line.appendChild(node('td', null, row.size != null ? bytes(row.size) : ''));
        revisions.appendChild(line);
      });
      card.appendChild(revisions);
      if (instrument.has_more) {
        card.appendChild(node('p', 'board-note', 'more revisions below \u2014 the page is cut at 50'));
      }
      return card;
    }
    if (instrument.mode === 'diff') {
      if (instrument.hunks) {
        var hunks = node('div', 'history-diff');
        (instrument.changes || []).forEach(function (row) {
          var line = node('div', 'history-diff-line history-diff-' +
            (row.op === '+' ? 'add' : row.op === '-' ? 'del' : 'ctx'));
          line.textContent = (row.op || ' ') + ' ' + (row.text || '');
          hunks.appendChild(line);
        });
        card.appendChild(hunks);
        return card;
      }
      var paths = node('table', 'board-table');
      (instrument.items || []).forEach(function (row) {
        var line = node('tr');
        var cell = node('td');
        var link = node('button', 'link', row.path + ' \u00b7 ' + (row.kind || ''));
        link.type = 'button';
        link.addEventListener('click', function () {
          face.historyDiffPath = row.path;
          face.historyPath = row.path;
          readWorkspaceFile(true);
          paintWorkspaceHistory();
        });
        cell.appendChild(link);
        line.appendChild(cell);
        paths.appendChild(line);
      });
      card.appendChild(paths);
      return card;
    }
    return card;
  }

  // The diff's ends: the ref's own parent or base — the commit's
  // parent when it is in the answer, else the layer (or first commit)
  // it stands on. Both ids ride the server, which resolves a layer
  // through the substrate's own comparable mapping and names a
  // NotComparable layer (the import's own) by refusal.
  function historyDiffEnds(read, ref) {
    var commits = read.commits || [];
    var layers = read.layers || [];
    var byId = {};
    commits.forEach(function (commit) { byId[commit.id] = commit; });
    layers.forEach(function (layer) { byId[layer.id] = layer; });
    var branch = (read.branches || []).find(function (one) { return one.name === ref; });
    var to = branch ? branch.head : ref;
    var node_ = byId[to];
    if (!node_) return null;
    var from = node_.parent || node_.base || null;
    return (from && from !== to) ? { from: from, to: to } : null;
  }

  function historyFileKey() {
    return [face.historyBranch, face.historyPath, face.historyMode,
      face.historyDiffPath || '', face.historyFilePage || 0].join('\u0000');
  }

  // The instrument's page: one read per (ref, path, mode, page),
  // cached on the face so a repaint does not re-ask.
  function readWorkspaceFile(force) {
    var ref = face.historyBranch;
    if (!ref) return;
    var read = face.history;
    var ends = historyDiffEnds(read || {}, ref) || {};
    var key = historyFileKey();
    var now = Date.now();
    if (!force && face.historyFile && face.historyFileKey === key &&
        now - face.historyFileAt < 5000) {
      return;
    }
    face.historyFileAt = now;
    var asked = '/api/workspace-history/file?ref=' + encodeURIComponent(ref) +
      '&path=' + encodeURIComponent(
        face.historyMode === 'diff' ? (face.historyDiffPath || '') : (face.historyPath || '')) +
      '&mode=' + encodeURIComponent(face.historyMode) +
      '&from=' + encodeURIComponent(ends.from || '') +
      '&to=' + encodeURIComponent(ends.to || '') +
      '&page=' + (face.historyFilePage || 0);
    return fetch(asked, { headers: { accept: 'application/json' } })
      .then(function (reply) {
        if (reply.status === 401) throw new Error('token');
        return reply.json();
      })
      .then(function (doc) {
        doc.key = key;
        face.historyFile = doc;
        face.historyFileKey = key;
        if (face.panel === 'workspace-history') paintWorkspaceHistory();
      })
      .catch(function () { });
  }

  // The tree's keyboard follow: F reads the NEWEST node — the trunk's
  // head Layer (the answer carries layers newest first), or the newest
  // commit when the Store holds no Layer yet. The reference's own
  // binding; a node's hover card names it.
  window.addEventListener('keydown', function (event) {
    if (face.panel !== 'workspace-history' || event.metaKey || event.ctrlKey || event.altKey) return;
    if (event.key !== 'f' && event.key !== 'F') return;
    var tag = (event.target && event.target.tagName) || '';
    if (tag === 'INPUT' || tag === 'TEXTAREA' ||
        (event.target && event.target.isContentEditable)) return;
    var read = face.history;
    if (!read) return;
    var layers = read.layers || [];
    var commits = (read.commits || []).slice().sort(function (a, b) {
      return (b.time || 0) - (a.time || 0);
    });
    var newest = layers[0] || commits[0];
    if (!newest) return;
    event.preventDefault();
    historyBrowse(newest.id, layers[0]
      ? 'layer ' + newest.id.slice(0, 12)
      : 'commit ' + newest.id.slice(0, 12));
  });

  // The Store's accounting, in the substrate's own terms: what it
  // holds on disk, what main's head would cost as plain files, the
  // whole history's raw size, the unique content, its packed form, the
  // metadata subset (a SUBSET of the packed bytes — never added), and
  // the SQLite overhead named separately (file − free − stored). The
  // ratios are computed from the bytes, never trusted as strings. The
  // Init record says what the import carried. Per-commit costs come
  // from the history's own commits — average, largest, and one bar
  // per commit oldest→newest, coloured by how it was made.
  function historyStorage(read) {
    var card = boardCard(12, 'the store', null,
      'What the substrate holds, in its own terms: on disk, main\u2019s latest as plain ' +
      'files, every snapshot raw, the unique content, its packed form — and what each ' +
      'commit cost.');
    var storage = read.storage;
    if (!storage) {
      card.appendChild(node('p', 'board-empty',
        'The figures have not landed. They are read through the bundle\u2019s backend, the ' +
        'one route to the Store\u2019s accounting.'));
      return card;
    }
    var layers = (read.layers || []).slice(); // newest first
    var latest = layers.length ? layers[0].logical : 0;
    var overhead = storage.file - storage.free - storage.stored;
    var rows = [
      ['on disk', storage.file, 'what the Store\u2019s database occupies'],
      ['main\u2019s latest, raw', latest, 'one plain copy of main\u2019s head'],
      ['every snapshot, raw', storage.logical, 'the whole history as plain files'],
      ['unique', storage.canonical, 'each distinct file and chunk once'],
      ['compressed', storage.stored, 'that content packed'],
      ['of which metadata', storage.metadata, 'a subset of the packed bytes \u2014 not additional'],
      ['SQLite overhead', overhead, 'file \u2212 free \u2212 stored, named separately'],
    ];
    var widest = rows.reduce(function (most, row) { return Math.max(most, row[1]); }, 1);
    var bars = node('div', 'figure-bars');
    rows.forEach(function (row) {
      var line = node('div', 'figure-bar');
      line.appendChild(node('span', 'figure-bar-name', row[0]));
      var track = node('span', 'figure-bar-track');
      var fill = node('span', 'figure-bar-fill');
      fill.style.width = Math.max(2, Math.round((row[1] / widest) * 100)) + '%';
      track.appendChild(fill);
      line.appendChild(track);
      line.appendChild(node('span', 'figure-bar-value', bytes(row[1])));
      line.appendChild(node('span', 'figure-bar-note', row[2]));
      bars.appendChild(line);
    });
    card.appendChild(bars);
    // The ratios, computed from the byte figures the answer carried.
    var dedupe = storage.canonical > 0 ? storage.logical / storage.canonical : null;
    var compression = storage.stored > 0 ? storage.canonical / storage.stored : null;
    card.appendChild(node('p', 'board-note',
      (dedupe != null ? Math.round(dedupe * 100) / 100 + '\u00d7 dedupe' : 'dedupe \u2014') +
      ' \u00b7 ' +
      (compression != null ? Math.round(compression * 100) / 100 + '\u00d7 compression' :
        'compression \u2014') +
      ' \u2014 the canonical tree shared between branches is kept once.'));
    // The Init record: what the import carried and what it cost, or
    // the named absence of it.
    if (storage.init) {
      card.appendChild(node('p', 'board-note',
        'init: ' + storage.init.files + ' files \u00b7 ' + bytes(storage.init.bytes) +
        ' imported from ' + storage.init.entries + ' entries in ' + storage.init.millis +
        ' ms.'));
    } else {
      card.appendChild(node('p', 'board-note',
        'the Store carries no Init record (it was built before Init recorded one).'));
    }
    // The per-commit costs: the history's own commits carry what each
    // Save added. `store.branch.list_commit_storage` refuses main —
    // main is a Layer chain — so main's per-commit figures ride the
    // layers, and the bars here are every commit the read carried.
    var commits = (read.commits || []).slice().sort(function (a, b) {
      return (a.time || 0) - (b.time || 0);
    });
    var costed = commits.filter(function (commit) { return commit.new_stored > 0; });
    if (costed.length) {
      var sum = costed.reduce(function (total, commit) { return total + commit.new_stored; }, 0);
      var largest = costed.reduce(function (most, commit) {
        return Math.max(most, commit.new_stored);
      }, 0);
      card.appendChild(node('p', 'board-note',
        'per commit: ' + bytes(Math.round(sum / costed.length)) + ' average \u00b7 ' +
        bytes(largest) + ' largest \u2014 from the ' + costed.length +
        (costed.length === 1 ? ' commit' : ' commits') + ' the read carried.'));
      var tall = Math.max.apply(null, costed.map(function (commit) { return commit.new_stored; }));
      var chart = node('div', 'figure-bars');
      costed.forEach(function (commit) {
        var line = node('div', 'figure-bar');
        line.appendChild(node('span', 'figure-bar-name',
          clip(commit.id, 8)));
        var track = node('span', 'figure-bar-track');
        var fill = node('span', 'figure-bar-fill');
        fill.style.width = Math.max(2, Math.round((commit.new_stored / tall) * 100)) + '%';
        fill.dataset.via = commit.via || '?';
        track.appendChild(fill);
        line.appendChild(track);
        line.appendChild(node('span', 'figure-bar-value', bytes(commit.new_stored)));
        line.appendChild(node('span', 'figure-bar-note',
          (commit.message || '(no message)') + ' \u00b7 via ' + (commit.via || '?')));
        line.title = 'commit ' + commit.id + '\n' + bytes(commit.logical) + ' tree \u00b7 +' +
          bytes(commit.new_canonical) + ' canonical \u00b7 +' + bytes(commit.new_stored) +
          ' stored';
        chart.appendChild(line);
      });
      card.appendChild(chart);
    } else {
      card.appendChild(node('p', 'board-note',
        'no commit\u2019s own cost was carried in this read \u2014 per-commit figures ride the ' +
        'commits, and main\u2019s layers carry theirs.'));
    }
    // The layers as the store's own accounting: what each publish cost.
    if (layers.length) {
      var table = node('table', 'board-table');
      layers.forEach(function (layer) {
        var line = node('tr');
        line.appendChild(node('td', null, layer.id.slice(0, 12)));
        line.appendChild(node('td', null, bytes(layer.logical)));
        line.appendChild(node('td', null, layer.parent ? layer.parent.slice(0, 12) : 'the root'));
        line.appendChild(node('td', null,
          layer.source_branch ? layer.source_branch.replace(/^merge-/, '') : 'the import'));
        line.appendChild(node('td', null,
          layer.overwrote_count ? layer.overwrote_count + ' replaced' : ''));
        table.appendChild(line);
      });
      card.appendChild(table);
    }
    return card;
  }

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
        else if (face.panel === 'vault') paintVault();
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

  /// The paired devices, read beside the channel peers. A device token is
  /// stored as a digest, so this names who is paired and never what would
  /// let them in.
  function readAccess(force) {
    var now = Date.now();
    if (!force && face.access && now - face.accessAt < 4000) return Promise.resolve();
    face.accessAt = now;
    return fetch('/api/access', { headers: { accept: 'application/json' } })
      .then(function (reply) {
        if (reply.status === 401) throw new Error('token');
        return reply.json();
      })
      .then(function (doc) {
        face.access = doc;
        if (face.panel === 'bots') paintBots();
      })
      .catch(function () { });
  }

  /// The paired phones and browsers. The store holds token digests, so a
  /// row names the device and never the token; revoking forgets it.
  function devicesCard() {
    var read = face.access;
    var devices = (read && read.devices) || [];
    var card = boardCard(12, 'paired devices', devices.length
      ? devices.length + (devices.length === 1 ? ' device' : ' devices')
      : null,
      'A device token is stored as a digest; this names who is paired and never what would ' +
      'let them in. Revoking forgets the token.');
    if (!read) {
      card.appendChild(node('p', 'board-empty',
        'The access read has not landed yet. It reads the home\u2019s device store.'));
      return card;
    }
    if (!devices.length) {
      card.appendChild(node('p', 'board-empty',
        'No device is paired. Mint a code from the pair button to let a phone in.'));
      return card;
    }
    devices.forEach(function (device) {
      var row = node('div', 'bots-row');
      var head = node('div', 'bots-row-main');
      head.appendChild(node('span', 'bots-row-name', device.name || device.id));
      head.appendChild(node('span', 'bots-row-meta', 'paired ' + (device.paired || '\u2014')));
      if (device.seen) head.appendChild(node('span', 'bots-row-meta', 'last seen ' + device.seen));
      row.appendChild(head);
      var acts = node('div', 'bots-acts');
      acts.appendChild(botsButton('revoke-device', 'revoke', function () {
        if (!window.confirm('Revoke the device `' + (device.name || device.id) + '`?')) return;
        post('/api/access/revoke', { kind: 'device', id: device.id }).then(function () {
          readAccess(true);
        });
      }));
      row.appendChild(acts);
      card.appendChild(row);
    });
    return card;
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

  /// Take the owner to a card without changing what the board shows. The
  /// door and the activity are their own cards, not filters of the peers
  /// list, so their tiles navigate instead of pressing an empty chip.
  function revealCard(id, flash) {
    var card = document.getElementById(id);
    if (!card) return;
    var smooth = !window.matchMedia ||
      !window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    card.scrollIntoView({ block: 'start', behavior: smooth ? 'smooth' : 'auto' });
    if (flash) {
      card.dataset.flash = 'on';
      window.setTimeout(function () { delete card.dataset.flash; }, 1600);
    }
  }

  function revealDoor() {
    revealCard('bots-door', true);
  }

  function revealActivity() {
    revealCard('bots-activity', true);
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
    var waitingSeats = Number(read.waiting_seats || 0);
    var instrument = surfaceInstrument('the service',
      live + ' of ' + reports.length + ' live',
      service.stale
        ? (service.note || 'the heartbeat has stopped')
        : (service.pid
          ? 'pid ' + service.pid + ' \u00b7 started ' + (service.started || '\u2014')
          : (service.note || 'no channel service has run in this home')),
      [{
        label: 'open settings',
        run: function () { window.location.hash = '#/settings'; }
      }]);
    el.botsBoard.appendChild(surfaceHead('System', 'Bots',
      'The channels that carry work in from outside: who is paired, what they may do, ' +
      'whether the service is listening, and what they have asked for.', instrument));
    el.botsBoard.appendChild(surfaceStrip([
      [live, 'channels live'],
      [failed, 'failed', failed ? 'down' : null],
      [peers.length, 'peers paired'],
      [pending.length, 'waiting at the door', pending.length ? 'warn' : null],
      [refusedAtTheDoor, 'refusals logged'],
      [waitingSeats, 'waiting on an answer']
    ], read.read_at));
    el.botsBoard.appendChild(filterBar([
      { value: 'all', label: 'all', count: peers.length },
      { value: 'acting', label: 'may act',
        count: peers.filter(function (peer) { return peer.capability === 'task'; }).length },
      { value: 'listening', label: 'may only listen',
        count: peers.filter(function (peer) { return peer.capability === 'observe'; }).length }
    ], filter, function (value) {
      face.botsFilter = value;
      paintBots();
    }));

    // The service: what is listening, and the start/stop that changes it.
    // A stale heartbeat is named rather than rendered as a live service.
    // The stamp names the heartbeat, not a channel's phase: the process
    // can be running while every channel in it is stopped.
    var serviceCard = boardCard(12, 'the service',
      service.stale ? (service.updated || 'last report') : (service.pid ? 'running' : null),
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
    // The Telegram credential, write-only: the value is sent once, the
    // input is cleared, and no read carries it back.
    var telegram = read.telegram || { present: false, source: 'unset' };
    var credential = node('div', 'bots-row');
    var credentialHead = node('div', 'bots-row-main');
    credentialHead.appendChild(node('span', 'bots-row-name', 'telegram'));
    credentialHead.appendChild(badge(telegram.present
      ? 'token \u00b7 ' + telegram.source
      : 'no token', telegram.present ? 'ok' : 'warn'));
    credential.appendChild(credentialHead);
    var token = node('input', 'bots-input');
    token.type = 'password';
    token.autocomplete = 'off';
    token.placeholder = 'FLEET_TELEGRAM_BOT_TOKEN';
    token.setAttribute('aria-label', 'the Telegram bot token');
    credential.appendChild(token);
    var credentialActs = node('div', 'bots-acts');
    credentialActs.appendChild(botsButton('seal-token', 'seal the token', function () {
      if (!token.value.trim()) return;
      var value = token.value;
      token.value = '';
      post('/api/settings', {
        op: 'secret',
        name: 'FLEET_TELEGRAM_BOT_TOKEN',
        value: value
      }).then(function (reply) {
        if (reply.status !== 200) {
          notice('the token was not sealed',
            (reply.body && reply.body.error) || 'the writer refused it');
          return;
        }
        readBots(true);
      });
    }));
    credential.appendChild(credentialActs);
    serviceCard.appendChild(credential);
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
    el.botsBoard.appendChild(devicesCard());

    // Waiting at the door: a refusal recorded the peer, so it can be let
    // in from here instead of hunting an id out of a log.
    var doorCard = boardCard(12, 'waiting at the door', pending.length
      ? pending.length + (pending.length === 1 ? ' peer' : ' peers')
      : null,
      'peers that tried to reach a channel and are not paired. Letting one in mints its ' +
      'credential; turning one away forgets it until it tries again.');
    doorCard.id = 'bots-door';
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
      'A webhook peer gets a token minted here and shown once, and a callback where its ' +
      'outcomes go. A Telegram chat is let in by the id it refused under. The capability is ' +
      'part of the pairing, not a later edit.');
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
    idInput.placeholder = 'a telegram chat id';
    var idField = labelled('peer id', idInput);
    form.appendChild(idField);
    var nameInput = node('input', 'bots-input');
    nameInput.name = 'name';
    nameInput.type = 'text';
    nameInput.placeholder = 'the CI box';
    form.appendChild(labelled('name', nameInput));
    var callbackInput = node('input', 'bots-input');
    callbackInput.name = 'callback';
    callbackInput.type = 'text';
    callbackInput.placeholder = 'http://127.0.0.1:9/outcome (optional)';
    var callbackField = labelled('callback', callbackInput);
    form.appendChild(callbackField);
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
    // The fields follow the channel: a webhook peer mints its own id and
    // answers a callback; a Telegram peer is identified by its chat id
    // and has no callback to name.
    function syncPairFields() {
      var webhook = channelPick.value === 'webhook';
      idField.hidden = webhook;
      callbackField.hidden = !webhook;
      idInput.required = !webhook;
    }
    channelPick.addEventListener('change', syncPairFields);
    syncPairFields();
    form.addEventListener('submit', function (event) {
      event.preventDefault();
      writeBots('/api/channels/pair', {
        channel: channelPick.value,
        id: idInput.value,
        name: nameInput.value,
        capability: capPick.value,
        callback: callbackInput.value
      }, revealPairing);
    });
    pairCard.appendChild(form);
    el.botsBoard.appendChild(pairCard);

    // Who can reach the fleet: the peers, their capability, and the two
    // actions that change it.
    var shown = peers.filter(function (peer) {
      if (filter === 'acting') return peer.capability === 'task';
      if (filter === 'listening') return peer.capability === 'observe';
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
        usageRecord('actions', 'bots:inspect-peer', 'uses');
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
    var fedCard = boardCard(12, 'what they have done',
      activity.length
        ? (activity.length >= bound
          ? 'the newest ' + activity.length + ' \u00b7 the read\u2019s bound'
          : 'all ' + activity.length + (activity.length === 1 ? ' line' : ' lines'))
        : null,
      'deliveries, refusals, replies, seats, and state changes, newest last. The log is the ' +
      'service\u2019s own file; a row opens the peer, the session, or the channel it names.');
    fedCard.id = 'bots-activity';
    if (!activity.length) {
      fedCard.appendChild(node('p', 'board-empty',
        'Nothing recorded yet. The log fills as peers deliver, answer, and are refused.'));
    }
    var activityFilter = face.botsActivityFilter || 'all';
    var countKind = function (kind) {
      return activity.filter(function (line) { return line.kind === kind; }).length;
    };
    fedCard.appendChild(filterBar([
      { value: 'all', label: 'all', count: activity.length },
      { value: 'delivery', label: 'ran', count: countKind('delivery') },
      { value: 'refusal', label: 'refused', count: countKind('refusal') },
      { value: 'seat', label: 'seats', count: countKind('seat') }
    ], activityFilter, function (value) {
      face.botsActivityFilter = value;
      paintBots();
    }));
    var feed = node('div', 'bots-feed');
    activity.slice().reverse().forEach(function (line) {
      if (activityFilter !== 'all' && line.kind !== activityFilter) return;
      feed.appendChild(activityRow(line, peers, pending, channels, reports, activity));
    });
    if (!feed.childNodes.length) {
      feed.appendChild(node('p', 'board-empty',
        'No line of this kind is in the read. Clear the filter to see the rest.'));
    }
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
    if (doc.callback) {
      sections.push(['where outcomes go', node('p', 'derivation',
        doc.callback + ' \u2014 a delivery may name its own callback, and that one wins')]);
    } else if (doc.channel === 'webhook') {
      sections.push(['where outcomes go', node('p', 'derivation',
        'nowhere yet: each delivery names its own callback, and one that names none ' +
        'gets no reply. Re-pair with a callback to give it a default.')]);
    }
    if (doc.token) {
      var token = node('code', 'bots-token', doc.token);
      // The token wraps in the dock's narrow column, so a copy button is
      // the reliable way to move it: reading a broken line back by eye is
      // how a token gets mistyped.
      var tokenBox = node('div', 'bots-token-box');
      tokenBox.appendChild(token);
      var copy = node('button', 'replay-button', 'copy the token');
      copy.type = 'button';
      copy.dataset.act = 'copy-token';
      copy.addEventListener('click', function () {
        if (navigator.clipboard) {
          navigator.clipboard.writeText(doc.token).catch(function () { });
        }
        copy.textContent = 'copied';
        window.setTimeout(function () { copy.textContent = 'copy the token'; }, 1200);
      });
      tokenBox.appendChild(copy);
      sections.push(['the token \u2014 shown once', tokenBox]);
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

  /// One peer's inspector: what it may do, its id, where its conversation
  /// is, and what it last did.
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
      ['its id', node('p', 'derivation', peer.channel + ' \u00b7 ' + peer.id)],
      ['where outcomes go', node('p', 'derivation', peer.callback
        ? peer.callback + ' \u2014 a delivery may name its own callback, and that one wins'
        : (peer.channel === 'webhook'
          ? 'nowhere stored \u2014 each delivery names its own callback, or gets no reply'
          : 'the chat itself'))],
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

  /// A peer at the door: how it asked, and the ways to let it in. The
  /// door card carries the same buttons; this is the object view of one
  /// row, so a line in the feed has somewhere to go.
  function openPending(peer) {
    var sections = [
      ['what it is', node('p', 'derivation', peer.channel + ' \u00b7 ' + peer.id)],
      ['how it asked', node('p', 'derivation',
        peer.attempts + (peer.attempts === 1 ? ' attempt' : ' attempts') +
        ' \u00b7 first ' + (peer.first_seen || '\u2014') +
        ' \u00b7 last ' + (peer.last_seen || '\u2014'))]
    ];
    var letIn = function (capability) {
      writeBots('/api/channels/pair', {
        channel: peer.channel,
        id: peer.id,
        name: peer.name,
        capability: capability
      }, revealPairing);
    };
    var acts = node('div', 'bots-acts');
    acts.appendChild(botsButton('let-in-task', 'let it in \u00b7 may act', function () {
      letIn('task');
    }));
    acts.appendChild(botsButton('let-in-listen', 'let it in \u00b7 may only listen', function () {
      letIn('observe');
    }));
    acts.appendChild(botsButton('turn-away', 'turn away', function () {
      writeBots('/api/channels/forget', { channel: peer.channel, id: peer.id }, function () {
        closeDrawer();
      });
    }));
    sections.push(['let it in', acts]);
    openDrawer(peer.name || peer.id, sections);
  }

  /// A channel's inspector: the phase and port a state line in the feed
  /// is about.
  function openChannel(name, reports, peers) {
    var report = null;
    (reports || []).forEach(function (row) {
      if (row.name === name) report = row;
    });
    var mine = (peers || []).filter(function (peer) { return peer.channel === name; });
    var sections = [
      ['what it is', node('p', 'derivation', report
        ? report.phase + (report.port ? ' on 127.0.0.1:' + report.port : '')
        : 'not started in the process this face hosts')],
      ['since', node('p', 'derivation', report && report.since ? report.since : '\u2014')],
      ['paired', node('p', 'derivation',
        mine.length + (mine.length === 1 ? ' peer' : ' peers'))]
    ];
    if (report && report.error) {
      sections.push(['why it stopped', node('p', 'derivation', report.error)]);
    }
    openDrawer(name, sections);
  }

  /// One line of the feed. Every row goes somewhere — the session it
  /// landed in, the peer it names, the door that peer is waiting at, or
  /// the channel a state line is about — because a visible row that does
  /// nothing is the dead end the audit refuses.
  function activityRow(line, peers, pending, channels, reports, activity) {
    var row = node('div', 'bots-activity');
    row.dataset.kind = line.kind;
    var paired = null;
    var atDoor = null;
    if (line.peer_id) {
      (peers || []).forEach(function (peer) {
        if (peer.id === line.peer_id && peer.channel === line.channel) paired = peer;
      });
      (pending || []).forEach(function (peer) {
        if (peer.id === line.peer_id && peer.channel === line.channel) atDoor = peer;
      });
    }
    var destination;
    if (line.session) {
      destination = 'the conversation ' + line.session;
    } else if (paired) {
      destination = paired.name || paired.id;
    } else if (atDoor) {
      destination = (atDoor.name || atDoor.id) + ' at the door';
    } else {
      destination = line.channel;
    }
    var open = function () {
      usageRecord('actions', 'bots:open-activity', 'uses');
      if (line.session) {
        window.location.hash = '#/sessions/' + encodeURIComponent(line.session);
      } else if (paired) {
        openPeer(paired, activity, peers, channels);
      } else if (atDoor) {
        openPending(atDoor);
      } else {
        openChannel(line.channel, reports, peers);
      }
    };
    row.dataset.act = 'open-activity';
    row.tabIndex = 0;
    row.setAttribute('role', 'button');
    row.title = 'open ' + destination;
    row.addEventListener('click', function (event) {
      if (event.target && event.target.closest && event.target.closest('button')) return;
      open();
    });
    row.addEventListener('keydown', function (event) {
      if (event.key === 'Enter' || event.key === ' ') {
        event.preventDefault();
        open();
      }
    });
    row.appendChild(node('span', 'bots-activity-at', clock(line.at)));
    row.appendChild(badge(line.kind, line.kind === 'refusal' ? 'down'
      : line.kind === 'delivery' ? 'ok' : null));
    var said = node('span', 'bots-activity-said');
    said.textContent = line.peer || line.peer_id || line.channel;
    row.appendChild(said);
    var detail = line.task || line.detail || '';
    if (detail) row.appendChild(node('span', 'bots-row-meta', detail));
    if (line.session) {
      var openSession = node('button', 'bots-session', line.session);
      openSession.type = 'button';
      openSession.dataset.act = 'open-session';
      openSession.title = 'open the conversation';
      openSession.addEventListener('click', function () {
        window.location.hash = '#/sessions/' + encodeURIComponent(line.session);
      });
      row.appendChild(openSession);
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

  function toolsCard() {
    var read = face.settings || {};
    var tools = read.tools;
    if (!tools) {
      var waiting = boardCard(6, 'the tools', null, null);
      waiting.appendChild(node('p', 'board-empty',
        'The tool read has not landed yet.'));
      return waiting;
    }
    var card = boardCard(6, 'the tools', String(tools.native.length) + ' native',
      'Above ' + tools.deferred_above + ' visible tools or ' + tools.definition_tokens +
      ' definition tokens the MCP tools are held behind tool_search; at most ' +
      tools.calls_per_turn + ' calls run in one turn.');
    var list = node('ul', 'tool-list');
    tools.native.forEach(function (tool) {
      var row = node('li', null, null);
      var state = tool.state || 'active';
      var chip = node('span', 'tool-state tool-state-' + state, state);
      row.appendChild(chip);
      row.appendChild(node('span', null, ' ' + tool.name + ' \u2014 ' + tool.summary));
      if (state === 'disabled') {
        var enable = node('button', 'tool-enable', 'enable');
        enable.type = 'button';
        enable.addEventListener('click', function (stop) {
          if (stop && stop.stopPropagation) stop.stopPropagation();
          writeSettings('tool', { name: tool.name, disabled: false });
        });
        row.appendChild(enable);
      } else if (state !== 'protected') {
        var off = node('button', 'tool-enable', 'disable');
        off.type = 'button';
        off.addEventListener('click', function (stop) {
          if (stop && stop.stopPropagation) stop.stopPropagation();
          writeSettings('tool', { name: tool.name, disabled: true });
        });
        row.appendChild(off);
      }
      list.appendChild(row);
    });
    card.appendChild(list);
    if (tools.last_sweep) {
      card.appendChild(node('p', 'board-note',
        'last sweep ' + tools.last_sweep + ' \u00b7 window ' + (tools.window || 50) + ' sessions'));
    }
    if (tools.last_session) {
      var last = tools.last_session;
      var methods = Object.keys(last.methods || {}).map(function (key) {
        return key + ' ' + last.methods[key];
      }).join(' \u00b7 ');
      card.appendChild(node('p', 'board-note',
        'last session ' + last.session + ': ' + last.calls + ' calls \u00b7 ' +
        last.errors + ' errors \u00b7 p95 ' + last.p95_bytes + ' bytes \u00b7 ' +
        last.truncated + ' truncated \u00b7 ' + last.redundant + ' redundant' +
        (methods ? ' \u00b7 ' + methods : '')));
    }
    return card;
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
    var instrument = surfaceInstrument('the home', read.scalars.length + ' knobs',
      'every write rides the writer the CLI rides',
      [{
        label: 'open the router',
        run: function () { window.location.hash = '#/router'; }
      }]);
    el.settingsBoard.appendChild(surfaceHead('System', 'Settings',
      'The router, the providers, the lanes, the roles, the vault, the spend caps, the rules, ' +
      'the cron jobs, and the identity slot. Every value is picked from what this home already ' +
      'knows; every write rides the writer the CLI rides.', instrument));
    el.settingsBoard.appendChild(surfaceStrip([
      [read.scalars.length, 'knobs'],
      [read.lists.filter(function (e) { return e.kind === 'provider'; }).length,
        'provider overrides'],
      [read.lists.filter(function (e) { return e.kind === 'workload'; }).length,
        'workload overrides'],
      [read.rules.length, 'remembered rules', read.rules.length ? 'live' : null],
      [read.cron.length, 'cron jobs'],
      [read.cron.filter(function (job) { return job.enabled; }).length, 'cron enabled']
    ], read.read_at));
    el.settingsBoard.appendChild(sectionNav());

    el.settingsBoard.appendChild(routerCard());
    el.settingsBoard.appendChild(providersCard());
    el.settingsBoard.appendChild(lanesCard());
    el.settingsBoard.appendChild(serversCard());
    el.settingsBoard.appendChild(toolsCard());
    el.settingsBoard.appendChild(rolesCard());
    el.settingsBoard.appendChild(vaultCard());
    el.settingsBoard.appendChild(spendCard());

    // The generic knobs. The router's own fields live in the router card,
    // so they are filtered here rather than rendered twice.
    var routerKeys = {
      'pin': true,
      'gates.min_confidence': true,
      'gates.default_workload': true,
      'gates.allow_api_models': true,
      'FLEET_JUDGE_BUDGET_USD_DAY': true,
      'FLEET_DELEGATE_BUDGET_USD_DAY': true,
      'FLEET_EXPLAIN_BUDGET_USD_DAY': true
    };
    var knobs = boardCard(12, 'the other knobs', null,
      'the value in the box is what the home holds. A knob sourced from the environment says ' +
      'so \u2014 a file write would not take effect until the env var is unset.');
    knobs.id = 'settings-knobs';
    var groups = {};
    var order = [];
    (read.scalars || []).forEach(function (knob) {
      if (routerKeys[knob.key]) return;
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
      ui: 'interface',
      channels: 'channels',
      remote: 'remote'
    };
    order.forEach(function (prefix) {
      knobs.appendChild(node('h3', 'form-section',
        groupNames[prefix] || prefix.replace(/_/g, ' ')));
      groups[prefix].forEach(function (knob) { knobs.appendChild(knobRow(knob)); });
    });
    el.settingsBoard.appendChild(knobs);

    var ruleCard = boardCard(12, 'the remembered rules', null,
      'the third list, and the one that makes prompts thin out over time. The always-ask and ' +
      'destructive classes are refused by name \u2014 approval for those happens in the ' +
      'current turn, and a rule cannot waive it.');
    ruleCard.id = 'settings-rules';
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

    var cronCard = boardCard(12, 'the cron jobs', null,
      'the scheduled runs. A fire is an ordinary headless run in its own session log, and the ' +
      'task is the instruction it is given.');
    cronCard.id = 'settings-cron';
    if (read.cron.length) {
      read.cron.forEach(function (job) { cronCard.appendChild(cronRow(job)); });
    } else {
      cronCard.appendChild(node('p', 'board-empty',
        'No jobs are scheduled. The five-field form is minute hour day-of-month month day-of-week.'));
    }
    cronCard.appendChild(addCronForm());
    el.settingsBoard.appendChild(cronCard);

    var soulCard = boardCard(12, 'the identity slot', read.soul.present ? 'SOUL.md is set' : 'absent',
      'the file the model-visible base is read from per run. Absent, the compiled-in voice ' +
      'stands. Writing one replaces the base; the skills and memory frames still append.');
    soulCard.id = 'settings-soul';
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
    envCard.id = 'settings-env';
    (read.env || []).forEach(function (entry) {
      var row = node('div', 'list-row');
      row.appendChild(node('span', 'list-row-key', entry.name));
      row.appendChild(badge(entry.present ? 'resolves' : 'unset', entry.present ? 'ok' : 'warn'));
      envCard.appendChild(row);
    });
    el.settingsBoard.appendChild(envCard);
  }

  // The section chips. Ten surfaces under one page head would be a wall;
  // a chip is a jump, not a second navigation.
  function sectionNav() {
    var nav = node('nav', 'cfg-nav');
    [
      ['router', 'router'], ['providers', 'providers'], ['lanes', 'lanes'],
      ['roles', 'roles'], ['vault', 'vault'], ['spend', 'spend'],
      ['knobs', 'knobs'], ['rules', 'rules'], ['cron', 'cron'],
      ['soul', 'identity'], ['env', 'environment']
    ].forEach(function (pair) {
      var chip = node('button', 'cfg-chip', pair[1]);
      chip.type = 'button';
      chip.dataset.act = 'jump';
      chip.title = 'jump to ' + pair[1];
      chip.addEventListener('click', function () {
        var target = document.getElementById('settings-' + pair[0]);
        if (target) target.scrollIntoView({ behavior: 'smooth', block: 'start' });
      });
      nav.appendChild(chip);
    });
    return nav;
  }

  // ------------------------------------------------------------- pickers

  var cfgSeq = 0;

  function cfgOption(option) {
    return typeof option === 'string' ? { value: option, label: option } : option;
  }

  function cfgField(label, control, note) {
    var wrap = node('div', 'cfg-field');
    wrap.appendChild(node('span', 'cfg-label', label));
    wrap.appendChild(control);
    if (note) wrap.appendChild(node('span', 'cfg-note', note));
    return wrap;
  }

  // A closed set. The current value is selected; a value the options do
  // not know stays visible as `(custom)` rather than being dropped.
  function cfgSelect(value, options, aria) {
    var select = node('select', 'knob-box');
    select.setAttribute('aria-label', aria || '');
    var current = value == null ? '' : String(value);
    var normalized = (options || []).map(cfgOption);
    var seen = {};
    var found = false;
    // An unset value gets its own selected blank when the set has no
    // empty entry of its own: the lane's default stands until the owner
    // picks something, rather than the browser promoting the first
    // option into a written value.
    if (current === '' && !normalized.some(function (option) { return option.value === ''; })) {
      var blank = node('option', null, '\u2014');
      blank.value = '';
      blank.selected = true;
      select.appendChild(blank);
      seen[''] = true;
      found = true;
    }
    normalized.forEach(function (option) {
      if (seen[option.value]) return;
      seen[option.value] = true;
      var choice = node('option', null, option.label);
      choice.value = option.value;
      if (option.value === current) { choice.selected = true; found = true; }
      select.appendChild(choice);
    });
    if (!found && current !== '') {
      var custom = node('option', null, current + ' (custom)');
      custom.value = current;
      custom.selected = true;
      select.appendChild(custom);
    }
    return select;
  }

  // An open set: the catalog suggests, the input stays free. The caller
  // reads `wrap.input.value` at save.
  function cfgCombo(value, options, aria) {
    var id = 'cfg-list-' + (++cfgSeq);
    var input = node('input', 'knob-box');
    input.type = 'text';
    input.value = value == null ? '' : String(value);
    input.setAttribute('list', id);
    input.setAttribute('aria-label', aria || '');
    var list = node('datalist');
    list.id = id;
    var seen = {};
    (options || []).map(cfgOption).forEach(function (option) {
      if (seen[option.value]) return;
      seen[option.value] = true;
      var choice = node('option');
      choice.value = option.value;
      choice.label = option.label;
      list.appendChild(choice);
    });
    var wrap = node('span', 'cfg-combo');
    wrap.appendChild(input);
    wrap.appendChild(list);
    wrap.input = input;
    return wrap;
  }

  // An ordered or unordered list of chips. `onChange` receives the whole
  // next list; the add control is a pick plus an explicit custom entry.
  function cfgChips(values, options, ordered, onChange, aria) {
    var wrap = node('div', 'chip-editor');
    var list = node('div', 'chip-list');
    var state = (values || []).slice();
    var addRow = node('div', 'chip-add');
    var pick = cfgSelect('', options || [], (aria || 'add') + ' pick');
    if (pick.options.length && pick.options[0].value === '') {
      pick.options[0].text = 'pick from the list\u2026';
    }
    var add = node('button', 'replay-button', 'add');
    add.type = 'button';
    add.dataset.act = 'add';
    var custom = node('input', 'knob-box');
    custom.type = 'text';
    custom.placeholder = 'or type a new value';
    custom.setAttribute('aria-label', (aria || 'value') + ' custom');
    var customAdd = node('button', 'replay-button', 'add custom');
    customAdd.type = 'button';
    customAdd.dataset.act = 'add-custom';
    // A list with no catalog is a free-text list: the empty pick would be
    // a control that can never add anything.
    if (!options || !options.length) {
      pick.hidden = true;
      add.hidden = true;
    }

    function commit() {
      if (onChange) onChange(state.slice());
    }

    function render() {
      list.textContent = '';
      if (!state.length) {
        list.appendChild(node('span', 'cfg-note', 'nothing listed'));
      }
      state.forEach(function (value, index) {
        var chip = node('span', 'chip');
        chip.appendChild(node('span', 'chip-text', value));
        if (ordered) {
          var up = node('button', 'chip-act', '\u2191');
          up.type = 'button';
          up.dataset.act = 'move-up';
          up.disabled = index === 0;
          up.addEventListener('click', function () {
            var swap = state[index - 1];
            state[index - 1] = state[index];
            state[index] = swap;
            render();
            commit();
          });
          chip.appendChild(up);
          var down = node('button', 'chip-act', '\u2193');
          down.type = 'button';
          down.dataset.act = 'move-down';
          down.disabled = index === state.length - 1;
          down.addEventListener('click', function () {
            var swap = state[index + 1];
            state[index + 1] = state[index];
            state[index] = swap;
            render();
            commit();
          });
          chip.appendChild(down);
        }
        var remove = node('button', 'chip-act', '\u00d7');
        remove.type = 'button';
        remove.dataset.act = 'remove';
        remove.addEventListener('click', function () {
          state.splice(index, 1);
          render();
          commit();
        });
        chip.appendChild(remove);
        list.appendChild(chip);
      });
    }

    add.addEventListener('click', function () {
      var value = pick.value;
      if (!value || state.indexOf(value) >= 0) return;
      state.push(value);
      pick.value = '';
      render();
      commit();
    });
    customAdd.addEventListener('click', function () {
      var value = custom.value.trim();
      if (!value || state.indexOf(value) >= 0) return;
      state.push(value);
      custom.value = '';
      render();
      commit();
    });

    addRow.appendChild(pick);
    addRow.appendChild(add);
    addRow.appendChild(custom);
    addRow.appendChild(customAdd);
    wrap.appendChild(list);
    wrap.appendChild(addRow);
    wrap.values = function () { return state.slice(); };
    render();
    return wrap;
  }

  function cfgSwitch(label, checked, onChange, note) {
    var wrap = node('label', 'cfg-switch');
    var box = node('input');
    box.type = 'checkbox';
    box.checked = !!checked;
    box.addEventListener('change', function () { onChange(box.checked); });
    wrap.appendChild(box);
    wrap.appendChild(node('span', null, label));
    if (note) wrap.appendChild(node('span', 'cfg-note', note));
    return wrap;
  }

  // The catalog slices every picker reads.
  function cfgCatalog() { return (face.settings && face.settings.catalog) || {}; }
  function cfgModels() { return (cfgCatalog().models || []); }
  function cfgModelValues() { return cfgModels().map(function (one) { return one.value; }); }
  function cfgEnvNames() { return (cfgCatalog().env_names || []); }
  function cfgPaths() { return (cfgCatalog().paths || []); }
  function cfgWorkloads() { return (cfgCatalog().workloads || []); }
  function cfgProviders() { return (cfgCatalog().providers || []); }

  function cfgModelOptions() {
    return cfgModels().map(function (one) {
      return {
        value: one.value,
        label: one.value + (one.known ? (one.key_present || one.kind === 'lane' ? '' : ' \u00b7 no key') : ' \u00b7 unknown')
      };
    });
  }

  // ---------------------------------------------------------- the router

  function routerCard() {
    var read = face.settings.router || {};
    var card = boardCard(12, 'the router', read.orchestrator ? 'walks ' + read.orchestrator : null,
      'The spend gate, the workload tiers, and the judge\u2019s defaults. The resolved column is ' +
      'the first candidate that would serve now: the gate applied, the provider enabled, its ' +
      'key present. The orchestrator walks ' + (read.orchestrator || 'hard') +
      '\u2019s chain whatever the task was judged; calibration can still demote a candidate.');
    card.id = 'settings-router';
    if (read.refused && read.refused.length) {
      card.appendChild(node('p', 'board-empty', read.refused.join(' \u00b7 ')));
      return card;
    }

    card.appendChild(cfgSwitch('allow hosted API models', read.spend_guard, function (on) {
      writeSettings('set', { key: 'gates.allow_api_models', value: on ? 'true' : 'false' });
    }, 'Off keeps every HTTP candidate out of every order \u2014 even a pin. Vendored lanes still serve.'));

    var grid = node('div', 'tier-grid');
    ['tier', 'target', 'fallbacks (ordered)', 'resolved now'].forEach(function (head) {
      grid.appendChild(node('div', 'tier-head', head));
    });
    // The remove column has no label; the cell keeps the five-column
    // grid aligned with every row's five children.
    grid.appendChild(node('div', 'tier-head'));
    (read.tiers || []).forEach(function (tier) {
      grid.appendChild(tierRow(tier, read.orchestrator));
    });
    card.appendChild(grid);

    var add = node('div', 'cfg-add');
    var name = node('input', 'knob-box');
    name.type = 'text';
    name.placeholder = 'a new workload name';
    name.setAttribute('aria-label', 'a new workload name');
    var target = cfgSelect('', cfgModelOptions(), 'the new workload target');
    var addButton = node('button', 'replay-button replay-resume', 'add a workload');
    addButton.type = 'button';
    addButton.dataset.act = 'add-workload';
    addButton.addEventListener('click', function () {
      var workload = name.value.trim();
      if (!workload || !target.value) return;
      writeSettings('workload', { workload: workload, candidates: [target.value] });
    });
    add.appendChild(name);
    add.appendChild(target);
    add.appendChild(addButton);
    card.appendChild(add);

    var thresholds = node('div', 'cfg-grid');
    var defaultWorkload = cfgSelect(read.default_workload, cfgWorkloads(), 'the default workload');
    defaultWorkload.addEventListener('change', function () {
      writeSettings('set', { key: 'gates.default_workload', value: defaultWorkload.value });
    });
    thresholds.appendChild(cfgField('default workload', defaultWorkload,
      'an absent or below-floor judgment routes here.'));
    var floor = node('input', 'knob-box');
    floor.type = 'number';
    floor.step = '0.01';
    floor.min = '0';
    floor.max = '1';
    floor.value = String(read.min_confidence);
    floor.setAttribute('aria-label', 'the judgment confidence floor');
    var saveFloor = node('button', 'replay-button', 'save');
    saveFloor.type = 'button';
    saveFloor.addEventListener('click', function () {
      writeSettings('set', { key: 'gates.min_confidence', value: floor.value });
    });
    var floorWrap = node('span', 'cfg-inline');
    floorWrap.appendChild(floor);
    floorWrap.appendChild(saveFloor);
    thresholds.appendChild(cfgField('min confidence', floorWrap,
      'a verdict below this falls to the default workload.'));
    var pinOptions = [{ value: '', label: 'no pin' }]
      .concat(cfgProviders().map(function (provider) {
        return { value: provider.id, label: provider.id };
      }))
      .concat(cfgModelValues().map(function (value) {
        return { value: value, label: value };
      }));
    var pin = cfgSelect(read.pin || '', pinOptions, 'the pin');
    pin.addEventListener('change', function () {
      writeSettings('set', { key: 'pin', value: pin.value });
    });
    thresholds.appendChild(cfgField('pin', pin,
      'a provider pin still judges the workload; a provider/model pair skips the judgment.'));
    thresholds.appendChild(cfgField('orchestrator walks', node('span', 'cfg-readonly', read.orchestrator || 'hard'),
      'the first model supervises every run, whatever the task was judged.'));
    card.appendChild(thresholds);

    var roster = node('details', 'cfg-details');
    var summary = node('summary', 'cfg-summary',
      'the roster it resolves against (' + (read.roster || []).length + ' providers)');
    roster.appendChild(summary);
    (read.roster || []).forEach(function (entry) {
      var row = node('div', 'cfg-roster-row');
      row.appendChild(node('span', 'cfg-roster-id', entry.id));
      row.appendChild(badge(entry.kind, null));
      row.appendChild(badge(entry.billing, entry.billing === 'payg' ? 'warn' : null));
      row.appendChild(badge(entry.enabled ? 'enabled' : 'disabled', entry.enabled ? 'ok' : 'warn'));
      row.appendChild(badge(entry.key_present ? 'key present' : (entry.key_required ? 'no key' : 'no key needed'),
        entry.key_present || !entry.key_required ? 'ok' : 'warn'));
      row.appendChild(node('span', 'cfg-note', entry.key_env));
      roster.appendChild(row);
    });
    card.appendChild(roster);
    return card;
  }

  function tierRow(tier, orchestrator) {
    var target = cfgSelect(tier.candidates[0] || '', cfgModelOptions(), tier.workload + ' target');
    target.addEventListener('change', function () {
      // A target promoted from the fallbacks leaves the fallback list:
      // one candidate, one position.
      saveTier(tier.workload, [target.value].concat(tier.candidates.slice(1)));
    });
    var fallbacks = cfgChips(tier.candidates.slice(1), cfgModelValues(), true, function (next) {
      saveTier(tier.workload, [tier.candidates[0]].concat(next));
    }, tier.workload + ' fallbacks');
    var resolved = node('div', 'tier-resolved');
    if (tier.resolved) {
      resolved.appendChild(badge(tier.resolved, 'ok'));
    } else {
      resolved.appendChild(badge('nothing resolves', 'warn'));
      if (tier.reason) resolved.appendChild(node('span', 'cfg-note', tier.reason));
    }
    if (tier.workload === orchestrator) {
      resolved.appendChild(badge('walker', 'live'));
    }
    var remove = node('button', 'chip-act', '\u00d7');
    remove.type = 'button';
    remove.dataset.act = 'remove-workload';
    remove.title = tier.reverts ? 'remove the override, reverting to the compiled-in chain'
      : 'remove a workload the file added';
    remove.addEventListener('click', function () {
      if (!tier.reverts && !window.confirm(
        '`' + tier.workload + '` was added by this home. Removing it takes the workload out ' +
        'of every walk that names it. Do it anyway?')) return;
      writeSettings('workload-remove', { workload: tier.workload, force: !tier.reverts });
    });
    var row = node('div', 'tier-row');
    row.appendChild(node('div', 'tier-name', tier.workload));
    row.appendChild(target);
    row.appendChild(fallbacks);
    row.appendChild(resolved);
    row.appendChild(remove);
    return row;
  }

  function saveTier(workload, candidates) {
    var cleaned = [];
    candidates.forEach(function (value) {
      if (!value || !value.trim() || cleaned.indexOf(value) >= 0) return;
      cleaned.push(value);
    });
    if (!cleaned.length) return;
    writeSettings('workload', { workload: workload, candidates: cleaned });
  }

  // --------------------------------------------------------- the providers

  function providersCard() {
    var card = boardCard(12, 'the providers', null,
      'The roster the router walks. Each field is seeded from the roster; the key env is a ' +
      'name, never a value, and a provider with no override is the compiled-in entry.');
    card.id = 'settings-providers';
    var roster = (face.settings.router && face.settings.router.roster) || [];
    var overrides = {};
    (face.settings.lists || []).forEach(function (entry) {
      if (entry.kind === 'provider') overrides[entry.key] = entry;
    });
    roster.forEach(function (entry) {
      card.appendChild(providerEditor(entry, overrides[entry.id]));
    });
    card.appendChild(providerAddForm());
    return card;
  }

  function providerEditor(entry, override) {
    var form = node('div', 'cfg-form');
    var head = node('div', 'cfg-form-head');
    head.appendChild(node('span', 'list-row-key', entry.id));
    head.appendChild(badge(entry.kind, null));
    head.appendChild(badge(entry.billing, entry.billing === 'payg' ? 'warn' : null));
    head.appendChild(badge(entry.enabled ? 'enabled' : 'disabled', entry.enabled ? 'ok' : 'warn'));
    head.appendChild(badge(entry.key_present ? 'key present' : (entry.key_required ? 'no key' : 'no key needed'),
      entry.key_present || !entry.key_required ? 'ok' : 'warn'));
    form.appendChild(head);

    var grid = node('div', 'cfg-grid');
    var base = cfgCombo(entry.base_url, [], entry.id + ' base url');
    grid.appendChild(cfgField('base url', base));
    var keyEnv = cfgCombo(entry.key_env, cfgEnvNames().map(function (name) {
      return { value: name.name, label: name.name + (name.present ? ' \u00b7 ' + name.source : ' \u00b7 unset') };
    }), entry.id + ' key env');
    grid.appendChild(cfgField('key env', keyEnv, 'a name; the value never rides this page.'));
    var models = cfgChips(entry.models || [], [], false, null, entry.id + ' models');
    grid.appendChild(cfgField('models', models, 'the model ids this provider serves.'));
    var billing = cfgSelect(entry.billing, ['plan', 'payg'], entry.id + ' billing');
    grid.appendChild(cfgField('billing', billing));
    var kind = cfgSelect(entry.kind, ['http', 'lane'], entry.id + ' kind');
    grid.appendChild(cfgField('kind', kind));
    form.appendChild(grid);

    var switches = node('div', 'cfg-inline');
    var enabled = cfgSwitch('enabled', entry.enabled, function () { }, null);
    var keyRequired = cfgSwitch('key required', entry.key_required, function () { }, null);
    switches.appendChild(enabled);
    switches.appendChild(keyRequired);
    form.appendChild(switches);
    var reason = cfgCombo(entry.disabled_reason || '', [], entry.id + ' disabled reason');
    form.appendChild(cfgField('disabled reason', reason, 'shown when the provider is off.'));

    var actions = node('div', 'replay-actions');
    var save = node('button', 'replay-button replay-resume', 'save');
    save.type = 'button';
    save.dataset.act = 'save-provider';
    save.addEventListener('click', function () {
      var payload = {
        id: entry.id,
        base_url: base.input.value.trim(),
        key_env: keyEnv.input.value.trim(),
        models: models.values(),
        billing: billing.value,
        kind: kind.value,
        enabled: enabled.querySelector('input').checked,
        key_required: keyRequired.querySelector('input').checked
      };
      var said = reason.input.value.trim();
      if (said) payload.disabled_reason = said;
      writeSettings('provider', { entry: JSON.stringify(payload) });
    });
    actions.appendChild(save);
    var pin = node('button', 'replay-button', 'pin this provider');
    pin.type = 'button';
    pin.addEventListener('click', function () {
      writeSettings('set', { key: 'pin', value: entry.id });
    });
    actions.appendChild(pin);
    if (override) {
      var remove = node('button', 'replay-button',
        override.reverts ? 'remove the override' : 'remove \u2014 takes it out of every walk');
      remove.type = 'button';
      remove.addEventListener('click', function () {
        if (!override.reverts && !window.confirm(
          '`' + entry.id + '` was added by this home. Removing it takes the provider out of ' +
          'every walk that names it. Do it anyway?')) return;
        writeSettings('provider-remove', { id: entry.id, force: !override.reverts });
      });
      actions.appendChild(remove);
    }
    form.appendChild(actions);
    return form;
  }

  function providerAddForm() {
    var form = node('div', 'cfg-form');
    var head = node('div', 'cfg-form-head');
    head.appendChild(node('span', 'list-row-key', 'add a provider'));
    form.appendChild(head);
    var grid = node('div', 'cfg-grid');
    var id = node('input', 'knob-box'); id.type = 'text'; id.placeholder = 'my-provider';
    grid.appendChild(cfgField('id', id));
    var base = cfgCombo('', [], 'a new provider base url');
    grid.appendChild(cfgField('base url', base));
    var key = cfgCombo('', cfgEnvNames().map(function (name) {
      return { value: name.name, label: name.name };
    }), 'a new provider key env');
    grid.appendChild(cfgField('key env', key));
    var model = node('input', 'knob-box'); model.type = 'text'; model.placeholder = 'model-id';
    grid.appendChild(cfgField('models', model, 'comma-separated.'));
    var billing = cfgSelect('plan', ['plan', 'payg'], 'a new provider billing');
    grid.appendChild(cfgField('billing', billing));
    var kind = cfgSelect('http', ['http', 'lane'], 'a new provider kind');
    grid.appendChild(cfgField('kind', kind));
    form.appendChild(grid);
    var add = node('button', 'replay-button replay-resume', 'add');
    add.type = 'button';
    add.addEventListener('click', function () {
      var models = model.value.split(',').map(function (one) { return one.trim(); })
        .filter(function (one) { return one.length > 0; });
      if (!id.value.trim() || !base.input.value.trim() || !key.input.value.trim() || !models.length) return;
      writeSettings('provider', {
        entry: JSON.stringify({
          id: id.value.trim(),
          base_url: base.input.value.trim(),
          key_env: key.input.value.trim(),
          models: models,
          billing: billing.value,
          kind: kind.value
        })
      });
    });
    form.appendChild(add);
    return form;
  }

  // ------------------------------------------------------- the MCP servers

  // One settings write over the registry's own ops, with the reply's
  // document — a scan's rows or a probe's outcome — handed to the
  // caller before the panel repaints.
  function serversAction(op, fields, onDone) {
    var body = Object.assign({ op: op }, fields || {});
    post('/api/settings', body).then(function (reply) {
      if (reply.status !== 200) {
        notice('the registry said no',
          (reply.body && reply.body.error) || 'the surface said no without a reason');
        return;
      }
      if (reply.body && reply.body.scan) face.serversScan = reply.body.scan;
      return readSettings(true).then(function () { if (onDone) onDone(reply.body); });
    }).catch(function () { });
  }

  function serversCard() {
    var read = (face.settings && face.settings.servers) || {};
    var card = boardCard(12, 'the MCP servers', null,
      'The servers the owner configured beside the vendored lanes, carried by the home\u2019s ' +
      'own mcp.json. An enabled server\u2019s tools are declared to every run from its first ' +
      'request. The registry carries env NAMES only \u2014 a value has no field to live in; ' +
      'each name resolves from the environment at spawn.');
    card.id = 'settings-servers';
    if (read.refused && read.refused.length) {
      card.appendChild(node('p', 'board-empty', read.refused.join(' \u00b7 ')));
      return card;
    }
    var servers = read.servers || [];
    var actions = node('div', 'bots-acts');
    actions.appendChild(serversButton('server-scan', 'scan this machine', function () {
      serversAction('server-scan', {}, function (body) {
        var rows = (body.scan && body.scan.rows) || [];
        var skips = (body.scan && body.scan.skips) || [];
        notice('the scan finished',
          rows.length + ' entries across the other harnesses' +
            (skips.length ? ' \u00b7 ' + skips.length + ' file(s) skipped' : ''));
      });
    }));
    card.appendChild(actions);
    if (!servers.length) {
      card.appendChild(node('p', 'board-empty',
        'No MCP servers configured yet \u2014 run a scan to discover the machine\u2019s other ' +
        'harnesses, or write one by hand.'));
    }
    servers.forEach(function (server) {
      card.appendChild(serverRow(server));
    });
    // The discovery half: the last scan's answer, rows the registry does
    // not carry yet, one import action each. A reload drops it.
    var scan = face.serversScan;
    if (scan) {
      var fresh = (scan.rows || []).filter(function (row) {
        return row.status === 'available';
      });
      var unsupported = (scan.rows || []).filter(function (row) {
        return row.status === 'unsupported';
      });
      var carried = (scan.rows || []).filter(function (row) {
        return row.status === 'configured';
      });
      if (fresh.length) {
        card.appendChild(node('p', 'server-scan-head',
          'discovered \u00b7 ' + fresh.length + ' available to import \u00b7 from the last scan'));
        fresh.forEach(function (row) {
          card.appendChild(scanRow(row));
        });
      }
      // An unsupported row still shows, with its import disabled and its
      // reason as the launch line: the scan reported it, so the card
      // does not make it disappear.
      unsupported.forEach(function (row) {
        card.appendChild(scanRow(row));
      });
      if (carried.length) {
        card.appendChild(node('p', 'server-scan-head',
          carried.length + ' already configured \u00b7 their rows are above'));
      }
      (scan.skips || []).forEach(function (skip) {
        card.appendChild(node('p', 'board-empty', 'skipped \u00b7 ' + skip));
      });
    }
    return card;
  }

  function serversButton(act, label, onClick, disabled) {
    var made = node('button', 'bots-act', label);
    made.type = 'button';
    made.dataset.act = act;
    if (disabled) made.disabled = true;
    made.addEventListener('click', onClick);
    return made;
  }

  // One configured server's row: the status dot (probed ok, never
  // probed, last probe failed), the name, the env names, the launch
  // line as the display redaction shows it, the source badge when the
  // server was imported, and the probe line.
  function serverRow(server) {
    var row = node('div', 'server-row');
    var probe = server.last_probe;
    var state = probe ? (probe.ok ? 'ok' : 'failed') : 'never';
    var dot = node('span', 'server-dot');
    dot.dataset.state = state;
    dot.title = probe
      ? (probe.ok
        ? 'probed ok \u00b7 ' + probe.at
        : 'last probe failed \u00b7 ' + probe.reason)
      : 'never probed';
    row.appendChild(dot);
    var main = node('div', 'server-main');
    var nameLine = node('div', 'server-name-line');
    nameLine.appendChild(node('b', null, server.name));
    if (server.source) {
      nameLine.appendChild(node('span', 'server-source',
        'imported from ' + server.source.harness + ', ' + server.source.imported));
    }
    nameLine.appendChild(node('span', 'server-enabled',
      server.enabled ? 'enabled' : 'disabled'));
    main.appendChild(nameLine);
    main.appendChild(node('span', 'server-launch',
      [server.program].concat(server.args || []).join(' ')));
    if ((server.env_names || []).length) {
      main.appendChild(node('span', 'server-env',
        'env ' + server.env_names.join(', ') + ' \u00b7 resolves at spawn'));
    }
    if (probe && probe.ok) {
      main.appendChild(node('span', 'server-probe-line',
        probe.server_name + ' v' + probe.server_version + ' \u00b7 ' + probe.tools +
        (probe.tools === 1 ? ' tool' : ' tools') + ' \u00b7 probed ' + probe.at));
    }
    row.appendChild(main);
    var acts = node('div', 'bots-acts');
    acts.appendChild(serversButton('server-toggle', server.enabled ? 'disable' : 'enable',
      function () {
        serversAction('server', { name: server.name, enabled: !server.enabled });
      }));
    acts.appendChild(serversButton('server-probe', 'probe', function () {
      serversAction('server-probe', { name: server.name }, function (body) {
        notice(body.probe.ok ? 'the probe came back ok' : 'the probe failed',
          body.probe.ok
            ? body.probe.server_name + ' v' + body.probe.server_version + ' \u00b7 ' +
              body.probe.tools + (body.probe.tools === 1 ? ' tool' : ' tools')
            : body.probe.reason);
      });
    }));
    acts.appendChild(serversButton('server-remove', '\u00d7', function () {
      serversAction('server-remove', { name: server.name });
    }));
    row.appendChild(acts);
    return row;
  }

  // One discovered entry: name, harness, badge, the redacted launch
  // line, and the import \u2014 disabled by shape with its reason when the
  // entry is not stdio.
  function scanRow(row) {
    var line = node('div', 'server-row scan');
    var main = node('div', 'server-main');
    var nameLine = node('div', 'server-name-line');
    nameLine.appendChild(node('b', null, row.name));
    nameLine.appendChild(node('span', 'server-source', row.harness));
    nameLine.appendChild(node('span', 'server-badge', row.badge));
    main.appendChild(nameLine);
    main.appendChild(node('span', 'server-launch', row.launch));
    if ((row.env_names || []).length) {
      main.appendChild(node('span', 'server-env', 'env ' + row.env_names.join(', ')));
    }
    line.appendChild(main);
    var acts = node('div', 'bots-acts');
    var supported = row.status === 'available';
    acts.appendChild(serversButton('server-import', 'import', function () {
      serversAction('server-import', { name: row.name, harness: row.harness }, function () {
        notice('imported', row.name + ' \u00b7 from ' + row.harness);
      });
    }, !supported));
    if (!supported) {
      line.title = row.launch;
    }
    line.appendChild(acts);
    return line;
  }

  // ------------------------------------------------------------- the lanes

  function lanesCard() {
    var read = face.settings.lanes || {};
    var card = boardCard(12, 'the lanes', null,
      'The services fleet launches: the judgment, tabular, embeddings, and media lanes beside ' +
      'the browser and the composer. A value here is the launch environment the lane receives; ' +
      'the lane\u2019s own repository and TOML are never rewritten. Key presence is a badge, ' +
      'never a value.');
    card.id = 'settings-lanes';
    if (read.refused && read.refused.length) {
      card.appendChild(node('p', 'board-empty', read.refused.join(' \u00b7 ')));
      return card;
    }
    (read.lanes || []).forEach(function (lane) {
      card.appendChild(laneBlock(lane));
    });
    return card;
  }

  function laneByKind(lane, kind) {
    return (lane.choices || []).filter(function (choice) { return choice.kind === kind; });
  }

  function laneProviders(lane) {
    var seen = {};
    var out = [];
    laneByKind(lane, 'provider').forEach(function (choice) {
      if (seen[choice.provider]) return;
      seen[choice.provider] = true;
      out.push(choice.provider);
    });
    return out;
  }

  function laneOptions(lane, kind, provider) {
    return laneByKind(lane, kind).filter(function (choice) {
      return !provider || choice.provider === provider;
    }).map(function (choice) {
      return { value: choice.value, label: choice.label };
    });
  }

  function envKeyOptions() {
    return cfgEnvNames().map(function (name) {
      return { value: name.name, label: name.name + (name.present ? ' \u00b7 ' + name.source : ' \u00b7 unset') };
    });
  }

  function sys1EnvFields(lane) {
    var fields = [
      { name: 'SYS1_CLASSIFY__PROVIDER', label: 'classify provider', kind: 'select',
        options: laneOptions(lane, 'classify') },
      { name: 'SYS1_CLASSIFY__CONFIDENCE_FLOOR', label: 'confidence floor', kind: 'text' },
      { name: 'SYS1_CLASSIFY__BLOCK_ON_CLASSIFICATION', label: 'block on classification', kind: 'toggle' },
      { name: 'SYS1_CLASSIFY__ESCALATE_ON_NULL', label: 'escalate on null', kind: 'toggle' },
      { name: 'SYS1_PROVIDERS__ROUTE__ENABLED', label: 'route enabled', kind: 'toggle' },
      { name: 'SYS1_PROVIDERS__ROUTE__DEFAULT_CHAIN', label: 'default chain', kind: 'chips',
        options: laneProviders(lane) },
      { name: 'SYS1_PROVIDERS__ENABLED', label: 'pool enabled', kind: 'text' },
      { name: 'SYS1_PROVIDERS__DISABLED', label: 'pool disabled', kind: 'text' }
    ];
    return fields;
  }

  function laneEnvFields(lane) {
    if (lane.name === 'sys1') return sys1EnvFields(lane);
    if (lane.name === 'sem1') {
      return [
        { name: 'SEM1_ST_VENV', label: 'st venv', kind: 'combo', options: cfgPaths().map(function (path) { return path.value; }) },
        { name: 'SEM1_VECTORS_DIR', label: 'vectors dir', kind: 'combo' },
        { name: 'SEM1_LOG_DIR', label: 'log dir', kind: 'combo' }
      ];
    }
    if (lane.name === 'sdm1') {
      return [
        { name: 'SDM1_TABPFN_BASE', label: 'TabPFN base', kind: 'combo', options: laneOptions(lane, 'endpoint') },
        { name: 'SDM1_TABPFN_TIMEOUT', label: 'TabPFN timeout', kind: 'text' },
        { name: 'SDM1_MODELS_PYTHON', label: 'models python', kind: 'combo', options: cfgPaths().map(function (path) { return path.value; }) },
        { name: 'SDM1_MODELS_VENV', label: 'models venv', kind: 'combo', options: cfgPaths().map(function (path) { return path.value; }) },
        { name: 'SDM1_TIMEOUT', label: 'worker timeout', kind: 'text' },
        { name: 'SDM1_LOG_DIR', label: 'log dir', kind: 'combo' }
      ];
    }
    if (lane.name === 'gen1') {
      return [
        { name: 'QWEN_BASE_URL', label: 'qwen base url', kind: 'combo', options: laneOptions(lane, 'endpoint', 'qwen') },
        { name: 'QWEN_TTS_WS_URL', label: 'qwen tts ws url', kind: 'combo' },
        { name: 'QWEN_TTS_MODEL', label: 'qwen tts model', kind: 'combo', options: laneOptions(lane, 'model', 'qwen') },
        { name: 'QWEN_TTS_VOICE', label: 'qwen voice', kind: 'combo', options: laneOptions(lane, 'voice', 'qwen') },
        { name: 'STEPFUN_BASE_URL', label: 'stepfun base url', kind: 'combo', options: laneOptions(lane, 'endpoint', 'stepfun') },
        { name: 'STEPFUN_TTS_MODEL', label: 'stepfun tts model', kind: 'combo', options: laneOptions(lane, 'model', 'stepfun') },
        { name: 'STEPFUN_TTS_VOICE', label: 'stepfun voice', kind: 'combo', options: laneOptions(lane, 'voice', 'stepfun') },
        { name: 'STEPFUN_ASR_MODEL', label: 'stepfun asr model', kind: 'combo', options: laneOptions(lane, 'model', 'stepfun') },
        { name: 'DASHSCOPE_BASE_URL', label: 'dashscope base url', kind: 'combo', options: laneOptions(lane, 'endpoint', 'wan') },
        { name: 'IMAGE_MODEL', label: 'wan image model', kind: 'combo', options: laneOptions(lane, 'model', 'wan') },
        { name: 'GEN1_KOKORO_MODEL', label: 'kokoro model', kind: 'combo', options: cfgPaths().map(function (path) { return path.value; }) },
        { name: 'GEN1_KOKORO_VOICES', label: 'kokoro voices', kind: 'combo', options: cfgPaths().map(function (path) { return path.value; }) },
        { name: 'GEN1_KOKORO_VOICE', label: 'kokoro voice', kind: 'combo', options: laneOptions(lane, 'voice', 'kokoro') },
        { name: 'GEN1_KOKORO_PYTHON', label: 'kokoro python', kind: 'combo' },
        { name: 'GEN1_TELEMETRY_FILE', label: 'telemetry file', kind: 'combo' }
      ];
    }
    return [];
  }

  function laneDefaultFields(lane) {
    if (lane.name === 'sem1') {
      return [
        { tool: 'sem1_embed', arg: 'provider', label: 'embed provider', kind: 'select', options: laneOptions(lane, 'provider') },
        { tool: 'sem1_embed', arg: 'model', label: 'embed model', kind: 'combo', options: laneOptions(lane, 'model') },
        { tool: 'sem1_embed', arg: 'endpoint', label: 'embed endpoint', kind: 'combo', options: laneOptions(lane, 'endpoint') }
      ];
    }
    if (lane.name === 'sdm1') {
      return [
        { tool: 'sdm1_classify', arg: 'model', label: 'classify provider', kind: 'select',
          options: laneOptions(lane, 'provider') }
      ];
    }
    if (lane.name === 'gen1') {
      return [
        { tool: 'gen1_imagine', arg: 'provider', label: 'imagine provider', kind: 'select',
          options: laneOptions(lane, 'provider').filter(function (choice) {
            return choice.value === 'wan';
          }) }
      ];
    }
    return [];
  }

  // The sys1 provider pool: three fields per provider, seeded from the
  // lane catalog. It lives behind its own summary because twelve
  // providers are a table, not a paragraph.
  function sys1Pool(lane, configured, controls) {
    var details = node('details', 'cfg-details');
    details.appendChild(node('summary', 'cfg-summary', 'the provider pool (' + laneProviders(lane).length + ')'));
    var grid = node('div', 'pool-grid');
    ['provider', 'model', 'base url', 'key env'].forEach(function (head) {
      grid.appendChild(node('div', 'tier-head', head));
    });
    laneProviders(lane).forEach(function (provider) {
      var upper = provider.toUpperCase().replace(/[^A-Z0-9]/g, '_');
      var modelName = 'SYS1_PROVIDERS__' + upper + '_MODEL';
      var urlName = provider === 'local'
        ? 'SYS1_PROVIDERS__LOCAL_SERVER_URL'
        : 'SYS1_PROVIDERS__' + upper + '_API_URL';
      var keyName = provider === 'local'
        ? 'SYS1_PROVIDERS__LOCAL_VENV'
        : 'SYS1_PROVIDERS__' + upper + '_API_KEY_ENV';
      var model = cfgCombo(configured[modelName] || '', laneOptions(lane, 'model', provider), provider + ' model');
      var url = cfgCombo(configured[urlName] || '', laneOptions(lane, 'endpoint', provider), provider + ' url');
      var key = cfgCombo(configured[keyName] || '',
        provider === 'local' ? cfgPaths().map(function (path) { return path.value; })
          : envKeyOptions().concat(laneOptions(lane, 'key_env', provider)),
        provider + ' key env');
      controls.push({ name: modelName, control: model, kind: 'combo', tool: null });
      controls.push({ name: urlName, control: url, kind: 'combo', tool: null });
      controls.push({ name: keyName, control: key, kind: 'combo', tool: null });
      grid.appendChild(node('div', 'pool-name', provider));
      grid.appendChild(model);
      grid.appendChild(url);
      grid.appendChild(key);
    });
    details.appendChild(grid);
    return details;
  }

  function laneBlock(lane) {
    var configured = lane.configured || {};
    var env = configured.env || {};
    var defaults = configured.defaults || {};
    var details = node('details', 'cfg-details');
    var summary = node('summary', 'cfg-summary');
    summary.appendChild(node('span', 'cfg-lane-name', lane.name));
    summary.appendChild(badge(lane.role, null));
    if (lane.unavailable) {
      summary.appendChild(badge('unavailable', 'warn'));
    } else {
      summary.appendChild(badge('home \u00b7 ' + (lane.source || 'bundled'), 'ok'));
    }
    details.appendChild(summary);
    if (lane.unavailable) {
      details.appendChild(node('p', 'cfg-note', lane.unavailable));
    }

    var controls = [];
    var home = cfgCombo(configured.home || '', cfgPaths().map(function (path) {
      return { value: path.value, label: path.value + ' \u00b7 ' + path.label };
    }), lane.name + ' home');
    details.appendChild(cfgField('home', home,
      'an absolute checkout; empty resolves the bundled copy.'));

    var envFields = laneEnvFields(lane);
    envFields.forEach(function (field) {
      var value = env[field.name] || '';
      var control;
      if (field.kind === 'select') {
        control = cfgSelect(value, field.options || [], field.label);
      } else if (field.kind === 'toggle') {
        control = cfgSwitch(field.label, value === 'true' || value === '1', function () { }, null);
      } else if (field.kind === 'chips') {
        control = cfgChips(value ? value.split(',').map(function (one) { return one.trim(); })
          .filter(function (one) { return one.length > 0; }) : [],
          field.options || [], true, null, field.label);
      } else {
        control = cfgCombo(value, field.options || [], field.label);
      }
      controls.push({ name: field.name, control: control, kind: field.kind });
      details.appendChild(cfgField(field.label, control));
    });

    if (lane.name === 'sys1') {
      details.appendChild(sys1Pool(lane, env, controls));
    }

    var defaultFields = laneDefaultFields(lane);
    if (defaultFields.length) {
      details.appendChild(node('h3', 'form-section', 'per-call defaults'));
      details.appendChild(node('p', 'cfg-note',
        'the model\u2019s own argument always wins; a default only fills what it left out.'));
      defaultFields.forEach(function (field) {
        var current = (defaults[field.tool] || {})[field.arg] || '';
        var control = field.kind === 'select'
          ? cfgSelect(current, field.options || [], field.label)
          : cfgCombo(current, field.options || [], field.label);
        controls.push({ tool: field.tool, arg: field.arg, control: control, kind: field.kind });
        details.appendChild(cfgField(field.label, control));
      });
    }

    var actions = node('div', 'replay-actions');
    var save = node('button', 'replay-button replay-resume', 'save the lane');
    save.type = 'button';
    save.dataset.act = 'save-lane';
    save.addEventListener('click', function () {
      var draft = { home: home.input.value.trim(), env: {}, defaults: {} };
      controls.forEach(function (field) {
        var value;
        if (field.kind === 'toggle') {
          value = field.control.querySelector('input').checked ? 'true' : 'false';
        } else if (field.kind === 'chips') {
          value = field.control.values().join(',');
        } else if (field.kind === 'combo') {
          value = field.control.input.value.trim();
        } else {
          value = field.control.value.trim();
        }
        if (!value) return;
        if (field.tool) {
          if (!draft.defaults[field.tool]) draft.defaults[field.tool] = {};
          draft.defaults[field.tool][field.arg] = value;
        } else {
          draft.env[field.name] = value;
        }
      });
      writeSettings('lane', { lane: lane.name, config: draft });
    });
    actions.appendChild(save);
    var reset = node('button', 'replay-button', 'reset this lane');
    reset.type = 'button';
    reset.addEventListener('click', function () {
      if (!window.confirm('Clear every configured value for `' + lane.name + '`?')) return;
      writeSettings('lane', { lane: lane.name, config: { env: {}, defaults: {} } });
    });
    actions.appendChild(reset);
    details.appendChild(actions);
    return details;
  }

  // ------------------------------------------------------------- the roles

  function rolesCard() {
    var read = face.settings.roles || {};
    var card = boardCard(12, 'the roles', null,
      'The named specializations a delegation runs under. A role is a markdown file in the ' +
      'home; the tools, skills, and shapes are picked from the vocabularies Fleet already ' +
      'knows, and the overlay is the standing instruction the child carries.');
    card.id = 'settings-roles';
    (read.entries || []).forEach(function (entry) {
      card.appendChild(roleBlock(entry));
    });
    card.appendChild(roleAddForm());
    return card;
  }

  function roleBlock(entry) {
    var details = node('details', 'cfg-details');
    var summary = node('summary', 'cfg-summary');
    summary.appendChild(node('span', 'cfg-lane-name', entry.name));
    if (entry.broken) {
      summary.appendChild(badge('broken', 'down'));
    } else {
      summary.appendChild(node('span', 'cfg-note', entry.description));
      if (entry.modified) summary.appendChild(badge(entry.modified, null));
    }
    details.appendChild(summary);
    if (entry.broken) {
      details.appendChild(node('p', 'board-empty', entry.broken));
    }
    var description = node('input', 'knob-box');
    description.type = 'text';
    description.value = entry.description || '';
    description.setAttribute('aria-label', entry.name + ' description');
    details.appendChild(cfgField('description', description));
    var tools = cfgChips(entry.tools || [], cfgCatalog().tools || [], false, null, entry.name + ' tools');
    details.appendChild(cfgField('tools', tools, 'empty narrows nothing: the brief\'s own set stands.'));
    var skills = cfgChips(entry.skills || [], cfgCatalog().skills || [], false, null, entry.name + ' skills');
    details.appendChild(cfgField('skills', skills));
    var shapes = cfgChips(entry.shapes || [], cfgCatalog().shapes || [], false, null, entry.name + ' shapes');
    details.appendChild(cfgField('shapes', shapes));
    var overlay = node('textarea', 'soul-box');
    overlay.value = entry.overlay || '';
    overlay.setAttribute('aria-label', entry.name + ' overlay');
    details.appendChild(cfgField('overlay', overlay));
    var actions = node('div', 'replay-actions');
    var save = node('button', 'replay-button replay-resume', 'save the role');
    save.type = 'button';
    save.dataset.act = 'save-role';
    save.addEventListener('click', function () {
      if (!description.value.trim()) {
        notice('the role was not written', 'a role needs a description');
        return;
      }
      writeSettings('role', {
        name: entry.name,
        description: description.value.trim(),
        tools: tools.values(),
        skills: skills.values(),
        shapes: shapes.values(),
        overlay: overlay.value
      });
    });
    actions.appendChild(save);
    var remove = node('button', 'replay-button', 'remove the role');
    remove.type = 'button';
    remove.addEventListener('click', function () {
      if (!window.confirm('Remove the role `' + entry.name + '`?')) return;
      writeSettings('role-remove', { name: entry.name });
    });
    actions.appendChild(remove);
    details.appendChild(actions);
    return details;
  }

  function roleAddForm() {
    var details = node('details', 'cfg-details');
    details.appendChild(node('summary', 'cfg-summary', 'add a role'));
    var name = node('input', 'knob-box');
    name.type = 'text';
    name.placeholder = 'auditor';
    name.setAttribute('aria-label', 'a new role name');
    details.appendChild(cfgField('name', name, 'a name, not a path; the file is <name>.md.'));
    var description = node('input', 'knob-box');
    description.type = 'text';
    description.placeholder = 'what this role is for';
    description.setAttribute('aria-label', 'a new role description');
    details.appendChild(cfgField('description', description));
    var tools = cfgChips([], cfgCatalog().tools || [], false, null, 'the new role tools');
    details.appendChild(cfgField('tools', tools));
    var skills = cfgChips([], cfgCatalog().skills || [], false, null, 'the new role skills');
    details.appendChild(cfgField('skills', skills));
    var shapes = cfgChips([], cfgCatalog().shapes || [], false, null, 'the new role shapes');
    details.appendChild(cfgField('shapes', shapes));
    var overlay = node('textarea', 'soul-box');
    overlay.setAttribute('aria-label', 'a new role overlay');
    details.appendChild(cfgField('overlay', overlay));
    var add = node('button', 'replay-button replay-resume', 'create the role');
    add.type = 'button';
    add.addEventListener('click', function () {
      if (!name.value.trim() || !description.value.trim()) {
        notice('the role was not written', 'a name and a description are required');
        return;
      }
      writeSettings('role', {
        name: name.value.trim(),
        description: description.value.trim(),
        tools: tools.values(),
        skills: skills.values(),
        shapes: shapes.values(),
        overlay: overlay.value
      });
    });
    details.appendChild(add);
    return details;
  }

  // ------------------------------------------------------------- the vault

  // The vault's own surface. The Settings page keeps its section card;
  // this is the first-class destination, so onboarding is one click from
  // the rail rather than a scroll through the settings page.
  function paintVault() {
    var read = face.settings;
    el.vaultBoard.textContent = '';
    if (!read) {
      el.vaultBoard.appendChild(surfaceHead('System', 'Vault',
        'The credentials Fleet seals, and the passphrase that keys them.', null));
      var waiting = boardCard(12, 'reading', null, null);
      waiting.appendChild(node('p', 'board-empty',
        'The vault read has not landed yet. It reads this home\u2019s vault and its .env.'));
      el.vaultBoard.appendChild(waiting);
      return;
    }
    var vault = read.vault || {};
    var instrument = surfaceInstrument('the vault',
      vault.exists ? (vault.names.length + ' sealed') : 'not set up',
      'a name is listed; a value is never rendered',
      null);
    el.vaultBoard.appendChild(surfaceHead('System', 'Vault',
      'The credentials Fleet seals, and the passphrase that keys them. A value is never ' +
      'rendered; onboarding creates the vault and migrates the plaintext .env entries.',
      instrument));
    el.vaultBoard.appendChild(vaultCard('vault-card'));
  }

  function vaultPassword(aria) {
    var input = node('input', 'knob-box');
    input.type = 'password';
    input.autocomplete = 'off';
    input.setAttribute('aria-label', aria);
    return input;
  }

  function vaultCard(cardId) {
    var read = face.settings.vault || {
      names: [], exists: false, passphrase: false, plaintext: []
    };
    var card = boardCard(12, 'the vault',
      read.exists ? (read.names.length + ' sealed') : 'no vault',
      read.exists
        ? 'The sealed credentials. A name is listed; a value is never rendered. The passphrase ' +
          'is held by this process only \u2014 unlock it here without restarting, or launch fleet ' +
          'with FLEET_VAULT_PASSPHRASE.'
        : 'No vault exists yet. Choose a passphrase to create one; it keys the vault and is ' +
          'never stored. There is no recovery, so store it before sealing anything.');
    card.id = cardId || 'settings-vault';

    if (!read.passphrase) {
      var plaintext = read.plaintext || [];
      if (!read.exists) {
        // First run. The passphrase is chosen here, confirmed, and never
        // stored; the create action seals the .env credentials.
        var first = node('div', 'cfg-form');
        first.appendChild(node('p', 'knob-summary',
          'Choose a passphrase for the new vault. It is sent once, held in memory by this ' +
          'process, and never written. There is no recovery: store it in your password ' +
          'manager before you seal anything.'));
        var passphrase = vaultPassword('the new vault passphrase');
        var confirm = vaultPassword('confirm the new vault passphrase');
        var create = node('button', 'replay-button replay-resume',
          'create the vault' + (plaintext.length
            ? ' and seal ' + plaintext.length + ' .env '
              + (plaintext.length === 1 ? 'credential' : 'credentials')
            : ''));
        create.type = 'button';
        create.addEventListener('click', function () {
          if (!passphrase.value) {
            notice('the vault was not created', 'a passphrase is required');
            return;
          }
          if (passphrase.value !== confirm.value) {
            notice('the vault was not created', 'the two passphrases do not match');
            return;
          }
          var value = passphrase.value;
          passphrase.value = '';
          confirm.value = '';
          writeSettings('vault-onboard', { passphrase: value });
        });
        first.appendChild(cfgField('passphrase', passphrase));
        first.appendChild(cfgField('confirm', confirm));
        first.appendChild(node('p', 'cfg-note',
          'The plaintext .env lines stay after migration; the card lets you drop each one ' +
          'once it is sealed.'));
        first.appendChild(create);
        card.appendChild(first);
        if (plaintext.length) {
          card.appendChild(node('h3', 'form-section', 'will be sealed'));
          plaintext.forEach(function (entry) {
            var row = node('div', 'list-row');
            row.appendChild(node('span', 'list-row-key', entry.name));
            row.appendChild(badge('plaintext', 'warn'));
            card.appendChild(row);
          });
        }
        return card;
      }

      // An existing vault, locked for this process.
      var unlockForm = node('div', 'cfg-form');
      unlockForm.appendChild(node('p', 'knob-summary',
        'The vault is locked for this process. Unlocking holds the passphrase in memory until ' +
        'fleet exits or you lock it again; it is never written.'));
      var passphrase = vaultPassword('the vault passphrase');
      var unlock = node('button', 'replay-button replay-resume', 'unlock the vault');
      unlock.type = 'button';
      unlock.addEventListener('click', function () {
        if (!passphrase.value) return;
        var value = passphrase.value;
        passphrase.value = '';
        writeSettings('vault-unlock', { passphrase: value });
      });
      unlockForm.appendChild(passphrase);
      unlockForm.appendChild(unlock);
      card.appendChild(unlockForm);
      if (read.names.length) {
        card.appendChild(node('p', 'cfg-note',
          'The sealed names are still readable; their values are not.'));
        read.names.forEach(function (name) {
          var row = node('div', 'list-row');
          row.appendChild(node('span', 'list-row-key', name));
          row.appendChild(badge('sealed', 'ok'));
          card.appendChild(row);
        });
      }
      // The recovery path for a forgotten passphrase.
      var forgot = node('details', 'cfg-details');
      forgot.appendChild(node('summary', 'cfg-summary', 'I do not know the passphrase'));
      forgot.appendChild(node('p', 'board-empty',
        'Starting a new vault archives the old one (vault.json.bak-<timestamp>); its entries ' +
        'cannot be opened without the old passphrase. The plaintext .env credentials are ' +
        'sealed into the new vault.'));
      var newPass = vaultPassword('a new vault passphrase');
      var newConfirm = vaultPassword('confirm the new vault passphrase');
      var reset = node('button', 'replay-button', 'start a new vault');
      reset.type = 'button';
      reset.addEventListener('click', function () {
        if (!newPass.value) {
          notice('the vault was not reset', 'a passphrase is required');
          return;
        }
        if (newPass.value !== newConfirm.value) {
          notice('the vault was not reset', 'the two passphrases do not match');
          return;
        }
        if (!window.confirm(
          'Archive the old vault and start a new one? Its entries stay sealed and unreadable ' +
          'without the old passphrase.')) {
          return;
        }
        var value = newPass.value;
        newPass.value = '';
        newConfirm.value = '';
        writeSettings('vault-reset', { passphrase: value });
      });
      forgot.appendChild(cfgField('new passphrase', newPass));
      forgot.appendChild(cfgField('confirm', newConfirm));
      forgot.appendChild(reset);
      card.appendChild(forgot);
      return card;
    }

    var status = node('div', 'cfg-inline');
    status.appendChild(badge('unlocked for this process', 'ok'));
    var lock = node('button', 'replay-button', 'lock the vault');
    lock.type = 'button';
    lock.addEventListener('click', function () {
      writeSettings('vault-lock', {});
    });
    status.appendChild(lock);
    card.appendChild(status);

    if (!read.exists || !read.names.length) {
      var unsealed = (read.plaintext || []).filter(function (entry) {
        return !entry.sealed;
      });
      var create = node('button', 'replay-button replay-resume',
        'create the vault' + (unsealed.length
          ? ' and seal ' + unsealed.length + ' .env '
            + (unsealed.length === 1 ? 'entry' : 'entries')
          : ''));
      create.type = 'button';
      create.addEventListener('click', function () {
        if (unsealed.length && !window.confirm(
          'Create the vault and seal the ' + unsealed.length + ' credential-shaped .env ' +
          'entries? The plaintext lines stay; the card lets you drop each one after.')) return;
        writeSettings('vault-init', {});
      });
      card.appendChild(create);
    }

    if (read.names.length) {
      read.names.forEach(function (name) {
        var row = node('div', 'list-row');
        row.appendChild(node('span', 'list-row-key', name));
        row.appendChild(badge('sealed', 'ok'));
        var remove = node('button', 'replay-button', 'remove');
        remove.type = 'button';
        remove.addEventListener('click', function () {
          if (!window.confirm('Remove the sealed entry `' + name + '`?')) return;
          writeSettings('secret-remove', { name: name });
        });
        row.appendChild(remove);
        card.appendChild(row);
      });
    } else if (read.exists) {
      card.appendChild(node('p', 'board-empty', 'No entry is sealed yet.'));
    }

    // The .env side: seal a plaintext credential, then drop the line. Both
    // actions are server-side, so the value never reaches this page.
    if ((read.plaintext || []).length) {
      card.appendChild(node('h3', 'form-section', 'in .env'));
      card.appendChild(node('p', 'cfg-note',
        'A sealed name overrides its plaintext line. Seal first, then drop the line.'));
      read.plaintext.forEach(function (entry) {
        var row = node('div', 'list-row');
        row.appendChild(node('span', 'list-row-key', entry.name));
        row.appendChild(badge(entry.sealed ? 'sealed' : 'plaintext',
          entry.sealed ? 'ok' : 'warn'));
        var actions = node('div', 'replay-actions');
        if (entry.sealed) {
          var drop = node('button', 'replay-button', 'drop the .env line');
          drop.type = 'button';
          drop.addEventListener('click', function () {
            if (!window.confirm('Remove the plaintext `' + entry.name + '` line from .env?')) {
              return;
            }
            writeSettings('vault-remove-env', { name: entry.name });
          });
          actions.appendChild(drop);
        } else {
          var seal = node('button', 'replay-button', 'seal it');
          seal.type = 'button';
          seal.addEventListener('click', function () {
            writeSettings('vault-seal-env', { name: entry.name });
          });
          actions.appendChild(seal);
        }
        row.appendChild(actions);
        card.appendChild(row);
      });
    }

    var form = node('div', 'cfg-form');
    var name = node('input', 'knob-box');
    name.type = 'text';
    name.placeholder = 'FLEET_TELEGRAM_BOT_TOKEN';
    name.setAttribute('aria-label', 'a new sealed entry name');
    var value = node('input', 'knob-box');
    value.type = 'password';
    value.autocomplete = 'off';
    value.placeholder = 'the value';
    value.setAttribute('aria-label', 'a new sealed entry value');
    var seal = node('button', 'replay-button replay-resume', 'seal the entry');
    seal.type = 'button';
    seal.addEventListener('click', function () {
      if (!name.value.trim() || !value.value.trim()) return;
      var secret = value.value;
      value.value = '';
      writeSettings('secret', { name: name.value.trim(), value: secret });
    });
    form.appendChild(cfgField('name', name));
    form.appendChild(cfgField('value', value));
    form.appendChild(seal);
    card.appendChild(form);
    return card;
  }

  // ------------------------------------------------------------- the spend

  function spendCard() {
    var caps = {
      FLEET_JUDGE_BUDGET_USD_DAY: 'judge',
      FLEET_DELEGATE_BUDGET_USD_DAY: 'delegate',
      FLEET_EXPLAIN_BUDGET_USD_DAY: 'explain'
    };
    var card = boardCard(12, 'the spend caps', null,
      'The daily cap each judged beat may spend. A cap tightens the compiled ceiling and can ' +
      'never raise it; a write lands in fleet.env and applies at the next launch. Today\u2019s ' +
      'spend is on the calibration panel.');
    card.id = 'settings-spend';
    (face.settings.scalars || []).forEach(function (knob) {
      if (!caps[knob.key]) return;
      var row = node('div', 'cfg-form');
      var head = node('div', 'cfg-form-head');
      head.appendChild(node('span', 'list-row-key', caps[knob.key]));
      head.appendChild(badge(knob.value === 'unset' ? 'compiled default' : '$' + knob.value,
        knob.value === 'unset' ? null : 'ok'));
      if (knob.source !== 'default') {
        head.appendChild(badge(knob.source, knob.source === 'env' ? 'warn' : null));
      }
      row.appendChild(head);
      row.appendChild(node('p', 'knob-summary', knob.summary));
      var box = node('input', 'knob-box');
      box.type = 'number';
      box.step = '0.01';
      box.min = '0';
      if (knob.ceiling) box.max = String(knob.ceiling);
      // The compiled ceiling is the value in force, so the box is seeded
      // with it; the badge still says it is the default, and saving it
      // writes the same number explicitly.
      box.value = knob.value === 'unset'
        ? (knob.ceiling == null ? '' : String(knob.ceiling))
        : knob.value;
      box.setAttribute('aria-label', caps[knob.key] + ' daily cap');
      var save = node('button', 'replay-button', 'save');
      save.type = 'button';
      save.addEventListener('click', function () {
        writeSettings('set', { key: knob.key, value: box.value });
      });
      var line = node('div', 'cfg-inline');
      line.appendChild(box);
      line.appendChild(save);
      if (knob.ceiling) line.appendChild(node('span', 'cfg-note', 'at most ' + knob.ceiling));
      row.appendChild(line);
      card.appendChild(row);
    });
    return card;
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
      : 'Shift+Enter makes a newline';
  });

  // The plan door: the same task, the plan arm. A plan-mode run's final
  // message records as a plan, and the card it lands in is editable —
  // Build posts the edited text as an ordinary run.
  function planSend() {
    var task = el.task.value.trim();
    if (!task || face.running || face.seat) return;
    el.task.value = '';
    grow();
    clearDraft();
    el.empty.hidden = true;
    el.log.hidden = false;
    face.planned = true;
    Promise.resolve(folderSync).then(function () {
      var planPath = '/api/plan' + (face.session
        ? '?session=' + encodeURIComponent(face.session) : '');
      return post(planPath, { task: task });
    }).then(function (reply) {
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

    calibration: el.panelCalibration,
    'second-brain': el.panelSecondBrain,
    'workspace-history': el.panelWorkspaceHistory,
    bots: el.panelBots,
    vault: el.panelVault,
    settings: el.panelSettings
  };

  function showRoute() {
    var raw = (window.location.hash || '').replace(/^#\/?/, '');
    var parts = raw.split('/');
    var which = parts[0] || '';
    var focusId = parts.length > 1 ? decodeURIComponent(parts.slice(1).join('/')) : '';
    // The deliverable address (KI-289): an agent closes its work with a
    // link, and the link opens the face with the pane's preview in
    // front — the rendered document for markdown, the file otherwise.
    // The conversation stays the surface behind the pane.
    if (which === 'file' && focusId) {
      face.panel = 'chat';
      face.focus = null;
      try { window.localStorage.setItem('fleet-panel', 'chat'); } catch (error) { }
      markRail();
      setWork(false);
      Object.keys(PANEL_EL).forEach(function (key) {
        PANEL_EL[key].hidden = key !== 'chat';
      });
      el.panelLater.hidden = true;
      readHome(false).then(function () {
        paintHome();
        paintRecent();
        paintSince();
      }).catch(function () { });
      openPane();
      showPaneTab('preview');
      openFile(focusId);
      return;
    }
    var known = ROUTES.filter(function (route) { return route[0] === which; })[0];
    var address = which === 'chat' || !known ? 'chat' : which;
    // The work surface is not a panel of its own any more: it is the
    // conversation with the work column beside it, so the two are one
    // place with one verb (open in place) instead of a list you leave.
    var work = address === 'sessions';
    closeDrawer();
    face.panel = address;
    usageRecord('panels', address, 'opens');
    face.focus = focusId ? { panel: address, id: focusId } : null;
    try { window.localStorage.setItem('fleet-panel', address); } catch (error) { }
    markRail();
    setWork(work);
    Object.keys(PANEL_EL).forEach(function (key) {
      PANEL_EL[key].hidden = work ? key !== 'chat' : key !== address;
    });
    var panelHere = PANEL_EL[address] || (work ? el.panelChat : null);
    el.panelLater.hidden = Boolean(panelHere);
    if (!panelHere) {
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
      bots: function () {
        paintBots();
        readBots(true).then(paintBots);
        readAccess(true).then(paintBots);
      },
      sessions: function () {
        paintWork();
        paintConvHead();
        var asked = face.focus && face.focus.id ? face.focus.id : '';
        if (asked && asked !== face.session) {
          openWork(asked, { keepHash: true });
          return;
        }
        readSessions(true).then(function () {
          var rows = workRows();
          var needs = rows.filter(function (row) { return workNeeds(row.state); })[0];
          var pick = face.session || (needs || rows[0] || {}).id;
          if (pick) return openWork(pick);
          paintWork();
          paintConvHead();
        });
      },
      calibration: function () { paintCalibration(); readCalibration(true).then(paintCalibration); },
      'second-brain': function () { paintBrain(); readBrain(true).then(paintBrain); readPlugin(true); },
      'workspace-history': function () {
        paintWorkspaceHistory();
        readWorkspaceHistory(true).then(paintWorkspaceHistory);
        readWorkspaceCalls(true);
      },
      vault: function () { paintVault(); readSettings(true).then(paintVault); },
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
  // A path the reply names opens the pane at that file (D7, KI-289's
  // transcript half): one linkifier for the prose spans, applied after
  // the markdown shapes so a code span stays code.
  function linkifyPaths(text) {
    var out = node('span');
    var rest = String(text);
    var pattern = /(`[^`]+`)|([\w./-]+\/[\w./-]+)/g;
    var at = 0;
    var match;
    while ((match = pattern.exec(rest)) !== null) {
      if (match.index > at) out.appendChild(document.createTextNode(rest.slice(at, match.index)));
      if (match[1]) {
        out.appendChild(node('code', 'md-inline-code', match[1].slice(1, -1)));
      } else {
        var path = match[2];
        if (/\.(rs|js|ts|css|html|md|json|toml|py|sh|yaml|yml|txt)$/i.test(path) || /^src\//.test(path)) {
          var open = node('button', 'md-path', path);
          open.type = 'button';
          open.title = 'open ' + path + ' in the pane';
          open.addEventListener('click', function (here) {
            return function () { openPane(); openFile(here); };
          }(path));
          out.appendChild(open);
        } else {
          out.appendChild(document.createTextNode(path));
        }
      }
      at = match.index + match[0].length;
    }
    if (at < rest.length) out.appendChild(document.createTextNode(rest.slice(at)));
    return out;
  }

  function inline(text) {
    var span = node('span');
    var rest = String(text);
    var pattern = /(`[^`]+`)|(\*\*[^*]+\*\*)|(\[[^\]]+\]\([^)]+\))/g;
    var at = 0;
    var match;
    while ((match = pattern.exec(rest)) !== null) {
      if (match.index > at) span.appendChild(linkifyPaths(rest.slice(at, match.index)));
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
    if (at < rest.length) span.appendChild(linkifyPaths(rest.slice(at)));
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

  // The reference listbox (D5): the same box, grammar, and keys as the
  // command surface, over four families sourced from reads the face
  // already makes — files from the pane's tree, sessions from the work
  // rows, skills and memory from their own reads. Enter inserts; the
  // file reference opens its pane preview, and the session reference
  // leaves a link in the sent message that opens that conversation.
  function referenceRows(asked) {
    var query = (asked || '').toLowerCase();
    var rows = [];
    var read = face.tree;
    if (read && !read.refused) {
      var base = read.path ? read.path.replace(/\/$/, '') + '/' : '';
      (read.entries || []).forEach(function (entry) {
        if (entry.kind !== 'dir') {
          rows.push({ kind: 'file', label: base + entry.name,
            insert: base + entry.name, follow: 'file', value: base + entry.name });
        }
      });
    } else if (!face.tree) {
      readTree('', true);
    }
    workRows().forEach(function (one) {
      if (one.id !== face.session) {
        rows.push({ kind: 'session', label: one.title || one.id,
          insert: '@' + (one.title || one.id), follow: 'session', value: one.id });
      }
    });
    ((face.skills && face.skills.skills) || []).forEach(function (skill) {
      rows.push({ kind: 'skill', label: skill.name,
        insert: 'use the skill "' + skill.name + '"', follow: null });
    });
    ((face.memory && face.memory.facts) || []).forEach(function (fact) {
      if (fact && fact.name && !fact.retired) {
        rows.push({ kind: 'memory', label: fact.name,
          insert: 'recall the fact "' + fact.name + '"', follow: null });
      }
    });
    if (query) {
      rows = rows.filter(function (one) {
        return one.label.toLowerCase().indexOf(query) !== -1 ||
          one.kind.indexOf(query) === 0;
      });
    }
    return rows.slice(0, 12);
  }

  function openReferences(asked) {
    face.referenceOpen = true;
    face.referenceAt = 0;
    closeCommands();
    // The tree read may still be in flight on the first @: the box
    // paints what it has now and re-ranks when the read lands, so the
    // files family never silently misses.
    if (!face.tree) {
      var wanted = asked;
      Promise.resolve(readTree('', true)).then(function () {
        if (face.referenceOpen) openReferences(wanted);
      });
    }
    face.references = referenceRows(asked);
    paintReferences();
  }

  function closeReferences() {
    face.referenceOpen = false;
    face.references = [];
    if (!face.commandOpen) el.commands.hidden = true;
  }

  function moveReference(step) {
    if (!face.references.length) return;
    face.referenceAt = (face.referenceAt + step + face.references.length) % face.references.length;
    paintReferences();
  }

  function paintReferences() {
    if (!el.commands) return;
    el.commands.textContent = '';
    if (!face.references.length) {
      el.commands.hidden = true;
      return;
    }
    el.commands.hidden = false;
    face.references.forEach(function (one, at) {
      var row = node('button', 'command reference');
      row.type = 'button';
      row.setAttribute('role', 'option');
      row.dataset.kind = one.kind;
      if (at === face.referenceAt) row.setAttribute('aria-selected', 'true');
      row.appendChild(node('span', 'command-name', '@' + one.kind));
      row.appendChild(node('span', 'command-said', one.label));
      row.addEventListener('click', function () { insertReference(at); });
      el.commands.appendChild(row);
    });
  }

  // Insert the highlighted reference: the trailing @token is replaced by
  // the reference's text, and the kind's follow-on fires — a file opens
  // its pane preview, a session records the link the sent message
  // carries. The composer keeps focus: the owner is mid-sentence.
  function insertReference(at) {
    var one = face.references[at];
    if (!one) return;
    var value = el.task.value;
    var atMatch = value.match(/(^|\s)(@[^\s@]*)$/);
    if (atMatch) {
      value = value.slice(0, value.length - atMatch[2].length) + one.insert + ' ';
    } else {
      value += one.insert + ' ';
    }
    el.task.value = value;
    saveDraft();
    grow();
    closeReferences();
    if (one.follow === 'file') {
      openPane();
      openFile(one.value);
    } else if (one.follow === 'session') {
      face.sessionRefs = face.sessionRefs || {};
      face.sessionRefs[one.insert] = one.value;
    }
    el.task.focus();
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
  // rail says so rather than pretending every destination is built.
  var BUILT = ['chat', 'router', 'delegation', 'memory', 'skills', 'sessions', 'calibration',
    'tower', 'second-brain', 'workspace-history', 'bots', 'vault', 'settings'];

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
    // The installed app's browser chrome follows the theme; the manifest
    // can only carry one colour (paper), so this keeps the top bar from
    // staying paper under graphite.
    var chrome = document.querySelector('meta[name="theme-color"]');
    if (chrome) {
      var colors = {
        paper: '#f6f4ef',
        console: '#0e0f12',
        graphite: '#0a0c10',
        contrast: '#ffffff'
      };
      chrome.setAttribute('content', colors[mode] || colors.paper);
    }
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
  // The palette's own key, and Esc from anywhere: the overlays close
  // first (the goal pop, then the palette), an empty pane closes with
  // it, and a pane holding a file stays up while the hash walks home —
  // close-when-empty, home-when-file-open.
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
    if (el.goalPop && !el.goalPop.hidden) { setGoalPop(false); return; }
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
    // The tool's heartbeat is cheap enough for this clock, and the
    // state read is not: the plugin polls itself at one second while a
    // job runs, and this keeps the live dot honest when it does not.
    if (face.panel === 'second-brain') readPlugin(false);
  }, 5000);

  // The counters land after the first offers paint; the offers repaint
  // in their learned order, and the exposure was already recorded once.
  readUsage().then(paintOffers);

  // ------------------------------------------------------------- pairing

  // The QR card. The icon exists only on the loopback face: a paired
  // phone does not pair phones, and a relayed page has no origin of its
  // own to mint a code for. The server mints a fresh window per open, so
  // the newest code is the live one and an older screenshot stops working.
  function canPair() {
    if (window.location.pathname.indexOf('/s/') === 0) return false;
    var host = window.location.hostname;
    return host === '127.0.0.1' || host === 'localhost' || host === '::1' || host === '[::1]';
  }

  function closePair() {
    el.pairScrim.hidden = true;
    el.pairCard.hidden = true;
    el.pairBody.textContent = '';
  }

  function pairError(message) {
    el.pairBody.textContent = '';
    var card = node('div', 'pair-error');
    card.appendChild(node('h3', null, 'no code was minted'));
    card.appendChild(node('p', null, message));
    el.pairBody.appendChild(card);
  }

  function paintPair(doc) {
    el.pairBody.textContent = '';
    var qr = node('div', 'pair-qr');
    // Server-generated from the matrix: rects and numbers, no user text.
    qr.innerHTML = doc.svg;
    qr.setAttribute('role', 'img');
    qr.setAttribute('aria-label', 'pairing QR code');
    var side = node('div', 'pair-side');
    side.appendChild(node('p', 'pair-eyebrow', 'scan with your phone'));
    side.appendChild(node('div', 'pair-code', doc.code));
    side.appendChild(node('p', 'pair-url', doc.url));
    side.appendChild(node('p', 'pair-origin',
      'via ' + (doc.origin_label || doc.origin) + ' \u00b7 ' + doc.origin));
    if (doc.note) {
      var note = node('p', 'pair-note', doc.note);
      note.dataset.tone = 'warn';
      side.appendChild(note);
    }
    side.appendChild(node('p', 'pair-ttl',
      'this code lives ' + doc.ttl_minutes + ' minutes and burns when it is used. ' +
      'Opening this card again mints a new one; the newest code wins.'));
    var acts = node('div', 'pair-actions');
    var copy = node('button', 'replay-button', 'copy the URL');
    copy.type = 'button';
    copy.addEventListener('click', function () {
      if (navigator.clipboard) navigator.clipboard.writeText(doc.url).catch(function () { });
      copy.textContent = 'copied';
      window.setTimeout(function () { copy.textContent = 'copy the URL'; }, 1200);
    });
    acts.appendChild(copy);
    var again = node('button', 'replay-button', 'new code');
    again.type = 'button';
    again.addEventListener('click', function () {
      usageRecord('actions', 'pair:new', 'uses');
      mintPair();
    });
    acts.appendChild(again);
    var done = node('button', 'replay-button replay-resume', 'done');
    done.type = 'button';
    done.addEventListener('click', closePair);
    acts.appendChild(done);
    side.appendChild(acts);
    el.pairBody.appendChild(qr);
    el.pairBody.appendChild(side);
  }

  function mintPair() {
    el.pairBody.textContent = '';
    el.pairBody.appendChild(node('p', 'pair-loading',
      'minting a code\u2026 this asks this machine for a new single-use window.'));
    fetch('/api/pair/window', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: '{}'
    }).then(function (reply) {
      return reply.json().then(function (doc) { return { ok: reply.ok, doc: doc }; });
    }).then(function (result) {
      if (!result.ok) {
        pairError(result.doc.error || 'the pairing window was refused');
        return;
      }
      paintPair(result.doc);
    }).catch(function () {
      pairError('could not reach fleet to mint a code');
    });
  }

  function openPair() {
    usageRecord('actions', 'pair:open', 'uses');
    el.pairScrim.hidden = false;
    el.pairCard.hidden = false;
    mintPair();
  }

  if (canPair()) {
    el.pairOpen.hidden = false;
    el.pairOpen.addEventListener('click', openPair);
    el.pairClose.addEventListener('click', closePair);
    el.pairScrim.addEventListener('click', closePair);
    window.addEventListener('keydown', function (event) {
      if (event.key === 'Escape' && !el.pairCard.hidden) closePair();
    });
  }

  load()
    .then(function () {
      // The boot watchdog reads this: a shell that got this far is not a
      // failed boot, even if a panel's read is still in flight.
      document.documentElement.setAttribute('data-fleet-mounted', '1');
      listen();
    })
    .catch(function (error) {
      document.documentElement.setAttribute('data-fleet-mounted', '1');
      if (error && error.message === 'token') {
        el.empty.hidden = false;
        el.empty.querySelector('h1').textContent = 'This browser is not paired with fleet.';
        el.empty.querySelector('p').textContent =
          'Open the address fleet printed and pair this device. A paired device holds a cookie; ' +
          'a browser without one sees only the pairing page.';
      }
    });
})();
