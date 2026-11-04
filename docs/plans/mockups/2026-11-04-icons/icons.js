// The icon sheet's data: one entry per key — the shapes, what it means, where
// it is used, and whether it is kept, redrawn or new. The implementation
// transcribes this table into `ICONS` in src/gui/assets/app.js; the sheet is
// the review, the table is the code.

const ICONS = {
  // ------------------------------------------------------------ the main rail
  chat: {
    status: 'keep',
    meaning: 'the conversation: one prompt, one answer',
    use: 'main rail · palette · calibration "turn" · memory/offers',
    shapes: [['path', { d: 'M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z' }]]
  },
  design: {
    status: 'keep',
    meaning: 'the design canvas: a page on a board',
    use: 'main rail · palette · every board row',
    shapes: [
      ['rect', { x: 3.5, y: 4.5, width: 17, height: 15, rx: 2 }],
      ['circle', { cx: 8.5, cy: 9.5, r: 1.6 }],
      ['path', { d: 'M4 17.5l4.5-4.5 3.5 3.5 3-2.5 5 4' }]
    ]
  },
  sessions: {
    status: 'redraw',
    meaning: 'the archive: every conversation, stacked',
    use: 'main rail · palette · memory "archive"',
    was: [
      ['circle', { cx: 12, cy: 12, r: 8.5 }],
      ['path', { d: 'M12 7.5V12l3 1.8' }]
    ],
    shapes: [
      ['path', { d: 'M7 4.5h9a2 2 0 0 1 2 2v9' }],
      ['rect', { x: 4, y: 7.5, width: 13, height: 13, rx: 2 }],
      ['path', { d: 'M7.5 11.5h6M7.5 15h4' }]
    ]
  },
  huddles: {
    status: 'new',
    meaning: 'a room: the owner and bots, together',
    use: 'main rail · palette (was the same ring as vault and plugins)',
    shapes: [
      ['circle', { cx: 9, cy: 8, r: 2.7 }],
      ['path', { d: 'M4.6 18.5c.5-3 2.6-4.6 4.9-4.6s4.4 1.6 4.9 4.6' }],
      ['circle', { cx: 16.8, cy: 9.6, r: 2.2 }],
      ['path', { d: 'M15.2 14.4c2 .2 3.6 1.6 4 4.1' }]
    ]
  },
  router: {
    status: 'keep',
    meaning: 'the route: work entering a chain and leaving it',
    use: 'main rail · palette · calibration "route" · skills "web" (moving)',
    shapes: [
      ['path', { d: 'M4 12h5l4-6h6' }],
      ['path', { d: 'M9 12l4 6h6' }],
      ['circle', { cx: 4, cy: 12, r: 1.8 }],
      ['circle', { cx: 19, cy: 6, r: 1.8 }],
      ['circle', { cx: 19, cy: 18, r: 1.8 }]
    ]
  },
  delegation: {
    status: 'redraw',
    meaning: 'delegation: one brief, children beneath it',
    use: 'main rail · palette · settings "specialists" · calibration',
    was: [
      ['circle', { cx: 6, cy: 6, r: 2 }],
      ['circle', { cx: 18, cy: 6, r: 2 }],
      ['circle', { cx: 12, cy: 18, r: 2 }],
      ['path', { d: 'M7.4 7.3 10.8 16.2' }],
      ['path', { d: 'M16.6 7.3 13.2 16.2' }],
      ['path', { d: 'M8 6h8' }]
    ],
    shapes: [
      ['rect', { x: 9, y: 3, width: 6, height: 5, rx: 1.2 }],
      ['rect', { x: 3, y: 16, width: 6, height: 5, rx: 1.2 }],
      ['rect', { x: 15, y: 16, width: 6, height: 5, rx: 1.2 }],
      ['path', { d: 'M12 8v4M6 16v-4h12v4' }]
    ]
  },
  memory: {
    status: 'keep',
    meaning: 'memory: the facts the home keeps',
    use: 'main rail · palette · memory "reference" · skills "data" (moving)',
    shapes: [
      ['path', { d: 'M4 19.5A2.5 2.5 0 0 1 6.5 17H20' }],
      ['path', { d: 'M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z' }]
    ]
  },
  skills: {
    status: 'keep',
    meaning: 'skills: the library, layer on layer',
    use: 'main rail · palette · bots (moving to initials)',
    shapes: [
      ['path', { d: 'M12 2 2 7l10 5 10-5-10-5z' }],
      ['path', { d: 'M2 12l10 5 10-5' }],
      ['path', { d: 'M2 17l10 5 10-5' }]
    ]
  },
  'second-brain': {
    status: 'keep',
    meaning: 'the project atlas: many projects, one grid',
    use: 'main rail · palette',
    shapes: [
      ['rect', { x: 3.5, y: 3.5, width: 7, height: 7, rx: 1.5 }],
      ['rect', { x: 13.5, y: 3.5, width: 7, height: 7, rx: 1.5 }],
      ['rect', { x: 3.5, y: 13.5, width: 7, height: 7, rx: 1.5 }],
      ['rect', { x: 13.5, y: 13.5, width: 7, height: 7, rx: 1.5 }]
    ]
  },
  'workspace-history': {
    status: 'redraw',
    meaning: 'workspace history: a project, its branches',
    use: 'main rail · palette · every workspace project',
    was: [
      ['circle', { cx: 6, cy: 5.5, r: 2.4 }],
      ['circle', { cx: 6, cy: 12, r: 2.4 }],
      ['circle', { cx: 6, cy: 18.5, r: 2.4 }],
      ['path', { d: 'M6 8v2.5M6 14.5v2' }],
      ['circle', { cx: 17, cy: 9.5, r: 2.4 }],
      ['path', { d: 'M8.4 6.6 14.6 8.8' }],
      ['path', { d: 'M8.4 12.6 14.6 10.2' }]
    ],
    shapes: [
      ['path', { d: 'M6 4.5v15' }],
      ['circle', { cx: 6, cy: 6, r: 2.1 }],
      ['circle', { cx: 6, cy: 18, r: 2.1 }],
      ['circle', { cx: 17.5, cy: 10, r: 2.1 }],
      ['path', { d: 'M6 13.5h6.5a3 3 0 0 0 3-3' }]
    ]
  },
  bots: {
    status: 'redraw',
    meaning: 'bots: the personas who answer',
    use: 'main rail · palette · bots "channels" · skills "platforms" (moving)',
    was: [
      ['rect', { x: 4, y: 8, width: 16, height: 11, rx: 2.5 }],
      ['path', { d: 'M12 8V4.5' }],
      ['circle', { cx: 12, cy: 3.4, r: 1.3 }],
      ['path', { d: 'M9 13v1.5' }],
      ['path', { d: 'M15 13v1.5' }],
      ['path', { d: 'M2.5 12.5v3' }],
      ['path', { d: 'M21.5 12.5v3' }]
    ],
    shapes: [
      ['rect', { x: 4, y: 8, width: 16, height: 10.5, rx: 3 }],
      ['path', { d: 'M12 8V5.6' }],
      ['circle', { cx: 12, cy: 4.5, r: 1.1 }],
      ['path', { d: 'M9.5 12.8v1.4M14.5 12.8v1.4' }]
    ]
  },
  vault: {
    status: 'new',
    meaning: 'the vault: a sealed credential',
    use: 'main rail · palette · settings "trust" (moving to an open lock)',
    shapes: [
      ['rect', { x: 5, y: 10, width: 14, height: 10, rx: 2.5 }],
      ['path', { d: 'M8.5 10V7.2a3.5 3.5 0 0 1 7 0V10' }],
      ['circle', { cx: 12, cy: 15, r: 1.1 }]
    ]
  },
  settings: {
    status: 'keep',
    meaning: 'settings: the knobs of this home',
    use: 'main rail · palette · calibration "tool screen" (moving)',
    shapes: [
      ['path', { d: 'M4 7h9M17 7h3' }],
      ['circle', { cx: 15, cy: 7, r: 2 }],
      ['path', { d: 'M4 17h3M11 17h9' }],
      ['circle', { cx: 9, cy: 17, r: 2 }]
    ]
  },
  calibration: {
    status: 'redraw',
    meaning: 'calibration: a measured signal, two beats',
    use: 'main rail · palette · calibration ops',
    was: [['path', { d: 'M3 12h4l2.5-7 4 14 2.5-7h5' }]],
    shapes: [['path', { d: 'M3 13h3.5l2.5-6 4 11 2.5-6H21' }]]
  },
  plugins: {
    status: 'new',
    meaning: 'the plugin host: something that slots in',
    use: 'main rail · palette (was the same ring as huddles and vault)',
    shapes: [
      ['path', { d: 'M9 3v4M15 3v4' }],
      ['path', { d: 'M7 10.5h10v2.5a5 5 0 0 1-10 0z' }],
      ['path', { d: 'M12 18v3' }]
    ]
  },
  plugin: {
    status: 'keep',
    meaning: 'one installed bundle',
    use: 'a mounted plugin row in the rail',
    shapes: [
      ['rect', { x: 4, y: 4, width: 16, height: 16, rx: 3 }],
      ['path', { d: 'M9 9h6v6H9z' }]
    ]
  },

  // ------------------------------------------------------- the section rails
  pending: {
    status: 'new',
    meaning: 'needs you: something waits for a decision',
    use: 'memory and skills "needs you" (was the same ring as a bug)',
    shapes: [
      ['path', { d: 'M6.5 16.5h11l-1.4-2.2V10a4.1 4.1 0 0 0-8.2 0v4.3z' }],
      ['path', { d: 'M10 19.2a2 2 0 0 0 4 0' }]
    ]
  },
  repo: {
    status: 'new',
    meaning: 'a project: a folder the home works in',
    use: 'memory "project" · every workspace-history row',
    shapes: [['path', { d: 'M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z' }]]
  },
  person: {
    status: 'new',
    meaning: 'the owner: facts about you',
    use: 'memory "user"',
    shapes: [
      ['circle', { cx: 12, cy: 8, r: 3.2 }],
      ['path', { d: 'M5.2 20c.7-4 3.5-6 6.8-6s6.1 2 6.8 6' }]
    ]
  },
  rules: {
    status: 'new',
    meaning: 'feedback: a correction that stood',
    use: 'memory "feedback" · skills "curation" (moving)',
    shapes: [
      ['path', { d: 'M6 3.5h8l4 4v13H6z' }],
      ['path', { d: 'M14 3.5v4h4' }],
      ['path', { d: 'M9 12.5l1.8 1.8L14 11' }]
    ]
  },
  events: {
    status: 'new',
    meaning: 'episodic: what happened, in order',
    use: 'memory "episodic"',
    shapes: [
      ['path', { d: 'M4 12h16' }],
      ['circle', { cx: 7, cy: 12, r: 1.8 }],
      ['circle', { cx: 12, cy: 12, r: 1.8 }],
      ['circle', { cx: 17, cy: 12, r: 1.8 }]
    ]
  },
  other: {
    status: 'new',
    meaning: 'everything else: no category claims it',
    use: 'memory and skills "everything else" (was the bug mark)',
    shapes: [
      ['circle', { cx: 6.5, cy: 12, r: 1.5 }],
      ['circle', { cx: 12, cy: 12, r: 1.5 }],
      ['circle', { cx: 17.5, cy: 12, r: 1.5 }]
    ]
  },
  retired: {
    status: 'new',
    meaning: 'retired: kept, out of reach',
    use: 'memory and skills "retired"',
    shapes: [['path', { d: 'M20 14.6A8.2 8.2 0 1 1 9.4 4a6.6 6.6 0 0 0 10.6 10.6z' }]]
  },
  archive: {
    status: 'new',
    meaning: 'archive: put away, still kept',
    use: 'memory "archive"',
    shapes: [
      ['rect', { x: 3.5, y: 4.5, width: 17, height: 4.5, rx: 1.2 }],
      ['path', { d: 'M5.5 9v9a2 2 0 0 0 2 2h9a2 2 0 0 0 2-2V9' }],
      ['path', { d: 'M10 13h4' }]
    ]
  },
  today: {
    status: 'new',
    meaning: 'today: the day the home is in',
    use: 'memory "today"',
    shapes: [
      ['circle', { cx: 12, cy: 12, r: 8.2 }],
      ['path', { d: 'M12 7.5V12l3 1.8' }]
    ]
  },
  plan: {
    status: 'redraw',
    meaning: 'plan: written down before the work',
    use: 'skills "building" · calibration "verify" · the home offers',
    was: [
      ['path', { d: 'M9 4h6v3H9z' }],
      ['path', { d: 'M9 4H7a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V6a2 2 0 0 0-2-2h-2' }]
    ],
    shapes: [
      ['rect', { x: 5, y: 5, width: 14, height: 15.5, rx: 2 }],
      ['path', { d: 'M9.5 5V3.6h5V5' }],
      ['path', { d: 'M9 11h6M9 14.5h4' }]
    ]
  },
  web: {
    status: 'new',
    meaning: 'the web: the network itself',
    use: 'skills "web" (the route belongs to the router)',
    shapes: [
      ['circle', { cx: 12, cy: 12, r: 8.2 }],
      ['path', { d: 'M4 12h16' }],
      ['path', { d: 'M12 3.8c2.4 2.4 3.6 5.2 3.6 8.2s-1.2 5.8-3.6 8.2c-2.4-2.4-3.6-5.2-3.6-8.2S9.6 6.2 12 3.8z' }]
    ]
  },
  pen: {
    status: 'new',
    meaning: 'design work: drawn, not built',
    use: 'skills "design" (the artboard stays the canvas)',
    shapes: [
      ['path', { d: 'M14.5 4.5l5 5L9 20H4v-5z' }],
      ['path', { d: 'M12.5 6.5l5 5' }]
    ]
  },
  media: {
    status: 'new',
    meaning: 'media: something that plays',
    use: 'skills "media" (the clock belongs to the archive)',
    shapes: [
      ['circle', { cx: 12, cy: 12, r: 8.2 }],
      ['path', { d: 'M10 8.8l5.2 3.2-5.2 3.2z' }]
    ]
  },
  documents: {
    status: 'new',
    meaning: 'documents: a page of text',
    use: 'skills "documents" (the folder is a project)',
    shapes: [
      ['rect', { x: 5, y: 3.5, width: 14, height: 17, rx: 1.5 }],
      ['path', { d: 'M8.5 8h7M8.5 11.5h7M8.5 15h4' }]
    ]
  },
  data: {
    status: 'new',
    meaning: 'data: a store you can query',
    use: 'skills "data" (the book is memory)',
    shapes: [
      ['ellipse', { cx: 12, cy: 6, rx: 7, ry: 2.6 }],
      ['path', { d: 'M5 6v12c0 1.4 3.1 2.6 7 2.6s7-1.2 7-2.6V6' }],
      ['path', { d: 'M5 12c0 1.4 3.1 2.6 7 2.6s7-1.2 7-2.6' }]
    ]
  },
  platforms: {
    status: 'new',
    meaning: 'platforms: services somewhere else',
    use: 'skills "platforms" (the robot is bots)',
    shapes: [['path', { d: 'M7 18a4.2 4.2 0 0 1 .4-8.4 5.2 5.2 0 0 1 10 1.4A3.6 3.6 0 0 1 17 18z' }]]
  },
  tools: {
    status: 'keep',
    meaning: 'a machine: a shell answers here',
    use: 'skills "machine" · every tool card',
    shapes: [
      ['path', { d: 'M5 7l5 5-5 5' }],
      ['path', { d: 'M12.5 17H19' }]
    ]
  },
  curation: {
    status: 'new',
    meaning: 'curation: a pile narrowed to a set',
    use: 'skills "curation"',
    shapes: [['path', { d: 'M4 5h16l-6 7.5V19l-4 2v-8.5z' }]]
  },
  declined: {
    status: 'new',
    meaning: 'declined: a proposal turned down',
    use: 'skills "declined" (was the bug mark)',
    shapes: [
      ['circle', { cx: 12, cy: 12, r: 8.2 }],
      ['path', { d: 'M12 8v7M9 12.5l3 3 3-3' }]
    ]
  },
  gate: {
    status: 'new',
    meaning: 'the gate: a threshold with a keeper',
    use: 'calibration "the gate" and "eval corpus" (moving to a target)',
    shapes: [
      ['rect', { x: 6, y: 3.5, width: 12, height: 17, rx: 1.5 }],
      ['circle', { cx: 14.5, cy: 12.5, r: 1 }],
      ['path', { d: 'M4 20.5h16' }]
    ]
  },
  spark: {
    status: 'new',
    meaning: 'a proposal: an idea put forward',
    use: 'calibration "propose" · the offers',
    shapes: [
      ['path', { d: 'M12 3.5l1.7 4.8 4.8 1.7-4.8 1.7L12 16.5l-1.7-4.8L5.5 10l4.8-1.7z' }],
      ['path', { d: 'M18.2 15.2l.7 2 2 .7-2 .7-.7 2-.7-2-2-.7 2-.7z' }]
    ]
  },
  eval: {
    status: 'new',
    meaning: 'an eval run: aimed at a score',
    use: 'calibration "eval corpus"',
    shapes: [
      ['circle', { cx: 12, cy: 12, r: 8.2 }],
      ['circle', { cx: 12, cy: 12, r: 4.2 }],
      ['circle', { cx: 12, cy: 12, r: 1 }]
    ]
  },
  recall: {
    status: 'new',
    meaning: 'a recall hit: the corpus was read',
    use: 'calibration "recall hit"',
    shapes: [
      ['circle', { cx: 10.5, cy: 10.5, r: 5.8 }],
      ['path', { d: 'M15 15l4.5 4.5' }]
    ]
  },
  consolidate: {
    status: 'new',
    meaning: 'consolidation: two facts become one',
    use: 'calibration "consolidate"',
    shapes: [
      ['path', { d: 'M4 6h5l3 5 3-5h5' }],
      ['path', { d: 'M12 11v9M9 17l3 3 3-3' }]
    ]
  },
  screen: {
    status: 'new',
    meaning: 'a tool screen: a call judged and held',
    use: 'calibration "tool screen"',
    shapes: [['path', { d: 'M12 3.5l7 2.8v5.4c0 4.4-2.9 7.6-7 9-4.1-1.4-7-4.6-7-9V6.3z' }]]
  },
  question: {
    status: 'new',
    meaning: 'an explanation: the question asked of a run',
    use: 'calibration "explain" (the bubble alone is a turn)',
    shapes: [
      ['path', { d: 'M21 14.5a2 2 0 0 1-2 2H8l-4 4V5a2 2 0 0 1 2-2h13a2 2 0 0 1 2 2z' }],
      ['path', { d: 'M9.8 8.6a2.3 2.3 0 0 1 4.4.8c0 1.5-2.2 1.8-2.2 3' }],
      ['path', { d: 'M12 14.6h.01' }]
    ]
  },
  verify: {
    status: 'new',
    meaning: 'verified: a claim checked',
    use: 'calibration "verify" and "route judged"',
    shapes: [
      ['circle', { cx: 12, cy: 12, r: 8.2 }],
      ['path', { d: 'M8 12.3l2.6 2.6L16 9.4' }]
    ]
  },
  timing: {
    status: 'new',
    meaning: 'a measured interval',
    use: 'calibration "embed timing" (was the bug mark)',
    shapes: [
      ['circle', { cx: 12, cy: 13, r: 6.5 }],
      ['path', { d: 'M12 9.5V13l2.2 1.4' }],
      ['path', { d: 'M9.5 3.5h5M12 3.5v3' }]
    ]
  },
  cycle: {
    status: 'new',
    meaning: 'a lifecycle: it begins, ends, begins',
    use: 'calibration "lifecycle" (was the bug mark)',
    shapes: [
      ['path', { d: 'M20 12a8 8 0 0 1-13.7 5.6M4 12a8 8 0 0 1 13.7-5.6' }],
      ['path', { d: 'M17.7 3.2v3.5h-3.5M6.3 20.8v-3.5h3.5' }]
    ]
  },
  models: {
    status: 'new',
    meaning: 'the model catalog: what a provider serves',
    use: 'settings "models" (the route belongs to the router)',
    shapes: [
      ['rect', { x: 7, y: 7, width: 10, height: 10, rx: 2 }],
      ['path', { d: 'M12 7V3.6M12 20.4V17M7 12H3.6M20.4 12H17' }]
    ]
  },
  roles: {
    status: 'new',
    meaning: 'a role: a named contract a run wears',
    use: 'settings "roles" (the layer stack is skills)',
    shapes: [
      ['rect', { x: 4, y: 6, width: 16, height: 12, rx: 2 }],
      ['circle', { cx: 9, cy: 12, r: 2.2 }],
      ['path', { d: 'M13 11h4M13 14h3' }]
    ]
  },
  trust: {
    status: 'new',
    meaning: 'trust: access granted, not sealed',
    use: 'settings "trust" (was the bug mark; the vault is the closed lock)',
    shapes: [
      ['rect', { x: 5, y: 10, width: 14, height: 10, rx: 2.5 }],
      ['path', { d: 'M8.5 10V7.2a3.5 3.5 0 0 1 6.7-1.3' }],
      ['circle', { cx: 12, cy: 15, r: 1.1 }]
    ]
  },
  schedule: {
    status: 'new',
    meaning: 'a schedule: a day and a time',
    use: 'settings "schedule" (was the bug mark)',
    shapes: [
      ['rect', { x: 4, y: 5.5, width: 16, height: 14.5, rx: 2 }],
      ['path', { d: 'M4 10h16M8 3.5v4M16 3.5v4' }],
      ['circle', { cx: 12, cy: 14.5, r: 1.6, fill: 'currentColor', stroke: 'none' }]
    ]
  },
  interface: {
    status: 'new',
    meaning: 'the interface: the shape of a page',
    use: 'settings "interface" (was the bug mark)',
    shapes: [
      ['rect', { x: 4, y: 4.5, width: 16, height: 15, rx: 2 }],
      ['path', { d: 'M9.5 4.5v15M9.5 9h10.5' }]
    ]
  },
  environment: {
    status: 'new',
    meaning: 'the environment: the names a process reads',
    use: 'settings "environment" (the robot is bots)',
    shapes: [
      ['circle', { cx: 8, cy: 12, r: 3.5 }],
      ['path', { d: 'M11.5 12H20M17 12v3M20 12v2.5' }]
    ]
  },
  thinking: {
    status: 'redraw',
    meaning: 'the thought before the answer',
    use: 'a thought row in the transcript',
    was: [
      ['path', { d: 'M12 3.5 20.5 12 12 20.5 3.5 12z' }],
      ['circle', { cx: 12, cy: 12, r: 1.8 }]
    ],
    shapes: [['path', { d: 'M12 3.5 20.5 12 12 20.5 3.5 12z' }]]
  },
  theme: {
    status: 'new',
    meaning: 'the theme: light and dark at once',
    use: 'the topbar theme control',
    shapes: [
      ['circle', { cx: 12, cy: 12, r: 8.2 }],
      ['path', { d: 'M12 3.8a8.2 8.2 0 0 1 0 16.4z', fill: 'currentColor', 'fill-opacity': '0.22' }]
    ]
  },
  dot: {
    status: 'redraw',
    meaning: 'a state dot',
    use: 'state marks (the unknown state no longer shares it)',
    was: [['circle', { cx: 12, cy: 12, r: 6 }]],
    shapes: [['circle', { cx: 12, cy: 12, r: 2.5, fill: 'currentColor', stroke: 'none' }]]
  },
  unknown: {
    status: 'new',
    meaning: 'a key that is not drawn — a defect made visible',
    use: 'the icon() fallback (was the same mark as "everything else")',
    shapes: [
      ['circle', { cx: 12, cy: 12, r: 8.2, 'stroke-dasharray': '2.6 2.4' }],
      ['path', { d: 'M9.8 9.4a2.3 2.3 0 0 1 4.4.8c0 1.5-2.2 1.8-2.2 3' }],
      ['path', { d: 'M12 16h.01' }]
    ]
  },
  tower: {
    status: 'delete',
    meaning: 'declared, never used — removed',
    use: 'nothing',
    shapes: [
      ['path', { d: 'M5 20V10' }],
      ['path', { d: 'M12 20V4' }],
      ['path', { d: 'M19 20v-7' }],
      ['path', { d: 'M3 20h18' }]
    ]
  }
};

const DESTINATIONS = [
  ['chat', 'conversation'],
  ['design', 'design'],
  ['sessions', 'sessions'],
  ['huddles', 'huddles'],
  ['router', 'router'],
  ['delegation', 'delegation'],
  ['memory', 'memory'],
  ['skills', 'skills'],
  ['second-brain', 'second brain'],
  ['workspace-history', 'workspace history'],
  ['bots', 'bots'],
  ['vault', 'vault'],
  ['settings', 'settings'],
  ['calibration', 'calibration'],
  ['plugins', 'plugins']
];

const RAILS = [
  { name: 'memory', why: 'the fact corpus, by type', rows: [
    ['pending', 'needs you'], ['person', 'user'], ['rules', 'feedback'],
    ['repo', 'project'], ['memory', 'reference'], ['events', 'episodic'],
    ['other', 'everything else'], ['retired', 'retired'], ['archive', 'archive'],
    ['today', 'today']
  ] },
  { name: 'skills', why: 'the library, by category', rows: [
    ['pending', 'needs you'], ['plan', 'building'], ['web', 'web'], ['pen', 'design'],
    ['media', 'media'], ['documents', 'documents'], ['data', 'data'],
    ['platforms', 'platforms'], ['tools', 'machine'], ['other', 'everything else'],
    ['declined', 'declined'], ['curation', 'curation'], ['retired', 'retired']
  ] },
  { name: 'calibration', why: 'the day, by operation', rows: [
    ['gate', 'the gate'], ['router', 'route'], ['verify', 'route judged'],
    ['chat', 'turn'], ['delegation', 'delegation'], ['spark', 'propose'],
    ['eval', 'eval corpus'], ['recall', 'recall hit'], ['consolidate', 'consolidate'],
    ['screen', 'tool screen'], ['question', 'explain'], ['verify', 'verify'],
    ['timing', 'embed timing'], ['cycle', 'lifecycle']
  ] },
  { name: 'settings', why: 'the home, by section', rows: [
    ['models', 'models'], ['delegation', 'specialists'], ['roles', 'roles'],
    ['trust', 'trust'], ['schedule', 'schedule'], ['theme', 'interface'],
    ['environment', 'environment']
  ] }
];

const BOTS = [
  ['designer', 'makes pages, keeps their identity'],
  ['scholar', 'grounds every claim'],
  ['presenter', 'makes the deck'],
  ['clerk', 'keeps the ledger'],
  ['advocate', 'hunts the role'],
  ['auditor', 'checks what changed']
];

// --------------------------------------------------------------- rendering

const NS = 'http://www.w3.org/2000/svg';

function svgFor(name, size) {
  const spec = ICONS[name] || ICONS.unknown;
  const svg = document.createElementNS(NS, 'svg');
  svg.setAttribute('viewBox', '0 0 24 24');
  svg.setAttribute('fill', 'none');
  svg.setAttribute('stroke', 'currentColor');
  svg.setAttribute('stroke-width', '1.6');
  svg.setAttribute('stroke-linecap', 'round');
  svg.setAttribute('stroke-linejoin', 'round');
  svg.setAttribute('width', String(size));
  svg.setAttribute('height', String(size));
  svg.setAttribute('aria-hidden', 'true');
  (spec.shapes || []).forEach(([tag, attrs]) => {
    const shape = document.createElementNS(NS, tag);
    Object.keys(attrs).forEach((k) => shape.setAttribute(k, attrs[k]));
    svg.appendChild(shape);
  });
  return svg;
}

function el(tag, cls, text) {
  const made = document.createElement(tag);
  if (cls) made.className = cls;
  if (text !== undefined) made.textContent = text;
  return made;
}

function renderSheet() {
  const grid = document.getElementById('grid');
  if (!grid) return;
  Object.keys(ICONS).forEach((key) => {
    const spec = ICONS[key];
    const card = el('article', 'icon-card');
    card.dataset.status = spec.status;
    const head = el('div', 'icon-head');
    head.appendChild(el('span', 'icon-key', key));
    head.appendChild(el('span', 'icon-status', spec.status));
    card.appendChild(head);

    const sizes = el('div', 'icon-sizes');
    [13, 14, 17, 18].forEach((size) => {
      const cell = el('span', 'icon-size');
      cell.appendChild(svgFor(key, size));
      cell.appendChild(el('span', 'icon-size-n', String(size)));
      sizes.appendChild(cell);
    });
    card.appendChild(sizes);

    if (spec.was) {
      const was = el('div', 'icon-was');
      was.appendChild(el('span', 'icon-was-label', 'was'));
      [13, 17].forEach((size) => {
        const cell = el('span', 'icon-size was');
        const svg = document.createElementNS(NS, 'svg');
        svg.setAttribute('viewBox', '0 0 24 24');
        svg.setAttribute('fill', 'none');
        svg.setAttribute('stroke', 'currentColor');
        svg.setAttribute('stroke-width', '1.6');
        svg.setAttribute('stroke-linecap', 'round');
        svg.setAttribute('stroke-linejoin', 'round');
        svg.setAttribute('width', String(size));
        svg.setAttribute('height', String(size));
        spec.was.forEach(([tag, attrs]) => {
          const shape = document.createElementNS(NS, tag);
          Object.keys(attrs).forEach((k) => shape.setAttribute(k, attrs[k]));
          svg.appendChild(shape);
        });
        cell.appendChild(svg);
        cell.appendChild(el('span', 'icon-size-n', String(size)));
        was.appendChild(cell);
      });
      card.appendChild(was);
    }

    card.appendChild(el('p', 'icon-meaning', spec.meaning));
    card.appendChild(el('p', 'icon-use', spec.use));
    grid.appendChild(card);
  });
}

function railRow(key, label) {
  const row = el('button', 'rail-row');
  const mark = el('span', 'rail-mark');
  mark.appendChild(svgFor(key, 17));
  row.appendChild(mark);
  row.appendChild(el('span', 'rail-label', label));
  return row;
}

function renderRails() {
  const main = document.getElementById('main-rail');
  if (!main) return;
  DESTINATIONS.forEach(([key, label]) => main.appendChild(railRow(key, label)));

  const host = document.getElementById('rails');
  RAILS.forEach((rail) => {
    const col = el('section', 'rail-col');
    const head = el('header', 'rail-head');
    head.appendChild(el('h3', null, rail.name));
    head.appendChild(el('p', 'rail-why', rail.why));
    col.appendChild(head);
    rail.rows.forEach(([key, label]) => col.appendChild(railRow(key, label)));
    host.appendChild(col);
  });

  const bots = document.getElementById('bots-rail');
  BOTS.forEach(([name, voice]) => {
    const row = el('button', 'rail-row');
    const mark = el('span', 'rail-mark pmark');
    mark.textContent = name.charAt(0);
    row.appendChild(mark);
    const text = el('span', 'rail-bot');
    text.appendChild(el('span', 'rail-label', name));
    text.appendChild(el('span', 'rail-voice', voice));
    row.appendChild(text);
    bots.appendChild(row);
  });
  bots.appendChild(railRow('bots', 'channels'));
}

function renderToggle() {
  const full = document.getElementById('toggle-full');
  if (!full) return;
  ['full', 'icons'].forEach((mode) => {
    const host = document.getElementById('toggle-' + mode);
    const head = el('div', 'rail-head-bar');
    const brand = el('div', 'rail-brand');
    const glyph = el('span', 'rail-glyph', 'f');
    brand.appendChild(glyph);
    const text = el('span', 'rail-brand-text');
    text.appendChild(el('span', 'rail-brand-name', 'fleet'));
    text.appendChild(el('span', 'rail-brand-tag', 'local console'));
    brand.appendChild(text);
    head.appendChild(brand);
    const toggle = el('button', 'rail-toggle');
    toggle.title = mode === 'full' ? 'collapse the surfaces rail' : 'expand the surfaces rail';
    toggle.setAttribute('aria-label', toggle.title);
    const chev = el('span', 'rail-toggle-mark');
    chev.appendChild(svgFor('chevron', 16));
    toggle.appendChild(chev);
    head.appendChild(toggle);
    host.appendChild(head);
    host.dataset.mode = mode;
  });
}

// The toggle's own chevron: one drawing, rotated by its mode.
ICONS.chevron = {
  status: 'new',
  meaning: 'the rail collapses and expands',
  use: 'the rail head (icon only, no label)',
  shapes: [['path', { d: 'M14.5 5.5L8 12l6.5 6.5' }]]
};

renderSheet();
renderRails();
renderToggle();

// The 4× read: the drawings most at risk at 17px, shown at 68 so the eye
// judges pixels rather than intent.
const ZOOM = ['workspace-history', 'delegation', 'bots', 'plan', 'calibration',
  'thinking', 'data', 'platforms', 'schedule', 'models', 'cycle', 'unknown'];

function renderZoom() {
  const host = document.getElementById('zoom');
  if (!host) return;
  ZOOM.forEach((key) => {
    const cell = el('div', 'zoom-cell');
    cell.appendChild(svgFor(key, 68));
    cell.appendChild(el('span', null, key));
    host.appendChild(cell);
  });
}

renderZoom();
