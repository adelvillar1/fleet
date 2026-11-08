window.addEventListener('load', function () {
  var img = document.getElementById('shot');
  var c = document.createElement('canvas');
  c.width = img.naturalWidth; c.height = img.naturalHeight;
  var ctx = c.getContext('2d');
  ctx.drawImage(img, 0, 0);
  var d = ctx.getImageData(0, 0, c.width, c.height).data;
  var dark = 0, total = d.length / 4, colors = {};
  for (var i = 0; i < d.length; i += 4) {
    var l = 0.299 * d[i] + 0.587 * d[i+1] + 0.114 * d[i+2];
    if (l < 128) dark++;
    colors[(d[i] >> 4) + ',' + (d[i+1] >> 4) + ',' + (d[i+2] >> 4)] = 1;
  }
  var out = document.getElementById('out');
  out.textContent = 'ink:' + (100 * dark / total).toFixed(1) + '% colors:' + Object.keys(colors).length + ' size:' + c.width + 'x' + c.height;
});
