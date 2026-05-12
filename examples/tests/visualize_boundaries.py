#!/usr/bin/env python3
"""
Visualize DataTable cell, row, and column boundaries.

Usage:
    python visualize_boundaries.py [cell_boundaries.json] [output.png]

The JSON is produced by the DataTableFixture.TestDumpCellBoundaries C++ test.
Default JSON path: ./cell_boundaries.json
Default output:    ./boundaries_visualization.png

Requires: pymupdf  (pip install pymupdf)
"""

import sys
import json
import argparse
from pathlib import Path

try:
    import fitz  # pymupdf
except ImportError:
    print("pymupdf is not installed.  Run:  pip install pymupdf", file=sys.stderr)
    sys.exit(1)


# One colour per column (KEY … ANNOTATION), RGB 0–1 floats.
_COLUMN_COLORS = [
    (0.90, 0.20, 0.20),   # KEY          – red
    (0.95, 0.55, 0.00),   # DMC_ARMY     – orange
    (0.70, 0.70, 0.00),   # NATO_STOCK   – yellow
    (0.15, 0.70, 0.15),   # ITEM_NAME    – green
    (0.10, 0.50, 0.90),   # PART_NUM     – blue
    (0.60, 0.10, 0.90),   # NUM_OFF      – purple
    (0.00, 0.72, 0.72),   # ANNOTATION   – cyan
]

_COLUMN_NAMES = ["KEY", "DMC", "NATO", "ITEM", "PART#", "#OFF", "ANN"]


def _col_color(col: int) -> tuple:
    return _COLUMN_COLORS[(col - 1) % len(_COLUMN_COLORS)]


def _merge(a: fitz.Rect, b: fitz.Rect) -> fitz.Rect:
    return fitz.Rect(min(a.x0, b.x0), min(a.y0, b.y0),
                     max(a.x1, b.x1), max(a.y1, b.y1))


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Render an annotated image of DataTable cell boundaries.")
    parser.add_argument("json_path",   nargs="?", default="cell_boundaries.json",
                        help="Path to the JSON file produced by TestDumpCellBoundaries")
    parser.add_argument("output_path", nargs="?", default="boundaries_visualization.png",
                        help="Where to save the output PNG")
    parser.add_argument("--scale", type=float, default=2.0,
                        help="Render scale factor (default: 2.0)")
    args = parser.parse_args()

    json_path = Path(args.json_path)
    if not json_path.exists():
        print(f"JSON file not found: {json_path}", file=sys.stderr)
        sys.exit(1)

    with open(json_path, encoding="utf-8") as f:
        data = json.load(f)

    pdf_path = data["pdf_path"]
    page_idx = int(data["page"])
    raw_cells = data["cells"]

    print(f"Loaded {len(raw_cells)} cells  |  page {page_idx}  |  {pdf_path}")

    # ── Open the PDF ────────────────────────────────────────────────────────
    doc = fitz.open(pdf_path)
    page = doc[page_idx]
    page_width = page.rect.width

    # ── Aggregate per-column and per-row bounding boxes ─────────────────────
    col_bounds: dict[int, fitz.Rect] = {}
    row_bounds: dict[int, fitz.Rect] = {}

    for cell in raw_cells:
        b = cell["boundary"]
        rect = fitz.Rect(b["x0"], b["y0"], b["x1"], b["y1"])
        col = cell["col"]
        row = cell["row"]
        col_bounds[col] = _merge(col_bounds[col], rect) if col in col_bounds else fitz.Rect(rect)
        row_bounds[row] = _merge(row_bounds[row], rect) if row in row_bounds else fitz.Rect(rect)

    # ── Draw on the page (all changes are in-memory, never saved to PDF) ────
    shape = page.new_shape()

    # 1. Column bands — faint fill + thick coloured border
    for col, rect in sorted(col_bounds.items()):
        c = _col_color(col)
        shape.draw_rect(rect)
        shape.finish(color=c, fill=c, fill_opacity=0.08, width=1.5)
        label = _COLUMN_NAMES[col - 1] if col <= len(_COLUMN_NAMES) else f"C{col}"
        shape.insert_text(
            fitz.Point(rect.x0 + 1, rect.y0 + 7),
            label, fontsize=5, color=c,
        )

    # 2. Row bands — dashed grey border spanning the full page width
    for row, rect in sorted(row_bounds.items()):
        row_rect = fitz.Rect(0, rect.y0, page_width, rect.y1)
        shape.draw_rect(row_rect)
        shape.finish(color=(0.40, 0.40, 0.40), fill=None, width=0.4, dashes="[3 3] 0")
        shape.insert_text(
            fitz.Point(2, (rect.y0 + rect.y1) / 2 + 2),
            f"R{row}", fontsize=4, color=(0.30, 0.30, 0.30),
        )

    # 3. Individual cell borders — solid, coloured by column
    for cell in raw_cells:
        b = cell["boundary"]
        rect = fitz.Rect(b["x0"], b["y0"], b["x1"], b["y1"])
        c = _col_color(cell["col"])
        shape.draw_rect(rect)
        shape.finish(color=c, fill=None, width=0.8)

    shape.commit()

    # ── Render and save ──────────────────────────────────────────────────────
    mat = fitz.Matrix(args.scale, args.scale)
    pix = page.get_pixmap(matrix=mat)
    out_path = Path(args.output_path)
    pix.save(str(out_path))

    doc.close()
    print(f"Saved  →  {out_path.resolve()}")
    print(f"       {pix.width}×{pix.height} px  (scale {args.scale}×)")


if __name__ == "__main__":
    main()
