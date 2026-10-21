#!/usr/bin/env python3
"""Local document extraction for fleet's attachment path.

Reads one document and writes one JSON object to stdout:

    {"ok": true, "parser": "local:docx", "markdown": "...", "pages": null}
    {"ok": false, "code": "empty|missing_tool|unsupported|too_large", "reason": "..."}

The extractor never executes macros, never opens embedded objects, and
never touches the network. Office formats are OOXML zip containers and
are read directly; legacy binary formats go through LibreOffice with
--safe-mode; PDFs go through pdftotext; MarkItDown is used when its
package is importable (it is optional, not required).

Licences of the tools this file may invoke:
  MarkItDown     MIT
  LibreOffice    MPL-2.0
  poppler/pdf*   GPL-2.0 (invoked as a separate program)
  Tesseract      Apache-2.0 (not used here; the Rust side escalates scans)
"""

import argparse
import csv
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import zipfile
from html.parser import HTMLParser
from xml.etree import ElementTree as ET

MAX_BYTES = 64 * 1024 * 1024

W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"
A = "{http://schemas.openxmlformats.org/drawingml/2006/main}"
S = "{http://schemas.openxmlformats.org/spreadsheetml/2006/main}"
R = "{http://schemas.openxmlformats.org/officeDocument/2006/relationships}"


def emit(payload):
    sys.stdout.write(json.dumps(payload, ensure_ascii=False))
    sys.stdout.write("\n")
    return 0


def fail(code, reason):
    return emit({"ok": False, "code": code, "reason": reason})


def md_table(rows):
    if not rows:
        return ""
    width = max(len(row) for row in rows)
    rows = [row + [""] * (width - len(row)) for row in rows]

    def cell(value):
        return str(value).replace("|", "\\|").replace("\n", " ").strip()

    head = "| " + " | ".join(cell(value) for value in rows[0]) + " |"
    rule = "| " + " | ".join("---" for _ in range(width)) + " |"
    body = ["| " + " | ".join(cell(value) for value in row) + " |" for row in rows[1:]]
    return "\n".join([head, rule] + body)


def markitdown(path):
    try:
        from markitdown import MarkItDown  # type: ignore
    except Exception:
        return None
    try:
        result = MarkItDown().convert(str(path))
        text = getattr(result, "text_content", None) or ""
        return text if text.strip() else None
    except Exception:
        return None


def docx(path):
    with zipfile.ZipFile(path) as archive:
        root = ET.fromstring(archive.read("word/document.xml"))
    body = root.find(W + "body")
    if body is None:
        return ""
    lines = []
    for child in body:
        if child.tag == W + "p":
            text = "".join(node.text or "" for node in child.iter(W + "t"))
            if text.strip():
                lines.append(text)
        elif child.tag == W + "tbl":
            rows = []
            for row in child.iter(W + "tr"):
                cells = []
                for cell_node in row.findall(W + "tc"):
                    cells.append("".join(n.text or "" for n in cell_node.iter(W + "t")))
                rows.append(cells)
            table = md_table(rows)
            if table:
                lines.append(table)
    return "\n\n".join(lines)


def pptx(path):
    with zipfile.ZipFile(path) as archive:
        names = [name for name in archive.namelist() if re.match(r"ppt/slides/slide\d+\.xml$", name)]

        def number(name):
            return int(re.search(r"(\d+)\.xml$", name).group(1))

        sections = []
        for name in sorted(names, key=number):
            root = ET.fromstring(archive.read(name))
            lines = []
            for paragraph in root.iter(A + "p"):
                text = "".join(node.text or "" for node in paragraph.iter(A + "t"))
                if text.strip():
                    lines.append(text)
            if lines:
                sections.append("## Slide %d\n\n%s" % (number(name), "\n\n".join(lines)))
        notes = [name for name in archive.namelist() if re.match(r"ppt/notesSlides/notesSlide\d+\.xml$", name)]
        for name in sorted(notes, key=number):
            root = ET.fromstring(archive.read(name))
            lines = []
            for paragraph in root.iter(A + "p"):
                text = "".join(node.text or "" for node in paragraph.iter(A + "t"))
                if text.strip():
                    lines.append(text)
            if lines:
                sections.append("## Notes %d\n\n%s" % (number(name), "\n\n".join(lines)))
    return "\n\n".join(sections)


def col_index(ref):
    letters = re.match(r"([A-Z]+)", ref or "")
    if not letters:
        return 0
    value = 0
    for char in letters.group(1):
        value = value * 26 + (ord(char) - 64)
    return max(0, value - 1)


def xlsx(path):
    with zipfile.ZipFile(path) as archive:
        names = archive.namelist()
        shared = []
        if "xl/sharedStrings.xml" in names:
            root = ET.fromstring(archive.read("xl/sharedStrings.xml"))
            for item in root.findall(S + "si"):
                shared.append("".join(node.text or "" for node in item.iter(S + "t")))
        sheet_names = []
        if "xl/workbook.xml" in names:
            root = ET.fromstring(archive.read("xl/workbook.xml"))
            for sheet in root.iter(S + "sheet"):
                sheet_names.append(sheet.get("name") or "")
        sheets = [name for name in names if re.match(r"xl/worksheets/sheet\d+\.xml$", name)]

        def number(name):
            return int(re.search(r"(\d+)\.xml$", name).group(1))

        sections = []
        for name in sorted(sheets, key=number):
            root = ET.fromstring(archive.read(name))
            rows = []
            for row in root.iter(S + "row"):
                cells = {}
                for cell in row.findall(S + "c"):
                    ref = cell.get("r") or ""
                    kind = cell.get("t")
                    value = ""
                    if kind == "s":
                        node = cell.find(S + "v")
                        try:
                            value = shared[int(node.text)]
                        except Exception:
                            value = ""
                    elif kind == "inlineStr":
                        value = "".join(n.text or "" for n in cell.iter(S + "t"))
                    else:
                        node = cell.find(S + "v")
                        if node is not None and node.text is not None:
                            value = node.text
                        formula = cell.find(S + "f")
                        if formula is not None and formula.text:
                            value = "=" + formula.text + ((" \u2192 " + value) if value else "")
                    cells[col_index(ref)] = value
                if cells:
                    width = max(cells) + 1
                    rows.append([cells.get(index, "") for index in range(width)])
            if rows:
                label = sheet_names[number(name) - 1] if number(name) - 1 < len(sheet_names) else name
                sections.append("## Sheet: %s\n\n%s" % (label, md_table(rows)))
    return "\n\n".join(sections)


def csv_file(path):
    with open(path, newline="", encoding="utf-8", errors="replace") as handle:
        rows = list(csv.reader(handle))
    return md_table(rows)


class TextParser(HTMLParser):
    def __init__(self):
        super().__init__()
        self.parts = []
        self.skip = 0

    def handle_starttag(self, tag, attrs):
        if tag in ("script", "style"):
            self.skip += 1
        elif tag in ("p", "br", "div", "li", "tr", "h1", "h2", "h3", "h4"):
            self.parts.append("\n")

    def handle_endtag(self, tag):
        if tag in ("script", "style") and self.skip:
            self.skip -= 1
        elif tag in ("p", "div", "li", "tr", "h1", "h2", "h3", "h4"):
            self.parts.append("\n")

    def handle_data(self, data):
        if not self.skip:
            self.parts.append(data)


def html_file(path):
    with open(path, encoding="utf-8", errors="replace") as handle:
        parser = TextParser()
        parser.feed(handle.read())
    text = "".join(parser.parts)
    lines = [line.strip() for line in text.splitlines()]
    return "\n".join(line for line in lines if line)


def pdf_file(path):
    binary = shutil.which("pdftotext")
    if not binary:
        return None, "pdftotext is not installed"
    done = subprocess.run(
        [binary, "-layout", "-enc", "UTF-8", str(path), "-"],
        capture_output=True, text=True, timeout=60,
    )
    if done.returncode != 0:
        return None, (done.stderr.strip() or "pdftotext refused the file")
    return done.stdout, None


def legacy(path, extension):
    binary = shutil.which("soffice") or shutil.which("libreoffice")
    if not binary:
        return None, "LibreOffice is not installed"
    target = {"doc": "docx", "xls": "xlsx", "ppt": "pptx"}.get(extension)
    if not target:
        return None, "unsupported legacy extension"
    directory = tempfile.mkdtemp(prefix="fleet-doc-")
    try:
        done = subprocess.run(
            [binary, "--headless", "--safe-mode", "--convert-to", target,
             "--outdir", directory, str(path)],
            capture_output=True, text=True, timeout=120,
        )
        if done.returncode != 0:
            return None, (done.stderr.strip() or "LibreOffice refused the file")
        converted = os.path.join(directory, os.path.splitext(os.path.basename(path))[0] + "." + target)
        if not os.path.exists(converted):
            return None, "LibreOffice produced no converted file"
        return converted, None
    finally:
        # The caller reads the converted file and removes its directory.
        pass


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--path", required=True)
    args = parser.parse_args()
    path = os.path.abspath(args.path)
    if not os.path.isfile(path):
        return fail("unsupported", "the path is not a file")
    size = os.path.getsize(path)
    if size > MAX_BYTES:
        return fail("too_large", "the file is larger than the 64 MB local bound")
    extension = os.path.splitext(path)[1].lower().lstrip(".")

    try:
        if extension in ("docx", "docm"):
            text = markitdown(path)
            parser = "local:markitdown"
            if text is None:
                text, parser = docx(path), "local:docx"
            return emit({"ok": True, "parser": parser, "markdown": text, "pages": None})
        if extension in ("pptx", "pptm"):
            text = markitdown(path)
            parser = "local:markitdown"
            if text is None:
                text, parser = pptx(path), "local:pptx"
            return emit({"ok": True, "parser": parser, "markdown": text, "pages": None})
        if extension in ("xlsx", "xlsm"):
            text = markitdown(path)
            parser = "local:markitdown"
            if text is None:
                text, parser = xlsx(path), "local:xlsx"
            return emit({"ok": True, "parser": parser, "markdown": text, "pages": None})
        if extension == "csv":
            return emit({"ok": True, "parser": "local:csv", "markdown": csv_file(path), "pages": None})
        if extension in ("html", "htm"):
            return emit({"ok": True, "parser": "local:html", "markdown": html_file(path), "pages": None})
        if extension in ("md", "txt", "log", "xml", "json"):
            with open(path, encoding="utf-8", errors="replace") as handle:
                return emit({"ok": True, "parser": "local:text", "markdown": handle.read(), "pages": None})
        if extension == "pdf":
            text, error = pdf_file(path)
            if error:
                return fail("missing_tool", error)
            if not text.strip():
                return fail("empty", "pdftotext found no text; the PDF is probably a scan")
            return emit({"ok": True, "parser": "local:pdftotext", "markdown": text, "pages": None})
        if extension in ("doc", "xls", "ppt"):
            converted, error = legacy(path, extension)
            if error:
                return fail("missing_tool" if "installed" in error else "unsupported", error)
            target = {"doc": docx, "xls": xlsx, "ppt": pptx}[extension]
            text = target(converted)
            directory = os.path.dirname(converted)
            shutil.rmtree(directory, ignore_errors=True)
            return emit({"ok": True, "parser": "local:libreoffice", "markdown": text, "pages": None})
        return fail("unsupported", "no local extractor for ." + extension)
    except zipfile.BadZipFile:
        return fail("unsupported", "the file is not the Office container it claims to be")
    except Exception as error:  # noqa: BLE001 - one JSON answer, never a traceback on stdout
        return fail("unsupported", str(error))


if __name__ == "__main__":
    sys.exit(main())
