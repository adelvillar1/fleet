// core/store.js — the shared face state (P2 / U43).
//
// The one mutable state object every lens and the shell read. Extracted
// from the monolith so the store has a single home; app.js imports it.
export const CHAT_SCALES = [0.85, 1, 1.15, 1.3, 1.5, 1.7];
export const CHAT_SCALE_DEFAULT = 1;

export const face = {
    session: '',
    // The messages typed while a run executes, in the owner's order:
    // one flushes per run, the head first.
    queued: [],
    // The queued rows' rebuild guard: what the rows last said, in order.
    // `paintQueue` runs on every event, and a rebuild between the
    // owner's mousedown and mouseup eats the click on send, edit or drop.
    queueKey: '',
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
    turnSummaries: null,
    sessionQuery: '',
    memoryFilter: 'live',
    // The library holds 442 rows; the query and the show-all toggle are
    // what make a table of that length usable rather than a wall.
    skillQuery: '',
    skillShowAll: false,
    towerFilter: 'all',
    towerDay: '',
    routerHour: null,
    routerModel: '',
    // The visit stamp, read before it is overwritten, so the home can
    // say what happened while the owner was away.
    lastVisit: '',
    // column the owner never asked for standing open.
    // The live process: the run's own stage, turn, tool, and counts,
    // reduced from the same event log the transcript reads.
    process: null,
    // The goal's task list (the goal-task-pane plan): the last full
    // replacement a `todo` call carried — live off the `tasks` event,
    // rebuilt on load from the log's `tasks` records. A view, never a
    // synthesis: empty means the model declared nothing.
    tasks: [],
    // The completion's note, off the `goal_done` event. The done store
    // keeps the goal text; the note rides the event only, so it shows
    // live and on a replayed log, not on a bare state read.
    goalNote: '',
    // The verify step's gate, read from the state document: the same
    // failover knob the route module owns.
    gate: null,
    budget: 0,
    // The model whose declared window derived the budget, and whether
    // anything declared one at all (false = the floor, named).
    budgetModel: '',
    budgetDeclared: true,
    spend: 0,
    context: 0,
    // The health fields (the context-health plan's C2/C3/C7): the run's
    // folds so far, the first-hand share, the served provider's cached
    // count (null = the provider said nothing), and the last prompt
    // readings the turns-to-fold trend is derived from.
    folds: 0,
    share: null,
    cacheRead: null,
    promptHist: [],
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
    // The New intent: set by New, cleared by opening a conversation or
    // by the send that materializes it. The home's composer composes
    // into nothing until then — an abandoned compose leaves no log.
    pendingNew: false,
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
    // The run tab is the pane's own default: where are we, before what exists.
    paneTab: 'run'
  };
