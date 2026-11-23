# The drawer's binary fixtures (R7, R8)

Generated once by `make-fixtures.mjs` and committed as bytes, because a
pin that builds its own input cannot prove the reader against a file the
way a real one arrives. The generator is dependency-free (Node's `zlib`,
CRC-32 and a small zip writer) and deterministic (fixed timestamps):

```
node tests/face/fixtures/make-fixtures.mjs
```

## `release-ledger.xlsx` — the sheet that renders its own form

A real workbook (LibreOffice headless reads it and shows real dates,
grouped numbers and percentages), carrying exactly what R7's pin reads:

- a merged title `A1:D1` (`<mergeCells>`), bold;
- a bold header row;
- Excel date serials under a date style (2026-11-20/21/22);
- a grouped number (`#,##0`) and a percentage (`0.0%`);
- an `inlineStr` cell (`fleet-windows-x64.zip`) as well as shared strings;
- four `<cols>` widths;
- a second sheet, `long`, of 501 rows so the 500-row cap has something to
  say ("the first 500 of 501 rows").

Pin: `theSheetRendersItsOwnForm` in `tests/face/harness.mjs` — the harness
inflates the zip for real (`DecompressionStream`), parses the XML (a
general mini DOM, not a fixture matcher) and reads the rendered table.

## `broken-deck.pptx` — the archive the renderer must refuse

A valid zip that is **not an Office Open XML package**: one `readme.txt`,
no `[Content_Types].xml`. Measured against the live face, PPTXjs reports

> The archive is not an Office Open XML package ([Content_Types].xml missing).

and the drawer lands the text outline naming that reason. Two shapes were
tried first and **rejected as fixtures** because the viewer accepted them
as empty decks (`rendered`, 0 slides): a truncated `presentation.xml`, and
a package with content types but no presentation part. A "failing parse"
fixture has to be one the engine actually refuses.

Pin: `theDeckFallsBackToTheOutlineByName` — the harness mounts the frame,
reads the nonce, drives the frame's `pptx-failed` report (with the reason
the live face gave for these bytes) and asserts the named outline; a second
open drives `pptx-rendered` and asserts the outline is not drawn.

The live record (one probe face, `fleet gui --port 3995`, WKWebView):

- `release-ledger.xlsx` → 17 cells, merged bold title (`colspan=4`), dates
  2026-11-20…22, `41,208,112`, `12.5%` / `8.0%`, four column widths, tabs
  `assets`/`long`, the "formulas are shown as their cached values" line.
- `broken-deck.pptx` → the frame is removed and the outline carries the
  viewer's own refusal text above.
