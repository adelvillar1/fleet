window.addEventListener('load', function () {
  var lum = function (c) {
    var m = c.match(/rgba?\(([^)]+)\)/); if (!m) return 1;
    var p = m[1].split(',').map(function (x) { return parseFloat(x); });
    var a = p.length > 3 ? p[3] : 1;
    var f = function (v) { v /= 255; return v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4); };
    return (0.2126 * f(p[0]) + 0.7152 * f(p[1]) + 0.0722 * f(p[2])) * a + (1 - a);
  };
  var ratio = function (fg, bg) { var l1 = lum(fg), l2 = lum(bg); if (l1 < l2) { var t = l1; l1 = l2; l2 = t; } return (l1 + 0.05) / (l2 + 0.05); };
  var els = document.querySelectorAll('.mock-row, .mock-node, .mock-prop, .mock-section, .mock-pos');
  var min = 99, overflow = 0;
  els.forEach(function (el) {
    var cs = getComputedStyle(el);
    var bg = cs.backgroundColor === 'rgba(0, 0, 0, 0)' ? getComputedStyle(document.body).backgroundColor : cs.backgroundColor;
    min = Math.min(min, ratio(cs.color, bg));
    if (el.scrollWidth > el.clientWidth + 1) overflow++;
  });
  document.title = 'contrast:' + min.toFixed(2) + ' overflow:' + overflow;
});
