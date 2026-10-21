# The documents lane

Local document extraction for the composer's attachment path. One file in,
one JSON object out; no network, no key, no bill.

```
python3 extract.py --path <file>
{"ok": true, "parser": "local:docx", "markdown": "...", "pages": null}
{"ok": false, "code": "empty", "reason": "pdftotext found no text; the PDF is probably a scan"}
```

## What it reads

| Format | Engine | Notes |
|---|---|---|
| `.docx`, `.docm` | MarkItDown when installed, else the OOXML body | paragraphs and tables; macros are never executed |
| `.pptx`, `.pptm` | MarkItDown when installed, else the OOXML slides | slide text and speaker notes |
| `.xlsx`, `.xlsm` | MarkItDown when installed, else the OOXML sheets | every sheet as a Markdown table; formulas show `=f → value` |
| `.doc`, `.xls`, `.ppt` | LibreOffice `--safe-mode` | converted to the modern format, then read as above |
| `.pdf` | `pdftotext -layout` | a PDF with no text layer returns `empty`, which the caller escalates |
| `.csv` | Python's `csv` | as a Markdown table |
| `.html`, `.htm` | Python's `html.parser` | text only; scripts and styles are dropped |
| `.md`, `.txt`, `.log`, `.xml`, `.json` | raw read | text as-is |

Anything else returns `unsupported`. Images, email, audio, video, Visio
and Parquet are the hosted parser's case; the Rust side escalates them
without calling this lane.

## Bounds

- 64 MB per file (`MAX_BYTES`).
- 120 seconds per extraction, enforced by the caller (`src/documents.rs`);
  the child is killed and the refusal names the timeout.
- A missing lane or tool is a named refusal (`missing_tool`), never a
  crash. A malformed container is `unsupported`.

## Tools and licences

| Tool | Licence | Used for |
|---|---|---|
| MarkItDown | MIT | optional, when its package is importable |
| python-docx / openpyxl / python-pptx | MIT | not required; the stdlib OOXML readers cover the fixtures |
| LibreOffice | MPL-2.0 | legacy `.doc/.xls/.ppt`, `--safe-mode` |
| poppler `pdftotext` | GPL-2.0 | PDF text layer, invoked as a separate program |
| Tesseract | Apache-2.0 | not used here; scans escalate to Perception |

The lane prefers a MarkItDown install when one exists and falls back to
its own OOXML readers otherwise, so a fresh machine needs no Python
packages beyond the standard library.

## Fixtures

`fixtures/` holds what the Rust tests read: `simple.docx`, `simple.pptx`,
`simple.xlsx`, `simple.csv`, `simple.html`, `text.pdf`, `blank.pdf` (no
text layer) and `pixel.png` (an escalation case). They are small,
hand-built OOXML containers plus two PDFs converted by LibreOffice.

## Why not a Rust crate

The plan keeps this lane Python because the tools it wraps are already
Python's or the system's, and the Rust side only needs the decision and
the JSON. A Rust extractor crate is out of scope; if one is ever added,
the contract above is the seam it has to keep.
