/* The UX map study's shell: the redesign's topbar, the MAP rail (three
   work-kind regions + the door + a system foot), the stage, and an
   optional type-aware drawer. The rail model rides body[data-rail="A|B"];
   the theme rides ?theme= or localStorage, the redesign's own rule. */

(function () {
  function initialTheme() {
    var q = new URLSearchParams(window.location.search).get('theme');
    if (q) return q;
    try { return window.localStorage.getItem('fleet-theme-2') || 'night'; }
    catch (e) { return 'night'; }
  }
  document.documentElement.dataset.theme = initialTheme();

  window.addEventListener('DOMContentLoaded', function () {
    var body = document.body;
    if (body.dataset.shell === 'off') {
      if (new URLSearchParams(window.location.search).get('switch') !== 'off') {
        mountThemeSwitch(document.body, true, true);
      }
      return;
    }
    var stage = body.querySelector('[data-stage]');
    var drawer = body.querySelector('[data-drawer]');
    var current = body.dataset.current || '';
    var rail = document.createElement('nav');
    rail.className = 'rail map';
    rail.setAttribute('aria-label', 'the map');

    var shell = document.createElement('div');
    shell.className = 'shell' + (drawer ? '' : ' no-drawer');

    var top = document.createElement('header');
    top.className = 'topbar';
    top.innerHTML =
      '<button class="iconbtn" id="railToggle" aria-label="toggle the rail">' + I('rail') + '</button>' +
      '<div class="brand"><span class="brand-mark">f</span>fleet</div>' +
      '<div class="search">' + I('search') + ' search or jump <span class="kbd">⌘K</span></div>' +
      '<div class="who"><span class="chip ok">home live</span><span class="persona"><span class="disc tint-1">A</span>adelvillar</span></div>';

    rail.innerHTML = railHtml(body.dataset.rail || 'A', current);

    shell.appendChild(top);
    shell.appendChild(rail);
    if (stage) { stage.classList.add('stage'); shell.appendChild(stage); }
    if (drawer) { shell.appendChild(drawer); }
    body.appendChild(shell);

    var toggle = shell.querySelector('#railToggle');
    toggle && toggle.addEventListener('click', function () { shell.classList.toggle('rail-open'); });
    // `?switch=off` hides the study's theme control: captures must show
    // the design, not the control that switches it.
    if (new URLSearchParams(window.location.search).get('switch') !== 'off') mountThemeSwitch();
  });

  // --- the map --------------------------------------------------------------
  // The door is attention's entrance, not a region: one row, the held
  // count. Then the three work-kind regions; the system strip at the foot.
  function railHtml(model, current) {
    var lanes = model === 'B' ? placementB() : placementA();
    // The bracket marks the region that holds the current lane.
    lanes.forEach(function (region) {
      region.here = region.rows.some(function (row) { return row.id === current; });
    });
    var h = '';
    h += '<a class="map-door' + (current === 'inbox' ? ' current' : '') + '" href="#">' +
      I('inbox') + '<span class="label">the door</span>' +
      '<span class="count">2 held</span></a>';
    h += '<div class="map-regions">';
    lanes.forEach(function (region) {
      h += '<section class="map-region' + (region.here ? ' here' : '') + '">' +
        '<header>' + I(region.mark) + '<span class="t">' + region.t + '</span>' +
        (region.why ? '<span class="why">' + region.why + '</span>' : '') + '</header>';
      region.rows.forEach(function (row) {
        var cls = 'rail-row' + (row.id === current ? ' current' : '');
        h += '<a class="' + cls + '" href="#">' + I(row.mark) + '<span>' + row.t + '</span>' +
          (row.count ? '<span class="count">' + row.count + '</span>' : '') +
          (row.dot ? '<span class="dot ' + row.dot + '" aria-hidden="true"></span>' : '') +
          '</a>';
      });
      h += '</section>';
    });
    h += '</div>';
    h += '<div class="rail-sep"></div>' +
      '<div class="map-system"><div class="rail-label">system</div><div class="syschips">' +
      ['Health', 'Versions', 'Secrets', 'Plugins', 'Assistants', 'Settings']
        .map(function (name) { return '<span class="syschip">' + name + '</span>'; }).join('') +
      '</div></div>';
    h += '<div class="rail-foot">night · expert off · ⌘K</div>';
    return h;
  }

  // Placement A — by the work the region serves.
  function placementA() {
    return [
      { t: 'software', mark: 'code', rows: [
        { id: 'sessions', t: 'Conversations', mark: 'chat', dot: 'needs' },
        { id: 'board', t: 'Board', mark: 'board', count: '3' },
        { id: 'calibration', t: 'Reviews', mark: 'calibration' },
        { id: 'armada', t: 'Fleet', mark: 'armada' }
      ] },
      { t: 'design', mark: 'design', rows: [
        { id: 'design', t: 'Design', mark: 'design', dot: 'working' },
        { id: 'desk', t: 'Desk', mark: 'desk', count: '1' }
      ] },
      { t: 'knowledge', mark: 'brain', rows: [
        { id: 'memory', t: 'Intelligence', mark: 'memory' },
        { id: 'learning', t: 'Learning', mark: 'learning' }
      ] }
    ];
  }

  // Placement B — by the nature of the evidence.
  function placementB() {
    return [
      { t: 'software', mark: 'code', rows: [
        { id: 'sessions', t: 'Conversations', mark: 'chat', dot: 'needs' },
        { id: 'board', t: 'Board', mark: 'board', count: '3' },
        { id: 'armada', t: 'Fleet', mark: 'armada' }
      ] },
      { t: 'design', mark: 'design', rows: [
        { id: 'design', t: 'Design', mark: 'design', dot: 'working' }
      ] },
      { t: 'knowledge', mark: 'brain', rows: [
        { id: 'memory', t: 'Intelligence', mark: 'memory' },
        { id: 'learning', t: 'Learning', mark: 'learning' },
        { id: 'desk', t: 'Desk', mark: 'desk', count: '1' },
        { id: 'calibration', t: 'Reviews', mark: 'calibration' }
      ] }
    ];
  }

  function mountThemeSwitch(host, inline, topCenter) {
    var bar = document.createElement('div');
    bar.className = 'theme-switch';
    if (inline && host) {
      bar.style.cssText = 'position:static;transform:none;justify-content:center;display:flex;gap:4px;padding:8px;';
      host.appendChild(bar);
    } else if (topCenter) {
      bar.style.cssText = 'position:fixed;top:10px;left:50%;transform:translateX(-50%);z-index:90;display:flex;gap:4px;';
      document.body.appendChild(bar);
    } else {
      document.body.appendChild(bar);
    }
    ['night', 'day', 'console', 'contrast'].forEach(function (t) {
      var b = document.createElement('button');
      b.textContent = t;
      b.dataset.t = t;
      b.className = 'syschip';
      if ((document.documentElement.dataset.theme || 'night') === t) {
        b.style.borderColor = 'var(--accent)';
        b.style.color = 'var(--accent-ink)';
      }
      b.addEventListener('click', function () {
        document.documentElement.dataset.theme = t;
        try { window.localStorage.setItem('fleet-theme-2', t); } catch (e) {}
        Array.prototype.forEach.call(bar.children, function (one) {
          one.style.borderColor = '';
          one.style.color = '';
        });
        b.style.borderColor = 'var(--accent)';
        b.style.color = 'var(--accent-ink)';
      });
      bar.appendChild(b);
    });
    return bar;
  }

  var P = '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">';
  function I(name) {
    var d = {
      rail: '<path d="M4 6h16M4 12h16M4 18h16"/>',
      search: '<circle cx="11" cy="11" r="7"/><path d="M20 20l-3.5-3.5"/>',
      chat: '<path d="M21 12a8 8 0 1 1-3.2-6.4L21 5l-.6 3.4A8 8 0 0 1 21 12z"/><path d="M8 12h.01M12 12h.01M16 12h.01"/>',
      board: '<rect x="3" y="4" width="5" height="16" rx="1.5"/><rect x="10" y="4" width="5" height="10" rx="1.5"/><rect x="17" y="4" width="4" height="14" rx="1.5"/>',
      desk: '<path d="M4 20V9l8-5 8 5v11"/><path d="M9 20v-6h6v6"/>',
      memory: '<circle cx="12" cy="12" r="3"/><path d="M12 2v4M12 18v4M2 12h4M18 12h4M5 5l2.5 2.5M16.5 16.5L19 19M19 5l-2.5 2.5M7.5 16.5L5 19"/>',
      learning: '<path d="M12 3 2 8l10 5 10-5-10-5z"/><path d="M6 10.5V16c0 1.5 2.7 3 6 3s6-1.5 6-3v-5.5"/>',
      calibration: '<path d="M9 3h6M12 3v6"/><path d="M7 21a7 7 0 0 1 10 0"/><circle cx="12" cy="14" r="5"/>',
      inbox: '<path d="M22 12h-6l-2 3h-4l-2-3H2"/><path d="M5 5h14l3 7v7H2v-7l3-7z"/>',
      health: '<path d="M3 12h4l2-6 4 12 2-6h6"/>',
      armada: '<circle cx="5" cy="6" r="2"/><circle cx="19" cy="6" r="2"/><circle cx="12" cy="18" r="2"/><path d="M6.5 7.5 10.5 16M17.5 7.5 13.5 16M7 6h10"/>',
      design: '<circle cx="12" cy="12" r="9"/><path d="M12 3a9 9 0 0 0 0 18c1.5 0 2-1 2-2s-1-1.6-1-2.6c0-1.2 1-2 2.4-2H18a3 3 0 0 0 3-3c0-4.5-4-8.4-9-8.4z"/>',
      brain: '<path d="M9 4a3 3 0 0 0-3 3 3.5 3.5 0 0 0-1 6.8V16a4 4 0 0 0 4 4h1V4H9z"/><path d="M15 4a3 3 0 0 1 3 3 3.5 3.5 0 0 1 1 6.8V16a4 4 0 0 1-4 4h-1V4h1z"/>',
      code: '<path d="M8 6 3 12l5 6"/><path d="M16 6l5 6-5 6"/>'
    }[name] || '<circle cx="12" cy="12" r="9" stroke-dasharray="3 3"/>';
    return P + d + '</svg>';
  }
})();
