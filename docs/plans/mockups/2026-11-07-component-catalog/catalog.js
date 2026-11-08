/* The catalog's starter vocabulary and the end-to-end sample. The data here
   is the table the Rust module will own; the render is the mockup's own, so
   the loop is visible before any code. */

const CATALOG = [
  { kind: 'container', group: 'layout', dir: 'column', props: [{ n: 'dir', req: false }, { n: 'gap', req: false }, { n: 'align', req: false }],
    desc: 'A vertical or horizontal stack. Every other component sits inside one.',
    render: () => `<div class="cat-container"><div class="cat-text">first</div><div class="cat-text">second</div></div>` },
  { kind: 'heading', group: 'text', props: [{ n: 'text', req: true }, { n: 'level', req: false }],
    desc: 'A display line. Level 1 is the page title, 2 a section, 3 a subsection.',
    render: () => `<div class="cat-heading">The conclusion first</div>` },
  { kind: 'text', group: 'text', props: [{ n: 'text', req: true }, { n: 'tone', req: false }],
    desc: 'A paragraph. The workhorse; carries the evidence the heading asserts.',
    render: () => `<div class="cat-text">Revenue grew 34% year over year to $12M, and the growth came from the team plan, not the self-serve funnel.</div>` },
  { kind: 'quote', group: 'text', props: [{ n: 'text', req: true }, { n: 'attribution', req: false }],
    desc: 'A pulled quotation with its source.',
    render: () => `<blockquote class="cat-quote">The best design is the one that disappears.<cite>— the owner, on the canvas</cite></blockquote>` },
  { kind: 'callout', group: 'text', props: [{ n: 'title', req: true }, { n: 'body', req: true }, { n: 'tone', req: false }],
    desc: 'A titled note that stands apart from the flow. One tone at a time.',
    render: () => `<div class="cat-callout" data-tone="warn"><div class="cat-callout-title">The number is a snapshot</div><div class="cat-callout-body">Figures are as of Q3 2026 and come from the verified claim set.</div></div>` },
  { kind: 'badge', group: 'text', props: [{ n: 'label', req: true }, { n: 'tone', req: false }],
    desc: 'A short status mark. Solid, never translucent.',
    render: () => `<span class="cat-badge" data-tone="ok">verified</span>` },
  { kind: 'metric', group: 'data', props: [{ n: 'label', req: true }, { n: 'value', req: true }, { n: 'delta', req: false }, { n: 'tone', req: false }],
    desc: 'One number, large, with its label and an optional change.',
    render: () => `<div class="cat-metric"><span class="cat-metric-value">$12M</span><span class="cat-metric-label">Revenue, Q3 2026</span><span class="cat-metric-delta">+34% YoY</span></div>` },
  { kind: 'table', group: 'data', props: [{ n: 'columns', req: true }, { n: 'rows', req: true }],
    desc: 'Rows and columns with one neutral rule colour and hairline dividers.',
    render: () => `<table class="cat-table"><thead><tr><th>Plan</th><th>Seats</th><th>Price</th></tr></thead><tbody><tr><td>Team</td><td>8</td><td>$12,000</td></tr><tr><td>Self-serve</td><td>312</td><td>$4,200</td></tr></tbody></table>` },
  { kind: 'list', group: 'data', props: [{ n: 'items', req: true }, { n: 'ordered', req: false }],
    desc: 'A short list. Each item one line, one point.',
    render: () => `<ul class="cat-list"><li>Team plan drove the growth</li><li>Self-serve churned at 4%</li><li>Expansion outpaced new logos</li></ul>` },
  { kind: 'chart', group: 'data', props: [{ n: 'kind', req: true }, { n: 'series', req: true }, { n: 'labels', req: true }],
    desc: 'A generated chart from a data spec, never hand-drawn geometry.',
    render: () => `<div class="cat-chart"><div class="cat-chart-col"><div class="cat-chart-bar" style="height:46%"></div><span class="cat-chart-label">Q1</span></div><div class="cat-chart-col"><div class="cat-chart-bar" style="height:68%"></div><span class="cat-chart-label">Q2</span></div><div class="cat-chart-col"><div class="cat-chart-bar" style="height:58%"></div><span class="cat-chart-label">Q3</span></div><div class="cat-chart-col"><div class="cat-chart-bar" data-tone="muted" style="height:84%"></div><span class="cat-chart-label">Q4</span></div></div>` },
  { kind: 'timeline', group: 'data', props: [{ n: 'events', req: true }],
    desc: 'A sequence with a dot and a connector; one label and one note each.',
    render: () => `<div class="cat-timeline"><div class="cat-timeline-item"><span class="cat-timeline-dot"></span><div class="cat-timeline-body"><span class="cat-timeline-label">Q1 — the team plan launches</span><span class="cat-timeline-note">Priced at $1,000 per seat.</span></div></div><div class="cat-timeline-item" data-tone="muted"><span class="cat-timeline-dot"></span><div class="cat-timeline-body"><span class="cat-timeline-label">Q3 — expansion outpaces new logos</span><span class="cat-timeline-note">Net revenue retention reaches 118%.</span></div></div></div>` },
  { kind: 'comparison', group: 'data', props: [{ n: 'left', req: true }, { n: 'right', req: true }, { n: 'better', req: false }],
    desc: 'Two sides contrasted. One side may be marked better.',
    render: () => `<div class="cat-comparison"><div class="cat-comparison-side" data-tone="better"><div class="cat-comparison-head">Team plan</div><div class="cat-comparison-body">Higher revenue per account, longer sales cycle.</div></div><div class="cat-comparison-side"><div class="cat-comparison-head">Self-serve</div><div class="cat-comparison-body">Lower revenue per account, no sales cycle.</div></div></div>` },
  { kind: 'image', group: 'media', props: [{ n: 'src', req: true }, { n: 'alt', req: true }, { n: 'aspect', req: false }],
    desc: 'A figure with its alt text. The slot is decided before the crop.',
    render: () => `<div class="cat-image">16 : 9</div>` },
  { kind: 'link', group: 'action', props: [{ n: 'label', req: true }, { n: 'href', req: true }],
    desc: 'An inline jump. Understated, never a button.',
    render: () => `<a class="cat-link" href="#">Read the full report</a>` },
  { kind: 'button', group: 'action', props: [{ n: 'label', req: true }, { n: 'action', req: true }, { n: 'tone', req: false }],
    desc: 'A filled action. One primary per surface.',
    render: () => `<button class="cat-button" type="button">Export the report</button>` },
  { kind: 'form_field', group: 'form', props: [{ n: 'label', req: true }, { n: 'kind', req: true }, { n: 'placeholder', req: false }, { n: 'required', req: false }],
    desc: 'One labelled input. The label is always visible.',
    render: () => `<div class="cat-form-field"><label class="cat-form-label">Workspace</label><input class="cat-form-input" type="text" placeholder="acme-fleet" /></div>` },
];

/* The sample spec — the flat {root, elements} shape the Rust UiSpec parses. */
const SAMPLE_SPEC = {
  root: 'page',
  elements: {
    page: { type: 'container', props: { dir: 'column', gap: 12 }, children: ['head', 'metric', 'body', 'chart', 'cta'] },
    head: { type: 'heading', props: { text: 'Q3 2026 revenue review', level: 1 }, children: [] },
    metric: { type: 'metric', props: { label: 'Revenue, Q3 2026', value: '$12M', delta: '+34% YoY' }, children: [] },
    body: { type: 'text', props: { text: 'Revenue grew 34% year over year to $12M, and the growth came from the team plan, not the self-serve funnel.' }, children: [] },
    chart: { type: 'chart', props: { kind: 'bar', series: [46, 68, 58, 84], labels: ['Q1', 'Q2', 'Q3', 'Q4'] }, children: [] },
    cta: { type: 'button', props: { label: 'Export the report', action: 'export_report', tone: 'primary' }, children: [] },
  },
};

/* The mockup's own renderer — a stand-in for the Rust render, so the loop is
   visible here. The real one emits the same `cat-*` classes. */
function renderElement(spec, id) {
  const el = spec.elements[id];
  if (!el) return '';
  const made = CATALOG.find((c) => c.kind === el.type);
  const inner = made ? made.render() : '';
  const kids = (el.children || []).map((c) => renderElement(spec, c)).join('');
  return el.type === 'container' ? `<div class="cat-container">${inner}${kids}</div>` : inner;
}

function buildSheet() {
  const grid = document.getElementById('grid');
  grid.innerHTML = CATALOG.map((c) => `
    <div class="comp-card">
      <div class="comp-head">
        <span class="comp-kind">${c.kind}</span>
        <span class="comp-group">${c.group}</span>
      </div>
      <div class="comp-props">${c.props.map((p) => `<span class="comp-prop${p.req ? ' req' : ''}">${p.n}${p.req ? '*' : ''}</span>`).join('')}</div>
      <div class="comp-render">${c.render()}</div>
      <div class="comp-desc">${c.desc}</div>
    </div>`).join('');
}

function buildLoop() {
  document.getElementById('spec').textContent = JSON.stringify(SAMPLE_SPEC, null, 2);
  document.getElementById('loop-render').innerHTML = renderElement(SAMPLE_SPEC, SAMPLE_SPEC.root);
}

buildSheet();
buildLoop();
