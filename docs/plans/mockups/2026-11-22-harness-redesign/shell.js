/* The mockup shell: one topbar + rail for every screen.
   Each screen declares data-current and provides [data-stage] and
   optionally [data-drawer]; this script mounts the grid around them.
   The theme rides ?theme=day or localStorage['fleet-theme-2']. */

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
    // Screens that carry their own chrome (the bot's phone and iPad, the
    // console) declare data-shell="off": the theme switcher rides, the
    // desktop topbar and rail do not.
    if (body.dataset.shell === 'off') {
      // screens with their own chrome name the switch's home: 'top' floats
      // it top-center (the phone and the console's default), 'inline-sidebar'
      // seats it in their own sidebar foot (the iPad).
      var mode = body.dataset.themeSwitch || 'top';
      if (mode === 'inline-sidebar') {
        mountThemeSwitch(document.getElementById('side-foot'), true);
      } else {
        mountThemeSwitch(null, false, true);
      }
      return;
    }
    var stage = body.querySelector('[data-stage]');
    var drawer = body.querySelector('[data-drawer]');
    var current = body.dataset.current || '';
    var title = body.dataset.title || 'fleet';
    var say = body.dataset.say || '';

    var shell = document.createElement('div');
    shell.className = 'shell' + (drawer ? '' : ' no-drawer');

    // --- topbar
    var top = document.createElement('header');
    top.className = 'topbar';
    top.innerHTML =
      '<button class="iconbtn" id="railToggle" aria-label="toggle the rail">' + I('rail') + '</button>' +
      '<div class="brand"><span class="brand-mark">f</span>fleet</div>' +
      '<div class="search">' + I('search') + ' search or jump <span class="kbd">⌘K</span></div>' +
      '<div class="who"><span class="chip ok">home live</span><span class="persona"><span class="disc tint-1">A</span>adelvillar</span></div>';

    // --- rail
    var rail = document.createElement('nav');
    rail.className = 'rail';
    rail.setAttribute('aria-label', 'surfaces');
    rail.innerHTML = railHtml(current);

    // --- assemble
    shell.appendChild(top);
    shell.appendChild(rail);
    if (stage) { stage.classList.add('stage'); shell.appendChild(stage); }
    if (drawer) { shell.appendChild(drawer); }
    body.appendChild(shell);

    var toggle = shell.querySelector('#railToggle');
    toggle && toggle.addEventListener('click', function () {
      shell.classList.toggle('rail-open');
    });

    mountThemeSwitch();
  });

  function mountThemeSwitch(host, inline, topCenter) {
    var bar = document.createElement('div');
    bar.className = 'theme-switch';
    if (inline && host) {
      bar.style.cssText = 'position:static;transform:none;justify-content:center;';
      host.appendChild(bar);
    } else if (topCenter) {
      bar.style.cssText = 'top:10px;left:50%;transform:translateX(-50%);bottom:auto;right:auto;';
      document.body.appendChild(bar);
    } else {
      document.body.appendChild(bar);
    }
    ['night', 'day', 'console', 'contrast'].forEach(function (t) {
      var b = document.createElement('button');
      b.textContent = t;
      b.dataset.t = t;
      if ((document.documentElement.dataset.theme || 'night') === t) b.classList.add('on');
      b.addEventListener('click', function () {
        document.documentElement.dataset.theme = t;
        try { window.localStorage.setItem('fleet-theme-2', t); } catch (e) {}
        bar.querySelectorAll('button').forEach(function (x) { x.classList.toggle('on', x === b); });
      });
      bar.appendChild(b);
    });
    return bar;
  }

  // The rail: state groups first (needs you / working — read off the
  // needs-you document in production), then the nine lenses, then Advanced.
  function railHtml(current) {
    function row(address, label, mark, count, tone, sub) {
      var cls = 'rail-row' + (address === current ? ' current' : '') + (tone ? ' tone-' + tone : '');
      var c = count ? '<span class="count">' + count + '</span>' : '';
      return '<a class="' + cls + '" href="#">' + I(mark) + '<span>' + label + '</span>' + c + '</a>';
    }
    var h = '';
    h += '<div class="rail-group">' +
      '<div class="rail-label">needs you</div>' +
      row('board', 'Board', 'board', '3', 'needs') +
      row('inbox', 'Inbox', 'inbox', '2', 'needs') +
      row('sessions', 'Conversations', 'chat', '1', 'needs') +
      '</div>';
    h += '<div class="rail-group">' +
      '<div class="rail-label">working</div>' +
      row('desk', 'Desk', 'desk', '1', 'working') +
      row('design', 'Design', 'design', '1', 'working') +
      '</div>';
    h += '<div class="rail-sep"></div>';
    h += '<div class="rail-group">' +
      '<div class="rail-label">fleet</div>' +
      row('sessions', 'Conversations', 'chat') +
      row('board', 'Board', 'board') +
      row('desk', 'Desk', 'desk') +
      row('memory', 'Intelligence', 'memory') +
      row('learning', 'Learning', 'learning') +
      row('calibration', 'Reviews', 'calibration') +
      row('inbox', 'Inbox', 'inbox') +
      row('second-brain', 'Health', 'health') +
      row('armada', 'Fleet', 'armada') +
      '</div>';
    h += '<div class="rail-group">' +
      '<div class="rail-label">advanced</div>' +
      row('settings', 'Settings', 'settings') +
      row('bots', 'Assistants', 'bots') +
      row('workspace-history', 'Versions', 'layerfs') +
      row('vault', 'Secrets', 'vault') +
      row('plugins', 'Plugins', 'plugins') +
      '</div>';
    h += '<div class="rail-foot">night · expert off · ⌘K</div>';
    return h;
  }

  // One drawing per concept, 24 viewBox, 1.6 stroke, round caps — the
  // face's icon law, restated in the new ground.
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
      settings: '<circle cx="12" cy="12" r="3"/><path d="M19 12a7 7 0 0 0-.2-1.6l2-1.5-2-3.5-2.4 1a7 7 0 0 0-2.7-1.6L13 2h-2l-.7 2.8a7 7 0 0 0-2.7 1.6l-2.4-1-2 3.5 2 1.5A7 7 0 0 0 5 12c0 .5.1 1.1.2 1.6l-2 1.5 2 3.5 2.4-1a7 7 0 0 0 2.7 1.6L11 22h2l.7-2.8a7 7 0 0 0 2.7-1.6l2.4 1 2-3.5-2-1.5c.1-.5.2-1.1.2-1.6z"/>',
      bots: '<rect x="5" y="8" width="14" height="11" rx="3"/><path d="M12 8V4M9 13h.01M15 13h.01"/>',
      layerfs: '<path d="M12 3 3 8l9 5 9-5-9-5z"/><path d="M3 13l9 5 9-5"/>',
      vault: '<rect x="4" y="4" width="16" height="16" rx="3"/><circle cx="12" cy="12" r="3.5"/><path d="M12 8.5v-2M12 17.5v-2M8.5 12h-2M17.5 12h-2"/>',
      plugins: '<path d="M9 3v4M15 3v4M7 7h10a2 2 0 0 1 2 2v4a6 6 0 0 1-6 6h-2a6 6 0 0 1-6-6V9a2 2 0 0 1 2-2z"/>',
      design: '<circle cx="12" cy="12" r="9"/><path d="M12 3a9 9 0 0 0 0 18c1.5 0 2-1 2-2s-1-1.6-1-2.6c0-1.2 1-2 2.4-2H18a3 3 0 0 0 3-3c0-4.5-4-8.4-9-8.4z"/>',
      check: '<path d="M20 6 9 17l-5-5"/>',
      route: '<circle cx="6" cy="6" r="2.5"/><circle cx="18" cy="18" r="2.5"/><path d="M8 7.5c6 1.5 6 7.5 7.5 9"/>',
      key: '<circle cx="8" cy="15" r="4"/><path d="M11 12 21 3M17 6l2.5 2.5M14 8l2 2"/>',
      doc: '<path d="M6 3h8l4 4v14H6z"/><path d="M14 3v4h4"/>',
      pin: '<path d="M12 21s-6-5.5-6-10a6 6 0 1 1 12 0c0 4.5-6 10-6 10z"/><circle cx="12" cy="11" r="2"/>',
      send: '<path d="M22 2 11 13"/><path d="M22 2 15 22l-4-9-9-4 20-7z"/>',
      clock: '<circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 3"/>',
      user: '<circle cx="12" cy="8" r="4"/><path d="M4 21c1.5-3.5 4.5-5 8-5s6.5 1.5 8 5"/>',
      brain: '<path d="M9 4a3 3 0 0 0-3 3 3.5 3.5 0 0 0-1 6.8V16a4 4 0 0 0 4 4h1V4H9z"/><path d="M15 4a3 3 0 0 1 3 3 3.5 3.5 0 0 1 1 6.8V16a4 4 0 0 1-4 4h-1V4h1z"/>'
    }[name] || '<circle cx="12" cy="12" r="9" stroke-dasharray="3 3"/><path d="M12 8v5M12 16h.01"/>';
    return P + d + '</svg>';
  }
})();
