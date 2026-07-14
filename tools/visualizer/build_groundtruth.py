#!/usr/bin/env python3
"""
Ground-truth generator for the visualizer's "error %" tab.

Derives a per-page class map for data/large_test_doc.pdf by combining:

  1. the static-classifier export (data/large_test_doc_classified/) — the
     authoritative *sequence* of chapter/subchapter anchors (the export has no
     page numbers, and its data-block page counts are incomplete, so it cannot
     position pages on its own), and
  2. the PDF's own text signatures, which position every page:
       - anchor pages carry a standalone uppercase "CHAPTER x-y[-z]" heading
       - datatable pages carry the parts-table header ("DMC" + "... stock")
       - diagram pages carry a "Fig N:" caption (often OCR-mangled, so any
         page that is neither an anchor, a chapter blank, nor a table is a
         diagram — validated: every such page sits in a diagram slot)
       - the page after a chapter anchor is the blank/record-of-mods page
         (class "unknown", mirroring the BlankAfter override)

The detected anchor sequence is aligned to the export sequence (tolerating
OCR-mangled numbers) and the run aborts if any export anchor cannot be
positioned. Output: static/groundtruth.js (window.CLASSIFIER_GROUND_TRUTH).

Requires pymupdf: pip install pymupdf
"""

import json
import re
import sys
from datetime import date
from pathlib import Path

try:
    import fitz  # pymupdf
except ImportError:
    print("pymupdf is not installed. Run: pip install pymupdf", file=sys.stderr)
    sys.exit(1)

HERE = Path(__file__).resolve().parent
REPO_ROOT = HERE.parent.parent
EXPORT = REPO_ROOT / "data" / "large_test_doc_classified"
PDF = REPO_ROOT / "data" / "large_test_doc.pdf"
OUT = HERE / "static" / "groundtruth.js"

START_PAGE = 44
END_PAGE = 1280  # exclusive

# standalone heading line, e.g. "CHAPTER 2-1-13" (footers use mixed-case "Chapter")
ANCHOR_LINE_RE = re.compile(r"^\s*CHAPTER\s+(\d+(?:\s*-\s*\d+){1,2})\s*$", re.MULTILINE)
# looser fallback for OCR-mangled headings: uppercase CHAPTER + a "Page 1" footer
ANCHOR_LOOSE_RE = re.compile(r"CHAPTER\s*(\d+(?:\s*-\s*\d+){1,2})")
PAGE1_RE = re.compile(r"PAGE\s*1(?!\d)")
FIG_CAPTION_RE = re.compile(r"FIG\s*\d+\s*:", re.IGNORECASE)


def export_anchor_sequence():
    """(kind, number) anchors in document order, from the export metadata."""
    seq = []
    chapters = json.loads((EXPORT / "chapters.json").read_text(encoding="utf-8"))["chapters"]
    for ch in chapters:
        seq.append(("chapter", ch["chapter_number"]))
        ch_json = json.loads(
            (EXPORT / ch["relative_path"] / f"chapter_{ch['chapter_number']}.json")
            .read_text(encoding="utf-8")
        )
        for sub in ch_json["sub_chapters"]:
            seq.append(("subchapter", sub["sub_chapter_number"]))
    return seq


def detect_anchors(texts):
    """[(page, raw_number, strong)] anchor candidates.

    strong  — a standalone uppercase "CHAPTER <num>" heading line
    loose   — uppercase CHAPTER + <num> anywhere plus a "Page 1" footer
              (catches OCR-mangled headings; may include table pages whose
              annotations cite another chapter — alignment weeds those out)
    """
    out = []
    for page, text in texts.items():
        m = ANCHOR_LINE_RE.search(text)
        if m:
            out.append((page, re.sub(r"\s*", "", m.group(1)), True))
            continue
        up = re.sub(r"[ \t]+", " ", text.upper())
        m = ANCHOR_LOOSE_RE.search(up)
        if m and PAGE1_RE.search(up):
            out.append((page, re.sub(r"\s*", "", m.group(1)), False))
    return sorted(out)


def is_table_page(text):
    up = text.upper()
    return "DMC" in up and "STOCK" in up


def align(detected, expected):
    """Needleman-Wunsch alignment of detected anchor numbers vs export numbers.

    Digit-exact matches on strong detections are cheapest, then loose
    detections, then OCR-mangled pairings; gaps cost between the two so a
    junk detection is skipped rather than paired. Returns
    (pairs, false_positives, missed) where pairs is
    [(page, export_kind, export_number)].
    """
    digits = lambda s: re.sub(r"\D", "", s or "")

    def pair_cost(d, e):
        if digits(d[1]) == digits(e[1]):
            return 0 if d[2] else 1
        return 4

    n, m = len(detected), len(expected)
    GAP = 2
    INF = 10 ** 9
    dp = [[INF] * (m + 1) for _ in range(n + 1)]
    dp[0][0] = 0
    for i in range(n + 1):
        for j in range(m + 1):
            cur = dp[i][j]
            if cur == INF:
                continue
            if i < n and j < m:
                dp[i + 1][j + 1] = min(dp[i + 1][j + 1], cur + pair_cost(detected[i], expected[j]))
            if i < n:
                dp[i + 1][j] = min(dp[i + 1][j], cur + GAP)
            if j < m:
                dp[i][j + 1] = min(dp[i][j + 1], cur + GAP)

    i, j = n, m
    pairs, false_pos, missed = [], [], []
    while i > 0 or j > 0:
        cur = dp[i][j]
        if i > 0 and j > 0 and dp[i - 1][j - 1] + pair_cost(detected[i - 1], expected[j - 1]) == cur:
            pairs.append((detected[i - 1][0], *expected[j - 1]))
            i, j = i - 1, j - 1
            continue
        if i > 0 and dp[i - 1][j] + GAP == cur:
            false_pos.append(detected[i - 1])
            i -= 1
            continue
        missed.append(expected[j - 1])
        j -= 1
    pairs.reverse()
    return pairs, false_pos, missed


def main():
    doc = fitz.open(str(PDF))
    texts = {p: doc[p].get_text() for p in range(START_PAGE, min(END_PAGE, doc.page_count))}

    expected = export_anchor_sequence()
    detected = detect_anchors(texts)
    pairs, false_pos, missed = align(detected, expected)

    if missed:
        print(f"FATAL: {len(missed)} export anchors could not be positioned: {missed[:10]}",
              file=sys.stderr)
        sys.exit(1)

    anchors = {page: (kind, num) for page, kind, num in pairs}
    last_export_page = max(anchors)

    # detections past the export's coverage are real structure (e.g. chapter 3-1
    # at the tail of the range); mid-sequence extras would be OCR junk.
    # (loose detections are fine out here: no export coverage to misalign, and
    # the "Page 1" footer requirement already gates them)
    beyond = []
    for page, raw, _strong in false_pos:
        if page > last_export_page:
            kind = "chapter" if raw.count("-") == 1 else "subchapter"
            anchors[page] = (kind, raw)
            beyond.append({"page": page, "number": raw})
        else:
            print(f"warning: dropping unaligned anchor candidate page {page} ({raw!r})",
                  file=sys.stderr)

    pages = {}
    inferred_diagrams = 0
    for p in sorted(texts):
        if p in anchors:
            kind, num = anchors[p]
            pages[p] = {"c": kind, "n": num}
        elif (p - 1) in anchors and anchors[p - 1][0] == "chapter":
            pages[p] = {"c": "unknown"}  # blank / record-of-mods after a chapter
        elif is_table_page(texts[p]):
            pages[p] = {"c": "datatable"}
        else:
            pages[p] = {"c": "diagram"}
            if not FIG_CAPTION_RE.search(texts[p]):
                inferred_diagrams += 1
                pages[p]["i"] = 1  # caption unreadable; class inferred from position

    counts = {}
    for v in pages.values():
        counts[v["c"]] = counts.get(v["c"], 0) + 1

    payload = {
        "doc": PDF.name,
        "startPage": START_PAGE,
        "endPage": END_PAGE,
        "generated": date.today().isoformat(),
        "source": "data/large_test_doc_classified + PDF text signatures",
        "counts": counts,
        "anchorsFromExport": len(pairs),
        "anchorsBeyondExport": beyond,
        "inferredDiagrams": inferred_diagrams,
        "pages": {str(p): v for p, v in pages.items()},
    }

    OUT.write_text(
        "// Generated by build_groundtruth.py — do not edit by hand.\n"
        "// (encoding: utf-8)\n"
        "// Per-page ground truth for data/large_test_doc.pdf, derived from the\n"
        "// static-classifier export + PDF text signatures. See the script header.\n"
        "window.CLASSIFIER_GROUND_TRUTH = "
        + json.dumps(payload, separators=(",", ":"))
        + ";\n",
        encoding="utf-8",
    )
    print(f"anchors: {len(pairs)} export-aligned"
          + (f" + {len(beyond)} beyond export {[b['number'] for b in beyond]}" if beyond else ""))
    print(f"class counts: {counts}")
    print(f"diagram pages inferred from position (no readable caption): {inferred_diagrams}")
    print(f"wrote {OUT} ({OUT.stat().st_size // 1024} KiB)")


if __name__ == "__main__":
    main()
