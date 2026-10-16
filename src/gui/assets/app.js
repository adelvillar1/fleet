// The seam's client. It reads the launch token out of the URL once,
// strips it from the address bar, and holds the stream open. The token
// never leaves this tab: the HttpOnly cookie the page arrived with
// carries it for the requests that follow.
(function () {
  'use strict';

  var params = new URLSearchParams(window.location.search);
  var token = params.get('token');
  var link = document.getElementById('link');
  var stream = document.getElementById('stream');
  var face = document.getElementById('face');
  var beats = document.getElementById('beats');
  var count = 0;

  function state(next, text) {
    link.setAttribute('data-state', next);
    link.textContent = next;
    stream.textContent = text;
  }

  if (!token) {
    state('down', 'This page was opened without its token. Use the URL fleet printed, then reload.');
    return;
  }
  window.history.replaceState(null, '', window.location.pathname);

  var source = new EventSource('/events?token=' + encodeURIComponent(token));

  source.onopen = function () {
    state('connecting', 'The stream is open, waiting for the first beat.');
  };

  source.onmessage = function (event) {
    var data = JSON.parse(event.data);
    if (data.type === 'hello') {
      face.textContent = data.gui;
      state('live', 'The face is live.');
      return;
    }
    if (data.type === 'heartbeat') {
      count += 1;
      beats.textContent = String(count);
    }
  };

  source.onerror = function () {
    state('down', 'The stream dropped. fleet gui is no longer answering.');
  };
})();
