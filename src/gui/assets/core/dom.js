// core/dom.js — the pure element helpers (P2).
//
// The face builds every card, row and graph mark from these three. They
// depend on nothing but the document and their arguments, so they live in
// their own module: one home for the face's element grammar.

export function node(name, className, text) {
  var made = document.createElement(name);
  if (className) made.className = className;
  if (text != null) made.textContent = text;
  return made;
}

export function svg(name, attrs, said) {
  var made = document.createElementNS('http://www.w3.org/2000/svg', name);
  Object.keys(attrs || {}).forEach(function (key) { made.setAttribute(key, attrs[key]); });
  if (said != null) made.textContent = said;
  return made;
}

export function clip(text, most) {
  var said = String(text == null ? '' : text);
  return said.length > most ? said.slice(0, most - 1) + '\u2026' : said;
}
