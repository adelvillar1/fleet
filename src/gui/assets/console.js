// fleet console — the mini face (docs/plans/2026-11-21-mini-console.md).
//
// The owner's question was whether a TUI-shaped experience can ride the
// relay to a phone. The full face answers it expensively: 389 KB gz on the
// wire and 1.45 MB of JavaScript to parse before the first useful pixel.
// This face answers it cheaply: one column of text, one command line, and
// the documents the TUI's panels already read. Nothing here rederives a
// derivation — the board's columns, the delegation overlay and its
// divergence words, and the second brain's morning read are all printed
// from the bridge's own JSON, which is the bridge's own `boards` reads.
//
// The four-sentence law holds in every panel, in this face's voice: what a
// panel says while its read is in flight, when it holds nothing, when the
// read refused, and what is selected. A panel never prints a bare zero in
// place of a sentence, and a refusal always names what refused.
//
// The press rule holds too: every press acts and says what happened — a
// tapped row echoes its command, the read prints its own receipt, and a
// read that failed says so by name instead of going quiet.
'use strict';

// ------------------------------------------------------------ the column
//
// Wrapping is the face's own, not the browser's: `white-space: pre` means
// a line is a line, and a paragraph is a stack of lines this program
// chose. The fill is terminal-grade — a word too long for the column is
// broken at the column rather than allowed to run past it, and no
// character is ever dropped.

const MIN_COLS = 24;
const MAX_COLS = 96;

/// Fill `text` into lines no wider than `cols`, with a prefix chosen per
/// line. One implementation, so a paragraph and an indented row wrap by
/// the same law.
function fill(text, cols, prefixFor) {
  // The floor is only there so a caller cannot ask for a column no word
  // fits in; the face's own floor (MIN_COLS) is applied where the screen's
  // width is measured, not here, so a test can wrap at any width it likes.
  const width = Math.max(8, cols | 0);
  const out = [];
  // A newline is a paragraph boundary and stays one: a terminal never
  // joins two paragraphs to save a row.
  const fillOne = (paragraph) => {
    const before = out.length;
    let line = '';
    const room = () => Math.max(6, width - prefixFor(out.length).length);
    const flush = () => {
      out.push(prefixFor(out.length) + line);
      line = '';
    };
    const words = String(paragraph).replace(/\s+/g, ' ').trim().split(' ');
    for (const word of words) {
      if (!word) continue;
      if (line && line.length + 1 + word.length > room()) flush();
      if (word.length > room()) {
        // A token with no space in it (a path, an id, a base64 blob) is
        // broken at the column. The alternative is a horizontal scroll on
        // a phone, which is not a terminal.
        let rest = word;
        while (rest.length > room()) {
          out.push(prefixFor(out.length) + rest.slice(0, room()));
          rest = rest.slice(room());
        }
        line = rest;
        continue;
      }
      line = line ? line + ' ' + word : word;
    }
    if (line || out.length === before) flush();
  };
  String(text == null ? '' : text)
    .replace(/\r\n?/g, '\n')
    .split('\n')
    .forEach(fillOne);
  return out;
}

const wrap = (text, cols) => fill(text, cols, () => '');
const hang = (text, cols, head, rest) => fill(text, cols, (i) => (i ? rest : head));

/// One stored line, wrapped for the column the grid has now. A line's own
/// indentation is kept as the prefix and its continuations hang two spaces
/// further in, so a wrapped card reads as one card and not as two rows.
/// This runs at paint time, which is what makes a rotation re-wrap: the
/// transcript keeps the lines it was given, not the rows it drew.
function wrapLine(text, cols) {
  const said = String(text == null ? '' : text);
  const indent = (said.match(/^ */) || [''])[0].length;
  const pad = ' '.repeat(indent);
  if (said.length <= cols) return [said];
  return hang(said.slice(indent), cols, pad, pad + '  ');
}

/// A string clipped to `n` characters, with the cut marked — never a
/// silent truncation.
function clip(text, n) {
  const said = String(text == null ? '' : text).replace(/\s+/g, ' ').trim();
  return said.length > n ? said.slice(0, Math.max(0, n - 1)) + '\u2026' : said;
}

/// A count with its unit, so a line never reads `1 tasks`.
const many = (n, one, more) => `${n} ${n === 1 ? one : more || one + 's'}`;

/// The queue's own headline: a count that agrees with its verb.
const needYou = (n) => (n === 1 ? '1 thing needs you' : `${n} things need you`);

// -------------------------------------------------------- the sentences
//
// One table, keyed by the panel's own address, in the terminal's voice.
// `tests/console/harness.mjs` pins each of these by name: a wording that
// drifts fails a case that says which sentence it expected. A value that
// carries data is a function of that data; the wording lives here and
// nowhere else.

const SENTENCES = {
  sessions: {
    empty: 'no conversation under this home yet — run something and its log lands here.',
    loading: 'reading the archive\u2026',
    refused: (said) => 'the archive refused: ' + said.join('; ') + '.',
    selected: (d) => 'open: ' + d.title + (d.state ? ' · ' + d.state : '') +
      ' — the transcript below is the whole of it.',
  },
  board: {
    empty: 'no approved plan carries work right now — approve a plan and its tasks land here.',
    loading: 'reading the board\u2026',
    refused: (said) => 'the board refused: ' + said.join('; ') + '.',
    selected: (d) => 'focused: ' + d.plan + ' · ' + many(d.tasks, 'task') +
      ' — every plan is one press away.',
  },
  delegation: {
    empty: 'this home has declared no dependency graph — the graph is a file the plan\u2019s ' +
      'work writes, and nothing here is inferred from what a run did.',
    loading: 'reading the delegation\u2026',
    refused: (said) => 'the delegation refused: ' + said.join('; ') + '.',
    selected: (d) => 'showing ' + d.what + ' · ' + many(d.nodes, 'node') +
      ' — the overlay is the day\u2019s rows, not a guess.',
  },
  secondBrain: {
    empty: 'the second brain has no home on this machine — set SECOND_BRAIN_HOME and reload.',
    loading: 'reading the second brain\u2026',
    refused: (said) => 'the second brain refused: ' + said.join('; ') + '.',
    selected: (d) => 'the morning read · ' + (d.needs === 0 ? 'nothing needs you today'
      : needYou(d.needs)) + ' — the six sections are below.',
  },
};

function sentence(panel, kind, data) {
  const table = SENTENCES[panel];
  const words = table && table[kind];
  if (words == null) return '';
  return typeof words === 'function' ? words(data || {}) : words;
}

// ------------------------------------------------------------- the lines
//
// A line is a row of the grid plus what it means: its weight (`kind`),
// whether it is a press and what the press runs, whether it is the
// selection, and whether it is still being read.

const L = (kind, text, extra) => Object.assign({ kind, text }, extra || {});
const list = (value) => (Array.isArray(value) ? value.filter((row) => row != null) : []);
const num = (value) => (typeof value === 'number' && isFinite(value) ? value : 0);

// ------------------------------------------------------------- the board

function boardLines(doc, opts) {
  const out = [];
  const refused = list(doc.refused).map(String);
  const plans = list(doc.plans);
  const tasks = list(doc.tasks);
  if (!plans.length && !tasks.length) {
    out.push(L('answer', refused.length ? sentence('board', 'refused', refused)
      : sentence('board', 'empty')));
    return out;
  }
  const counts = doc.counts || {};
  const columns = [['queued', 'queued'], ['in-process', 'in process'], ['done', 'done']];
  const inColumn = (key) => plans
    .map((plan) => ({ plan, tasks: list(plan.tasks).filter((task) => task.column === key) }))
    .filter((row) => row.tasks.length > 0);
  const loose = (key) => tasks.filter((task) => task.column === key);

  out.push(L('receipt', [
    many(num(counts.plans), 'plan'),
    many(num(counts.queued) + num(counts['in-process']) + num(counts.done), 'task'),
    `${num(counts.queued)} queued`,
    `${num(counts['in-process'])} in process`,
    `${num(counts.done)} done`,
  ].join(' · ')));

  columns.forEach(([key, label]) => {
    const rows = inColumn(key);
    const adhoc = loose(key);
    const total = rows.reduce((sum, row) => sum + row.tasks.length, 0) + adhoc.length;
    out.push(L('heading', `\u25b8 ${label} · ${total}`, { gap: true }));
    if (!total) {
      // An empty column says so in words. `0` is a number dressed as a
      // reading, and the law forbids it.
      out.push(L('faint', '  nothing in this column yet'));
      return;
    }
    rows.forEach((row) => {
      out.push(L('answer', `  ${row.plan.plan} · ${many(row.tasks.length, 'task')} · ` +
        `${row.plan.status || 'no state word'}`, { press: { cmd: '/board', arg: row.plan.id } }));
    });
    if (adhoc.length) {
      out.push(L('answer', `  ad-hoc work · ${many(adhoc.length, 'task')} · the owner\u2019s own`));
    }
  });

  // The selection. A named plan that is not on the board is a refusal by
  // name — the console never quietly focuses something else.
  const focused = opts.focus
    ? plans.find((plan) => plan.id === opts.focus)
    : plans[0];
  if (opts.focus && !focused) {
    out.push(L('refusal', `no plan on this board is named \`${opts.focus}\` — /board lists them.`));
  }
  if (focused) {
    const own = list(focused.tasks);
    out.push(L('heading', `\u25b8 ${focused.plan} · ${many(own.length, 'task')} · ` +
      `${focused.status || 'no state word'}`, { gap: true, selected: true }));
    if (!own.length) out.push(L('faint', '  this plan declares no task yet'));
    own.forEach((task) => {
      const waiting = task.waiting_on ? ` — waiting on ${task.waiting_on}` : '';
      const reason = task.reason ? ` (${clip(task.reason, 60)})` : '';
      out.push(L('answer', `  ${task.id} · ${clip(task.title, 68)} · ${task.column}` +
        (task.state ? `/${task.state}` : '') + waiting + reason));
    });
    out.push(L('note', sentence('board', 'selected', { plan: focused.plan, tasks: own.length }),
      { sentence: true, gap: true }));
  }
  refused.forEach((said) => out.push(L('refusal', said, { gap: true })));
  return out;
}

// -------------------------------------------------------- the delegation

/// One delegation tree as text: the child, its depth as indentation, and
/// the words the log carries about it.
function delegationTree(node, depth, out) {
  if (!node) return;
  const pad = '  '.repeat(Math.min(depth, 6));
  const said = [node.id, node.kind || 'run', node.status || 'no state word'];
  if (num(node.turns)) said.push(many(num(node.turns), 'turn'));
  if (num(node.compactions)) said.push(many(num(node.compactions), 'compaction'));
  if (node.stop_reason) said.push(node.stop_reason);
  out.push(L('answer', pad + said.join(' · ')));
  if (node.goal) out.push(L('faint', pad + '  ' + clip(node.goal, 96)));
  list(node.children).forEach((child) => delegationTree(child, depth + 1, out));
}

/// The overlay tile and the graph, printed by name. The divergence words
/// are the read's own (`never spawned`, `ran twice`); the face adds no
/// fifth word and softens none of them.
function delegationLines(doc, opts) {
  const out = [];
  const refused = list(doc.refused).map(String);
  const graph = doc.graph || null;
  const live = doc.live || null;
  const history = list(doc.history);
  if (!graph && !live && !history.length) {
    out.push(L('answer', refused.length ? sentence('delegation', 'refused', refused)
      : sentence('delegation', 'empty')));
    return out;
  }

  if (graph) {
    const divergences = graph.divergences || {};
    const never = num(divergences.never_spawned);
    const twice = num(divergences.ran_twice);
    // The overlay tile: the two words, counted, and — when both are zero —
    // the sentence those zeros mean, so the tile is never a strip of
    // numbers standing in for a reading.
    out.push(L('heading', `\u25b8 the overlay · never spawned ${never} · ran twice ${twice}` +
      (never || twice ? '' : ' · the rows and the graph agree')));
    out.push(L('faint', '  ' + (divergences.basis || 'the day\u2019s delegation rows')));
    const nodes = list(graph.nodes);
    const workload = nodes.map((node) => node.declared_workload).find(Boolean);
    if (workload) out.push(L('note', '  declared workload: ' + workload));

    const wanted = opts.focus || (live && live.id) || '';
    const focusedNode = nodes.find((node) => node.id === wanted) || nodes[0];
    if (opts.focus && !nodes.find((node) => node.id === opts.focus)) {
      out.push(L('refusal', `no node in this graph is named \`${opts.focus}\` — ` +
        '/delegation lists them.'));
    }

    out.push(L('heading', `\u25b8 the declared graph · ${many(nodes.length, 'node')}` +
      (num(graph.revisions) ? ` · ${many(num(graph.revisions), 'revision')}` : ''), { gap: true }));
    if (!nodes.length) out.push(L('faint', '  the graph declares no node'));
    nodes.forEach((node) => {
      const overlay = node.overlay || null;
      const said = [node.id, node.status || 'no state word'];
      if (node.state && node.state !== node.status) said.push(node.state);
      if (num(node.cost)) said.push(`cost ${num(node.cost)}`);
      // The divergence word, by name, on the node it belongs to.
      if (overlay && overlay.divergence) said.push(overlay.divergence);
      const extra = { press: { cmd: '/delegation', arg: node.id } };
      if (focusedNode && node.id === focusedNode.id) extra.selected = true;
      out.push(L(overlay && overlay.divergence ? 'warn' : 'answer', '  ' + said.join(' · '), extra));
    });

    // The why-this-child line: the child's judged shape, the provider that
    // served it, and how it ended — the read's own three fields, in the
    // order the panel reads them.
    if (focusedNode && focusedNode.overlay) {
      const overlay = focusedNode.overlay;
      const shape = [overlay.brief_shape, overlay.provider ? 'on ' + overlay.provider : '',
        overlay.outcome ? '— ' + overlay.outcome : ''].filter(Boolean).join(' ');
      const spawned = num(overlay.spawn_count);
      out.push(L('note', '  why this child: ' + (shape || 'the rows name no shape for it') +
        (spawned ? ` · ${many(spawned, 'child', 'children')} spawned` : ''), { gap: true }));
      if (list(overlay.spawned_sessions).length) {
        out.push(L('faint', '  spawned: ' + list(overlay.spawned_sessions).join(', ')));
      }
    }
    if (graph.critical_path) out.push(L('faint', '  critical path: ' + clip(graph.critical_path, 90)));
    if (graph.blocks_most_work) {
      out.push(L('faint', '  blocks most work: ' + clip(graph.blocks_most_work, 90)));
    }
    const weak = graph.weakness;
    if (weak && weak.sentence) out.push(L('warn', '  weakest link: ' + clip(weak.sentence, 96)));
  }

  if (live) {
    out.push(L('heading', '\u25b8 the run in flight', { gap: true }));
    delegationTree(live, 0, out);
  }
  if (history.length) {
    out.push(L('heading', `\u25b8 history · ${many(history.length, 'run')}`, { gap: true }));
    history.forEach((node) => delegationTree(node, 0, out));
  }
  if (doc.depth_cap != null) out.push(L('faint', `  depth cap ${doc.depth_cap}`, { gap: true }));

  out.push(L('note', sentence('delegation', 'selected', {
    what: live ? 'the run in flight' : 'the declared graph',
    nodes: graph ? list(graph.nodes).length : (live ? 1 : history.length),
  }), { sentence: true, gap: true }));
  refused.forEach((said) => out.push(L('refusal', said, { gap: true })));
  return out;
}

// ------------------------------------------------------- the second brain

/// The morning read, derived the way the TUI's panel derives it: five
/// lines over one document, each line the read's own words and a refused
/// section named rather than counted as zero.
function morningLines(doc) {
  const out = [];
  const attention = doc.attention || null;
  const reports = doc.reports || null;
  const harness = doc.harness || null;
  const loops = doc.loops || null;
  const tower = doc.tower || null;
  const towerSaid = () => (tower && tower.reason) || 'the tower section did not land';

  if (attention) {
    const counts = attention.counts || {};
    out.push(L('answer', num(attention.total) > 0
      ? `${needYou(num(attention.total))} — ` +
        `${num(counts.drift)} drift · ${num(counts.stale)} stale · ` +
        `${many(num(counts.handoff), 'handoff')} · ${many(num(counts.conflicts), 'conflict')}`
      : 'nothing needs you today'));
  } else {
    out.push(L('faint', towerSaid()));
  }

  if (attention) {
    const first = list(attention.drift)[0] || list(attention.stale)[0] ||
      list(attention.handoff)[0] || list(attention.conflicts)[0];
    const said = first
      ? [first.project, first.what, first.evidence].filter((part) => part && String(part).trim())
        .join(' · ')
      : '';
    out.push(L('note', said ? 'first: ' + said : 'nothing is queued first'));
  } else {
    out.push(L('faint', towerSaid()));
  }

  if (reports) {
    const doing = num(reports.doing_total);
    const pending = num(reports.pending_verification);
    const open = num(reports.open_conflicts);
    out.push(L('answer', doing || pending || open
      ? `${doing} in flight · ${pending} awaiting verification · ${open} conflicts open`
      : 'nothing in flight'));
  } else {
    out.push(L('faint', 'the reports section did not land — its refusal is named below'));
  }

  if (harness) {
    const last = list(harness.recent)[0];
    out.push(L('note', last ? `${last.ts} ${last.kind}: ${last.label} — ${last.outcome}`
      : 'no harness events yet'));
  } else {
    out.push(L('faint', 'the harness section did not land — its refusal is named below'));
  }

  if (loops) {
    const owed = list(loops.loops).reduce((sum, row) => sum + num(row.pending_review), 0);
    const finding = list(loops.findings)[0];
    out.push(L('note', finding ? String(finding) : owed > 0
      ? `${many(owed, 'loop iteration')} await review` : 'the loops are quiet'));
  } else {
    out.push(L('faint', 'the loops section did not land — its refusal is named below'));
  }
  return out;
}

function secondBrainLines(doc) {
  const out = [];
  const refused = list(doc.refused).map(String);
  const landed = ['gates', 'harness', 'loops', 'reports', 'maturity', 'tower']
    .filter((name) => doc[name]);
  if (!landed.length) {
    out.push(L('answer', refused.length ? sentence('secondBrain', 'refused', refused)
      : sentence('secondBrain', 'empty')));
    return out;
  }
  out.push(L('heading', '\u25b8 the morning read'));
  morningLines(doc).forEach((line, index) => {
    out.push(Object.assign({}, line, { text: '  ' + line.text, gap: index === 0 }));
  });

  const section = (name, counts, rows) => {
    const body = doc[name];
    if (!body) {
      out.push(L('faint', `\u25b8 ${name} · not on this read`, { gap: true }));
      return;
    }
    out.push(L('heading', `\u25b8 ${name} · ${counts(body)}`, { gap: true }));
    rows(body).slice(0, 5).forEach((row) => out.push(L('answer', '  ' + row)));
  };
  section('gates', (gates) => `${num((gates.totals || {}).verdicts)} verdicts · ` +
    `${num((gates.totals || {}).dispositions)} dispositions · ` +
    `${num((gates.totals || {}).events)} events`,
  (gates) => list(gates.board).map((row) => `${row.gate} · ${row.verdicts}v · ${row.graded_progress}`));
  section('harness', (harness) => {
    const runs = num(list(harness.loops).reduce((sum, row) => sum + num(row.runs), 0));
    return runs ? many(runs, 'run') : 'no runs recorded';
  },
    (harness) => list(harness.recent).map((row) => `${row.ts} ${row.kind}: ${row.label} — ${row.outcome}`));
  section('loops', (loops) => `${many(list(loops.loops).length, 'loop')} · ` +
    `${num(list(loops.loops).reduce((sum, row) => sum + num(row.pending_review), 0))} pending review`,
  (loops) => list(loops.loops).map((row) => `${row.name} · ${row.family} · ${row.verdict} · ` +
    `${num(row.iterations)} iterations`));
  section('reports', (reports) => `${num(reports.doing_total)} in flight · ` +
    `${num(reports.pending_verification)} awaiting verification · ${num(reports.open_conflicts)} conflicts`,
  (reports) => list(reports.feed).map((row) => `${row.ts} ${row.kind} · ${clip(row.summary, 80)}`));
  section('maturity', (maturity) => `${many(list(maturity.weeks).length, 'week')} · ` +
    `${many(list(maturity.series).length, 'series', 'series')} · ` +
    `${many(list(maturity.milestones).length, 'milestone')}`,
  (maturity) => list(maturity.series).map((row) => `${row.name} · ${many(list(row.points).length, 'week')}`));
  section('tower', (tower) => `${many(list(tower.projects).length, 'project')} · ` +
    `${many(list(tower.timeline).length, 'day')} of commits`,
  (tower) => list(tower.projects).map((row) => `${row.name} · ${num(row.commits_30d)} commits in 30d · ` +
    `${num(row.dirty)} dirty`));

  if (doc.home) out.push(L('faint', '  home: ' + doc.home, { gap: true }));
  if (doc.read_at) out.push(L('faint', '  read at ' + doc.read_at));
  out.push(L('note', sentence('secondBrain', 'selected', {
    needs: num((doc.attention || {}).total),
  }), { sentence: true, gap: true }));
  refused.forEach((said) => out.push(L('refusal', said, { gap: true })));
  return out;
}

// ------------------------------------------------------------ the archive

function sessionsLines(doc, opts) {
  const out = [];
  const refused = list(doc.refused).map(String);
  const rows = list(doc.sessions);
  if (!rows.length) {
    out.push(L('answer', refused.length ? sentence('sessions', 'refused', refused)
      : sentence('sessions', 'empty')));
    return out;
  }
  const counts = doc.counts || {};
  out.push(L('receipt', [
    many(num(counts.yours), 'conversation'),
    num(counts.needs) === 1 ? '1 needs you' : `${num(counts.needs)} need you`,
    `${num(counts.working)} working`,
    `${num(counts.archived)} archived`,
  ].join(' · ')));
  const shown = opts.limit ? rows.slice(0, opts.limit) : rows;
  shown.forEach((row) => {
    const said = [row.state || 'no state word', clip(row.title || row.id, 62)];
    if (num(row.turns)) said.push(many(num(row.turns), 'turn'));
    if (num(row.compactions)) said.push(many(num(row.compactions), 'compaction'));
    out.push(L('answer', '  ' + said.join(' · '),
      { press: { cmd: '/sessions', arg: row.id }, selected: !!row.current }));
  });
  if (rows.length > shown.length) {
    out.push(L('faint', `  ${rows.length - shown.length} more — /sessions all lists them`));
  }
  const focused = rows.find((row) => row.current) || rows[0];
  out.push(L('note', sentence('sessions', 'selected', {
    title: clip(focused.title || focused.id, 48),
    state: focused.state || '',
  }), { sentence: true, gap: true }));
  refused.forEach((said) => out.push(L('refusal', said, { gap: true })));
  return out;
}

/// One conversation as pure transcript text: the cards the replay read
/// carries, one row each, no markdown and no chrome.
function transcriptLines(doc) {
  const out = [];
  const refused = list(doc.refused).map(String);
  if (!list(doc.cards).length) {
    out.push(L('answer', refused.length ? sentence('sessions', 'refused', refused)
      : 'this conversation carries no message yet — the log has its start record and nothing else.'));
    return out;
  }
  out.push(L('receipt', [doc.id, doc.model || 'no model yet', doc.ended ? 'ended' : 'open',
    doc.stop_reason || '', num(doc.turns) ? many(num(doc.turns), 'turn') : '',
    num(doc.compactions) ? many(num(doc.compactions), 'compaction') : '']
    .filter(Boolean).join(' · ')));
  list(doc.cards).forEach((card) => {
    const kind = card.type || 'row';
    if (kind === 'message') {
      const who = card.role === 'owner' ? 'owner' : (card.persona || 'assistant');
      out.push(L('heading', who + ' \u203a', { gap: true }));
      // The body is one stored line: the paint wraps it for the column the
      // grid has now, so a rotation re-wraps a paragraph instead of
      // leaving it at the width of the screen it was read on.
      String(card.text || '').split(/\n{2,}/).forEach((paragraph) => {
        out.push(L('answer', '  ' + paragraph.replace(/\s+/g, ' ').trim()));
      });
      return;
    }
    if (kind === 'tool_call') out.push(L('faint', `  · ${card.tool}: ${clip(card.summary, 92)}`));
    else if (kind === 'tool_result') {
      out.push(L('faint', `  · ${card.tool} returned: ${clip(card.preview, 92)}`));
    } else if (kind === 'reasoning') out.push(L('faint', `  · thinking (${num(card.seconds)}s): ` +
      clip(card.text, 92)));
    else if (kind === 'plan' || kind === 'goal') out.push(L('note', `  · ${kind}: ` +
      clip(card.text, 96)));
    else if (kind === 'join') out.push(L('faint', `  · ${card.persona} joined as ${card.role}`));
    else if (kind === 'mention') out.push(L('faint', `  · ${card.from} → ${card.member} (${card.kind})`));
    else if (kind === 'lane') out.push(L('faint', `  · lane ${card.lane}: ${num(card.tools)} tools` +
      (card.withdrawn ? ' (withdrawn)' : '')));
    else out.push(L('faint', `  · ${kind}`));
  });
  if (num(doc.omitted)) out.push(L('faint', `  ${many(num(doc.omitted), 'row')} omitted from this read`,
    { gap: true }));
  refused.forEach((said) => out.push(L('refusal', said, { gap: true })));
  return out;
}

// ---------------------------------------------------------- the transport
//
// The relay is inherited from the bot PWA, not rebuilt: a page reached
// through the relay carries the secret in the URL fragment (the part a
// browser never sends), every content-bearing request names the relay in
// a header, and the frame is sealed with the browser's own WebCrypto —
// X25519 through the relay, AES-256-GCM over the derived key. On the
// loopback face there is no stored secret and every line here is inert.

const RELAY_KEY_STORE = 'fleet_relay_k';
const RELAY_HANDSHAKE = '/api/relay/handshake';
const RELAY_OPEN = {
  '/console/': 1, '/console/console.css': 1, '/console/console.js': 1,
  '/console.webmanifest': 1, '/console/console-sw.js': 1,
  '/console-icon-192.png': 1, '/console-icon-512.png': 1, '/pair': 1,
};

const b64 = (bytes) => {
  let text = '';
  for (let i = 0; i < bytes.length; i++) text += String.fromCharCode(bytes[i]);
  return btoa(text);
};
const unb64 = (text) => {
  const raw = atob(text);
  const out = new Uint8Array(raw.length);
  for (let i = 0; i < raw.length; i++) out[i] = raw.charCodeAt(i);
  return out;
};

let relayReady = null;

function relaySecret() {
  try { return window.localStorage.getItem(RELAY_KEY_STORE) || ''; } catch { return ''; }
}

async function relayHandshake(secret) {
  const headers = { 'x-fleet-relay': secret };
  const asked = await fetch(RELAY_HANDSHAKE, { headers, cache: 'no-store' });
  if (!asked.ok) throw new Error(`the handshake did not answer: ${asked.status}`);
  const doc = await asked.json();
  const pair = await crypto.subtle.generateKey({ name: 'X25519' }, true, ['deriveBits']);
  const device = await crypto.subtle.importKey('raw', unb64(doc.public), { name: 'X25519' }, true, []);
  const shared = await crypto.subtle.deriveBits({ name: 'X25519', public: device }, pair.privateKey, 256);
  const hk = await crypto.subtle.importKey('raw', shared, 'HKDF', false, ['deriveBits']);
  const bits = await crypto.subtle.deriveBits({
    name: 'HKDF', hash: 'SHA-256',
    salt: new TextEncoder().encode(doc.sid),
    info: new TextEncoder().encode('fleet relay frame v1'),
  }, hk, 256);
  const key = await crypto.subtle.importKey('raw', bits, 'AES-GCM', false, ['encrypt', 'decrypt']);
  const mine = new Uint8Array(await crypto.subtle.exportKey('raw', pair.publicKey));
  const posted = await fetch(RELAY_HANDSHAKE, {
    method: 'POST',
    headers: Object.assign({ 'Content-Type': 'application/json' }, headers),
    body: JSON.stringify({ public: b64(mine) }),
  });
  if (!posted.ok) throw new Error(`the handshake was refused: ${posted.status}`);
  return { key };
}

async function unseal(key, reply) {
  if (reply.headers.get('x-fleet-enc') !== '1') return reply;
  const frame = reply.headers.get('x-fleet-frame') || '';
  const raw = unb64(String(await reply.text()).trim());
  const plain = await crypto.subtle.decrypt({
    name: 'AES-GCM', iv: raw.slice(0, 12),
    additionalData: new TextEncoder().encode(frame), tagLength: 128,
  }, key, raw.slice(12));
  return new Response(new TextDecoder().decode(plain), {
    status: reply.status, statusText: reply.statusText, headers: reply.headers,
  });
}

function armRelay() {
  if (typeof window === 'undefined' || typeof document === 'undefined') return;
  const secret = relaySecret();
  if (!secret || typeof window.fetch !== 'function') return;
  document.documentElement.setAttribute('data-relay', '1');
  relayReady = relayHandshake(secret).catch((error) => {
    document.documentElement.setAttribute('data-relay-error', '1');
    throw error;
  });
  const raw = window.fetch.bind(window);
  window.fetch = (input, init) => {
    init = init || {};
    const url = new URL(typeof input === 'string' ? input : input.url, location.href);
    if (url.origin !== location.origin) return raw(input, init);
    const method = String(init.method || 'GET').toUpperCase();
    if (method === 'GET' && RELAY_OPEN[url.pathname]) return raw(input, init);
    const headers = Object.assign({}, init.headers || {});
    headers['x-fleet-relay'] = secret;
    if (url.pathname === RELAY_HANDSHAKE) {
      return raw(url.href, Object.assign({}, init, { headers }));
    }
    return relayReady.then(async (seal) => {
      const send = Object.assign({}, init, { headers });
      if (init.body != null) {
        const nonce = crypto.getRandomValues(new Uint8Array(12));
        const sealed = await crypto.subtle.encrypt({
          name: 'AES-GCM', iv: nonce, tagLength: 128,
          additionalData: new TextEncoder().encode(url.pathname + url.search),
        }, seal.key, new TextEncoder().encode(String(init.body)));
        const both = new Uint8Array(nonce.length + sealed.byteLength);
        both.set(nonce, 0);
        both.set(new Uint8Array(sealed), nonce.length);
        headers['x-fleet-enc'] = '1';
        send.body = b64(both);
      }
      return unseal(seal.key, await raw(url.href, send));
    });
  };
}

/// One read. A refusal keeps its own words: the status, or the message the
/// bridge put in the body, so a panel can print what refused instead of
/// going quiet.
async function api(path) {
  const reply = await fetch(path, { cache: 'no-store' });
  const body = await reply.json().catch(() => ({}));
  if (!reply.ok && reply.status !== 409) {
    throw new Error(`${path} answered ${reply.status}` +
      (body && body.error ? ` — ${body.error}` : ''));
  }
  if (body && body.error) throw new Error(body.error);
  return body;
}

// -------------------------------------------------------------- the shell

const HELP = [
  ['/sessions [id|all]', 'the archive: your conversations, one as transcript text'],
  ['/board [plan]', 'the work at execution level, three columns'],
  ['/delegation [node]', 'the declared graph, the day\u2019s overlay, the run in flight'],
  ['/second-brain', 'the morning read and its six sections'],
  ['/clear', 'empty the transcript, not the archive'],
  ['/help', 'this table'],
];

const COMMANDS = ['/sessions', '/board', '/delegation', '/second-brain', '/clear', '/help'];

/// Everything the console knows how to run. A word that is not here is
/// answered by name rather than guessed at.
const RUN = {
  '/help': (ctx) => {
    ctx.say(L('heading', '\u25b8 commands'));
    HELP.forEach(([name, about]) => {
      ctx.say(L('answer', '  ' + name));
      // One stored line per description: the paint wraps it, the same law
      // every other row is wrapped by.
      ctx.say(L('faint', '      ' + about));
    });
    ctx.say(L('faint', 'anything else answers \u201cnot on this surface yet\u201d, by name.'));
  },

  '/clear': (ctx) => ctx.clear(),

  '/board': async (ctx, arg) => {
    const doc = await ctx.read('/api/board', 'board');
    if (!doc) return;
    ctx.paint(boardLines(doc, { focus: arg }));
  },

  '/delegation': async (ctx, arg) => {
    const doc = await ctx.read('/api/delegation', 'delegation');
    if (!doc) return;
    ctx.paint(delegationLines(doc, { focus: arg }));
  },

  '/second-brain': async (ctx) => {
    const doc = await ctx.read('/api/second-brain', 'secondBrain');
    if (!doc) return;
    ctx.paint(secondBrainLines(doc));
  },

  '/sessions': async (ctx, arg) => {
    if (arg && arg !== 'all') {
      const doc = await ctx.read('/api/session?id=' + encodeURIComponent(arg), 'sessions');
      if (!doc) return;
      ctx.paint(transcriptLines(doc));
      return;
    }
    const doc = await ctx.read('/api/sessions', 'sessions');
    if (!doc) return;
    ctx.paint(sessionsLines(doc, { limit: arg === 'all' ? 0 : 12 }));
  },
};

/// The console's own boot. The state is the transcript: an array of lines,
/// repainted whole when the column count changes (a rotation, a window
/// resize), because a wrap is a decision this program made and must be
/// able to make again.
function boot() {
  const root = document.getElementById('app');
  const state = { lines: [], cols: 46, history: [], at: 0 };

  const el = (tag, className, text) => {
    const node = document.createElement(tag);
    if (className) node.className = className;
    if (text != null) node.textContent = text;
    return node;
  };

  const head = el('div', 'term-head');
  const name = el('span', 'term-name');
  name.appendChild(el('b', null, 'fleet'));
  name.appendChild(document.createTextNode(' console'));
  const where = el('span', 'term-where', 'the TUI\u2019s economy, relayed');
  const conn = el('span', 'term-conn', 'local');
  head.appendChild(name);
  head.appendChild(where);
  head.appendChild(conn);

  const view = el('div');
  view.id = 'log';
  view.setAttribute('role', 'log');
  view.setAttribute('aria-live', 'polite');
  view.setAttribute('aria-label', 'console transcript');

  const form = el('form', 'prompt');
  const mark = el('span', 'prompt-mark', '\u203a');
  const input = el('input');
  input.id = 'cmd';
  input.type = 'text';
  input.autocomplete = 'off';
  input.autocapitalize = 'off';
  input.spellcheck = false;
  input.setAttribute('enterkeyhint', 'send');
  input.setAttribute('aria-label', 'command');
  input.placeholder = '/board, /delegation, /second-brain\u2026';
  form.appendChild(mark);
  form.appendChild(input);

  const keys = el('div', 'keys');
  ['/board', '/delegation', '/second-brain', '/sessions', '/help'].forEach((word) => {
    const key = el('button', 'key', word);
    key.type = 'button';
    key.addEventListener('click', () => run(word));
    keys.appendChild(key);
  });

  root.textContent = '';
  root.appendChild(head);
  root.appendChild(view);
  root.appendChild(form);
  root.appendChild(keys);

  /// The column count the grid actually has: measured from the element's
  /// own width and a monospace advance, so the wrap follows the screen
  /// rather than a guess about it.
  function columns() {
    const width = view.clientWidth || (root.clientWidth || 340);
    const step = parseFloat((document.defaultView && document.defaultView
      .getComputedStyle && document.defaultView.getComputedStyle(view).fontSize) || '13') || 13;
    return Math.max(MIN_COLS, Math.min(MAX_COLS, Math.floor(width / (step * 0.6))));
  }

  function row(line) {
    const node = el('div', 'line', line.text);
    node.setAttribute('data-kind', line.kind || 'answer');
    if (line.gap) node.setAttribute('data-gap', '1');
    if (line.selected) node.setAttribute('data-selected', '1');
    if (line.sentence) node.setAttribute('data-sentence', '1');
    if (line.state) node.setAttribute('data-state', line.state);
    if (line.press) {
      node.setAttribute('data-press', '1');
      node.addEventListener('click', () => run(line.press.cmd + (line.press.arg ? ' ' + line.press.arg : '')));
    }
    return node;
  }

  /// Paint stored lines. Each line is wrapped here, once, for the column
  /// the grid has now — the stored text is the program's, the rows are the
  /// screen's, and the two are never confused.
  function paint(entries) {
    let last = null;
    entries.forEach((line) => {
      state.lines.push(line);
      wrapLine(line.text, state.cols).forEach((text, index) => {
        const node = row(Object.assign({}, line, {
          text,
          gap: index === 0 && line.gap,
        }));
        view.appendChild(node);
        last = node;
      });
    });
    view.scrollTop = view.scrollHeight || 0;
    return last;
  }

  function say(line) { return paint([line]); }

  function clear() {
    state.lines = [];
    view.textContent = '';
  }

  function echo(text) {
    state.history.push(text);
    state.at = state.history.length;
    say(L('echo', '\u00bb ' + text));
  }

  /// One read, with its three sentences: the loading line printed before
  /// the fetch, the refusal printed when it fails (by name), and the
  /// settled mark when it lands. The loading sentence is never withdrawn —
  /// it is finished.
  async function read(path, panel) {
    const pending = say(L('note', sentence(panel, 'loading'), { pending: true }));
    const started = Date.now();
    try {
      const doc = await api(path);
      if (pending) pending.setAttribute('data-state', 'settled');
      say(L('receipt', `\u00b7 read in ${Math.max(1, Date.now() - started)} ms · ${path}`));
      return doc;
    } catch (error) {
      if (pending) pending.setAttribute('data-state', 'settled');
      say(L('refusal', sentence(panel, 'refused',
        [error && error.message ? error.message : String(error)])));
      return null;
    }
  }

  const ctx = { say, paint, clear, read, cols: columns };

  async function run(text) {
    const raw = String(text || '').trim();
    if (!raw) return;
    input.value = '';
    echo(raw);
    const [head, ...rest] = raw.split(/\s+/);
    const word = head.toLowerCase();
    const arg = rest.join(' ').trim();
    const handler = RUN[word];
    if (!handler) {
      say(L('refusal', `not on this surface yet — the console says so by name: ${word}. ` +
        'Type /help for the panels it prints.'));
      return;
    }
    form.setAttribute('data-busy', '1');
    try {
      await handler(ctx, arg);
      const relayed = document.documentElement.getAttribute('data-relay') === '1';
      conn.setAttribute('data-state', 'live');
      conn.textContent = relayed ? 'relayed' : 'local';
    } catch (error) {
      conn.setAttribute('data-state', 'down');
      conn.textContent = 'unreachable';
      say(L('refusal', 'the console could not finish: ' +
        (error && error.message ? error.message : String(error))));
    } finally {
      form.removeAttribute('data-busy');
      if (input.focus) input.focus();
    }
  }

  form.addEventListener('submit', (event) => {
    if (event && event.preventDefault) event.preventDefault();
    run(input.value);
  });

  input.addEventListener('keydown', (event) => {
    const key = event.key;
    if (key === 'ArrowUp' && state.history.length) {
      state.at = Math.max(0, state.at - 1);
      input.value = state.history[state.at] || '';
      if (event.preventDefault) event.preventDefault();
    } else if (key === 'ArrowDown' && state.history.length) {
      state.at = Math.min(state.history.length, state.at + 1);
      input.value = state.history[state.at] || '';
      if (event.preventDefault) event.preventDefault();
    } else if (key === 'Tab') {
      // The keyboard's share of the command line: complete the word from
      // the table, so a hardware keyboard never has to type the whole
      // address on a phone-sized screen.
      const typed = String(input.value || '').trim().toLowerCase();
      if (typed && typed.charAt(0) === '/') {
        const hit = COMMANDS.find((word) => word.indexOf(typed) === 0 && word !== typed);
        if (hit) {
          input.value = hit + ' ';
          if (event.preventDefault) event.preventDefault();
        }
      }
    }
  });

  // A rotation or a resize is a new column count, and the transcript is
  // wrapped again from its own lines rather than left at the old width.
  let pendingResize = null;
  const onResize = () => {
    if (pendingResize) return;
    pendingResize = setTimeout(() => {
      pendingResize = null;
      const cols = columns();
      if (cols === state.cols) return;
      state.cols = cols;
      const kept = state.lines;
      view.textContent = '';
      state.lines = [];
      paint(kept);
    }, 120);
  };
  if (window.addEventListener) window.addEventListener('resize', onResize);
  if (window.addEventListener) window.addEventListener('orientationchange', onResize);

  state.cols = columns();
  armRelay();

  // The welcome is the console's own empty state: what it is, and the way
  // forward — never a blank screen with a cursor on it.
  say(L('heading', '\u25b8 fleet console'));
  say(L('answer', 'the TUI\u2019s economy, relayed: one column, one command line,'));
  say(L('answer', 'and the documents the panels already read.'));
  say(L('faint', 'type a command, or tap one below. /help lists them.', { gap: true }));
  if (location.hash && location.hash.length > 2) {
    const asked = location.hash.replace(/^#\/?/, '');
    if (asked) run('/' + asked);
  }
  return { run, state, view, columns };
}

// ---------------------------------------------------------------- the seam
//
// The program is testable without a browser: with no `document` the core
// is handed to the caller (`tests/console/harness.mjs` boots it both ways)
// and nothing touches a page. In the browser the shell is the only entry.
const CORE = { SENTENCES, sentence, fill, wrap, hang, wrapLine, clip, many, L,
  boardLines, delegationLines, secondBrainLines, morningLines, sessionsLines,
  transcriptLines, COMMANDS, HELP, MIN_COLS, MAX_COLS };

if (typeof document === 'undefined') {
  globalThis.__console = CORE;
} else {
  boot();
}
