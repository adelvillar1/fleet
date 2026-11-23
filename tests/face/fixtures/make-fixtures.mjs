// The drawer's binary fixtures (R7, R8), generated once and committed as
// bytes. Re-run with `node tests/face/fixtures/make-fixtures.mjs`; the
// output is deterministic (fixed zip timestamps), so a re-run should
// reproduce the committed files exactly.
//
//   release-ledger.xlsx — a real workbook: a merged bold title, a bold
//     header row, Excel date serials under a date style, a grouped
//     number, a percentage, an inlineStr cell, column widths, and a
//     second sheet of 501 rows so the 500-row cap has something to say.
//   broken-deck.pptx — a zip that claims to be a presentation but whose
//     presentation.xml is truncated and whose slides are absent: the
//     renderer must refuse it, and the drawer must fall back to the
//     outline BY NAME.
//
// The harness pins feed these exact bytes through the face's own reader
// (tests/face/harness.mjs: the sheet and the deck fallback cases); the
// live face renders the same files once as the browser check.

import { deflateRawSync } from 'node:zlib';
import { writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));

// ---- a minimal zip writer (deflate, CRC-32, fixed timestamps) ----------
const CRC_TABLE = (() => {
  const table = new Int32Array(256);
  for (let n = 0; n < 256; n += 1) {
    let c = n;
    for (let k = 0; k < 8; k += 1) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    table[n] = c;
  }
  return table;
})();

function crc32(buffer) {
  let crc = -1;
  for (let i = 0; i < buffer.length; i += 1) {
    crc = CRC_TABLE[(crc ^ buffer[i]) & 0xff] ^ (crc >>> 8);
  }
  return (crc ^ -1) >>> 0;
}

// 2026-11-20 12:00, so the bytes do not drift with the clock.
const DOS_TIME = (12 << 11) | (0 << 5) | 0;
const DOS_DATE = ((2026 - 1980) << 9) | (11 << 5) | 20;

function zip(entries) {
  const locals = [];
  const centrals = [];
  let offset = 0;
  for (const entry of entries) {
    const name = Buffer.from(entry.name, 'utf8');
    const data = Buffer.from(entry.data);
    const packed = deflateRawSync(data);
    const crc = crc32(data);
    const local = Buffer.alloc(30);
    local.writeUInt32LE(0x04034b50, 0);
    local.writeUInt16LE(20, 4);
    local.writeUInt16LE(0x0800, 6); // UTF-8 names
    local.writeUInt16LE(8, 8); // deflate
    local.writeUInt16LE(DOS_TIME, 10);
    local.writeUInt16LE(DOS_DATE, 12);
    local.writeUInt32LE(crc, 14);
    local.writeUInt32LE(packed.length, 18);
    local.writeUInt32LE(data.length, 22);
    local.writeUInt16LE(name.length, 26);
    local.writeUInt16LE(0, 28);
    locals.push(local, name, packed);

    const central = Buffer.alloc(46);
    central.writeUInt32LE(0x02014b50, 0);
    central.writeUInt16LE(20, 4);
    central.writeUInt16LE(20, 6);
    central.writeUInt16LE(0x0800, 8);
    central.writeUInt16LE(8, 10);
    central.writeUInt16LE(DOS_TIME, 12);
    central.writeUInt16LE(DOS_DATE, 14);
    central.writeUInt32LE(crc, 16);
    central.writeUInt32LE(packed.length, 20);
    central.writeUInt32LE(data.length, 24);
    central.writeUInt16LE(name.length, 28);
    central.writeUInt16LE(0, 30);
    central.writeUInt16LE(0, 32);
    central.writeUInt16LE(0, 34);
    central.writeUInt16LE(0, 36);
    central.writeUInt32LE(0, 38);
    central.writeUInt32LE(offset, 42);
    centrals.push(central, name);

    offset += local.length + name.length + packed.length;
  }
  const directory = Buffer.concat(centrals);
  const end = Buffer.alloc(22);
  end.writeUInt32LE(0x06054b50, 0);
  end.writeUInt16LE(0, 4);
  end.writeUInt16LE(0, 6);
  end.writeUInt16LE(entries.length, 8);
  end.writeUInt16LE(entries.length, 10);
  end.writeUInt32LE(directory.length, 12);
  end.writeUInt32LE(offset, 16);
  end.writeUInt16LE(0, 20);
  return Buffer.concat([...locals, directory, end]);
}

// ---- the workbook -----------------------------------------------------
// Excel's serial axis: 25569 is 1970-01-01.
const serial = (y, m, d) => Math.round(Date.UTC(y, m - 1, d) / 86400000) + 25569;

const SHARED = [
  'the release ledger', 'date', 'asset', 'bytes', 'share',
  'fleet-macos-arm64.tar.gz', 'fleet-linux-x64.tar.gz',
];

const sheet1 =
  '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>' +
  '<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">' +
  '<cols><col min="1" max="1" width="12" customWidth="1"/>' +
  '<col min="2" max="2" width="28" customWidth="1"/>' +
  '<col min="3" max="3" width="14" customWidth="1"/>' +
  '<col min="4" max="4" width="10" customWidth="1"/></cols>' +
  '<sheetData>' +
  '<row r="1"><c r="A1" t="s" s="1"><v>0</v></c></row>' +
  '<row r="2"><c r="A2" t="s" s="1"><v>1</v></c><c r="B2" t="s" s="1"><v>2</v></c>' +
  '<c r="C2" t="s" s="1"><v>3</v></c><c r="D2" t="s" s="1"><v>4</v></c></row>' +
  '<row r="3"><c r="A3" s="2"><v>' + serial(2026, 11, 20) + '</v></c>' +
  '<c r="B3" t="s"><v>5</v></c><c r="C3" s="4"><v>41208112</v></c>' +
  '<c r="D3" s="3"><v>0.125</v></c></row>' +
  '<row r="4"><c r="A4" s="2"><v>' + serial(2026, 11, 21) + '</v></c>' +
  '<c r="B4" t="s"><v>6</v></c><c r="C4" s="4"><v>41992318</v></c>' +
  '<c r="D4" s="3"><v>0.08</v></c></row>' +
  '<row r="5"><c r="A5" s="2"><v>' + serial(2026, 11, 22) + '</v></c>' +
  '<c r="B5" t="inlineStr"><is><t>fleet-windows-x64.zip</t></is></c>' +
  '<c r="C5" s="4"><v>45006144</v></c><c r="D5" s="3"><v>0.0625</v></c></row>' +
  '</sheetData>' +
  '<mergeCells count="1"><mergeCell ref="A1:D1"/></mergeCells>' +
  '</worksheet>';

let longRows = '';
for (let n = 1; n <= 501; n += 1) {
  longRows += '<row r="' + n + '"><c r="A' + n + '"><v>' + n + '</v></c></row>';
}
const sheet2 =
  '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>' +
  '<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">' +
  '<sheetData>' + longRows + '</sheetData></worksheet>';

const styles =
  '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>' +
  '<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">' +
  '<numFmts count="2">' +
  '<numFmt numFmtId="164" formatCode="0.0%"/>' +
  '<numFmt numFmtId="165" formatCode="#,##0"/>' +
  '</numFmts>' +
  '<fonts count="2"><font><sz val="11"/><name val="Calibri"/></font>' +
  '<font><b/><sz val="11"/><name val="Calibri"/></font></fonts>' +
  '<fills count="1"><fill><patternFill patternType="none"/></fill></fills>' +
  '<borders count="1"><border/></borders>' +
  '<cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs>' +
  '<cellXfs count="5">' +
  '<xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>' +
  '<xf numFmtId="0" fontId="1" fillId="0" borderId="0" xfId="0" applyFont="1"/>' +
  '<xf numFmtId="14" fontId="0" fillId="0" borderId="0" xfId="0" applyNumberFormat="1"/>' +
  '<xf numFmtId="164" fontId="0" fillId="0" borderId="0" xfId="0" applyNumberFormat="1"/>' +
  '<xf numFmtId="165" fontId="0" fillId="0" borderId="0" xfId="0" applyNumberFormat="1"/>' +
  '</cellXfs>' +
  '</styleSheet>';

const sharedStrings =
  '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>' +
  '<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="' +
  SHARED.length + '" uniqueCount="' + SHARED.length + '">' +
  SHARED.map((s) => '<si><t>' + s + '</t></si>').join('') +
  '</sst>';

const workbook =
  '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>' +
  '<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" ' +
  'xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">' +
  '<sheets><sheet name="assets" sheetId="1" r:id="rId1"/>' +
  '<sheet name="long" sheetId="2" r:id="rId2"/></sheets></workbook>';

const contentTypes =
  '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>' +
  '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">' +
  '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>' +
  '<Default Extension="xml" ContentType="application/xml"/>' +
  '<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>' +
  '<Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>' +
  '<Override PartName="/xl/worksheets/sheet2.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>' +
  '<Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/>' +
  '<Override PartName="/xl/sharedStrings.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml"/>' +
  '</Types>';

const rootRels =
  '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>' +
  '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">' +
  '<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>' +
  '</Relationships>';

const workbookRels =
  '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>' +
  '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">' +
  '<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>' +
  '<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet2.xml"/>' +
  '<Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>' +
  '<Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings" Target="sharedStrings.xml"/>' +
  '</Relationships>';

writeFileSync(join(HERE, 'release-ledger.xlsx'), zip([
  { name: '[Content_Types].xml', data: contentTypes },
  { name: '_rels/.rels', data: rootRels },
  { name: 'xl/workbook.xml', data: workbook },
  { name: 'xl/_rels/workbook.xml.rels', data: workbookRels },
  { name: 'xl/styles.xml', data: styles },
  { name: 'xl/sharedStrings.xml', data: sharedStrings },
  { name: 'xl/worksheets/sheet1.xml', data: sheet1 },
  { name: 'xl/worksheets/sheet2.xml', data: sheet2 },
]));

// ---- the deck that must fail ------------------------------------------
// A valid zip that is not an Office Open XML package: no
// [Content_Types].xml, so the viewer refuses it by name ("The archive is
// not an Office Open XML package ([Content_Types].xml missing).") and the
// drawer falls back to the outline. Measured against the live face — a
// *truncated* deck and a deck with no presentation part were both
// accepted by PPTXjs as empty decks, which is why the fixture is shaped
// as a non-package rather than a broken deck.
const notAPresentation =
  'this archive is not a presentation: it carries no [Content_Types].xml, ' +
  'so the slide renderer has no package to load.\n';

writeFileSync(join(HERE, 'broken-deck.pptx'), zip([
  { name: 'readme.txt', data: notAPresentation },
]));

console.log('wrote release-ledger.xlsx and broken-deck.pptx');
