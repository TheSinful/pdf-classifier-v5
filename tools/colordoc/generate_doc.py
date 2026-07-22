"""Generate a random-length, structurally cohesive "colordoc" test document.

Every page is one flat monochromatic fill, so classification is objectively
correct: a page *is* its color. That makes the document useless as a real
corpus and ideal for exercising the classifier itself - any disagreement
between the engine's output and the ground truth emitted here is an engine bug,
never an ambiguous page.

The document follows the same shape as the reference project in ``examples/``
(root organizational class -> organizational child -> paired dependents), so it
drives the same constraint paths:

    document         := section_block+
    section_block    := SECTION [BLANK] [break_run] subsection_block+
    break_run        := pair+                 # pairs hanging directly off a section
    subsection_block := SUBSECTION pair+
    pair             := FIGURE CAPTION

- ``SECTION`` is the root class, so page 0 satisfies ``FirstPageRoot``.
- ``BLANK`` is an unpaintable white page, exercising ``BlankAfterClassOverride``.
- ``break_run`` is a deliberate hierarchy break (dependents with no
  ``subsection`` parent yet), exercising ``MultiPageHierarchyBreakOverride``.

Usage::

    python generate_doc.py --seed 7 --max-pages 160
"""

from dataclasses import dataclass, field
from pathlib import Path
import argparse
import json
import logging
import random

import pymupdf

import palette

logger = logging.getLogger(__name__)

#: Page filled with :data:`palette.BLANK`; matches no class and is expected to
#: come back from the engine as ``unknown``.
BLANK_CLASS = "unknown"

#: Deliberately small - nothing is ever read off these pages except one color.
PAGE_WIDTH = 180.0
PAGE_HEIGHT = 240.0

#: A section block cannot be smaller than section+blank+subsection+figure+caption.
MIN_BLOCK_PAGES = 5


@dataclass
class DocSpec:
    """Knobs controlling the shape of a generated document."""

    seed: int
    max_pages: int = 160
    max_subsections: int = 4
    max_pairs: int = 4
    blank_after_section: bool = True
    hierarchy_break_chance: float = 0.35
    max_break_pairs: int = 3

    def validate(self) -> None:
        floor = MIN_BLOCK_PAGES if self.blank_after_section else MIN_BLOCK_PAGES - 1
        if self.max_pages < floor:
            raise ValueError(
                f"--max-pages must be at least {floor} to fit one complete section block"
            )
        if self.max_subsections < 1 or self.max_pairs < 1:
            raise ValueError("--max-subsections and --max-pairs must both be at least 1")
        if not 0.0 <= self.hierarchy_break_chance <= 1.0:
            raise ValueError("--hierarchy-break-chance must be between 0.0 and 1.0")

    def as_dict(self) -> dict:
        return {
            "seed": self.seed,
            "max_pages": self.max_pages,
            "max_subsections": self.max_subsections,
            "max_pairs": self.max_pairs,
            "blank_after_section": self.blank_after_section,
            "hierarchy_break_chance": self.hierarchy_break_chance,
            "max_break_pairs": self.max_break_pairs,
        }


@dataclass
class GeneratedDoc:
    """The page-by-page plan for a document, before it is painted."""

    spec: DocSpec
    pages: list[str] = field(default_factory=list)
    section_count: int = 0
    subsection_count: int = 0
    break_run_count: int = 0

    @property
    def page_count(self) -> int:
        return len(self.pages)


def _pair() -> list[str]:
    return ["figure", "caption"]


def _subsection_block(rng: random.Random, spec: DocSpec) -> list[str]:
    pairs = rng.randint(1, spec.max_pairs)
    block = ["subsection"]
    for _ in range(pairs):
        block.extend(_pair())
    return block


def _section_block(rng: random.Random, spec: DocSpec) -> tuple[list[str], int, bool]:
    """Build one complete section block. Returns (pages, subsections, had_break)."""
    block = ["section"]
    if spec.blank_after_section:
        block.append(BLANK_CLASS)

    had_break = rng.random() < spec.hierarchy_break_chance
    if had_break:
        for _ in range(rng.randint(1, spec.max_break_pairs)):
            block.extend(_pair())

    subsections = rng.randint(1, spec.max_subsections)
    for _ in range(subsections):
        block.extend(_subsection_block(rng, spec))

    return block, subsections, had_break


def plan_document(spec: DocSpec) -> GeneratedDoc:
    """Lay out a random document, emitting only *whole* section blocks.

    Truncating a block mid-pair would produce a document the schema itself
    considers malformed, which would make an engine disagreement unattributable.
    So blocks are appended only while they fit under ``max_pages``.
    """
    spec.validate()
    rng = random.Random(spec.seed)
    doc = GeneratedDoc(spec=spec)

    while True:
        block, subsections, had_break = _section_block(rng, spec)

        if doc.pages and len(doc.pages) + len(block) > spec.max_pages:
            break

        if not doc.pages and len(block) > spec.max_pages:
            # First block overshot; shrink it to the minimum viable shape rather
            # than emitting a structurally invalid document.
            block = ["section"]
            if spec.blank_after_section:
                block.append(BLANK_CLASS)
            block.extend(["subsection", "figure", "caption"])
            subsections, had_break = 1, False

        doc.pages.extend(block)
        doc.section_count += 1
        doc.subsection_count += subsections
        doc.break_run_count += int(had_break)

    logger.info(
        "planned %d pages: %d sections, %d subsections, %d hierarchy breaks",
        doc.page_count, doc.section_count, doc.subsection_count, doc.break_run_count,
    )
    return doc


def _fill_for(class_name: str) -> palette.RGB:
    if class_name == BLANK_CLASS:
        return palette.BLANK
    return palette.PALETTE[class_name]


def paint(doc: GeneratedDoc, pdf_path: Path) -> None:
    """Render the plan to a PDF, one flat fill per page."""
    pdf = pymupdf.open()
    try:
        for class_name in doc.pages:
            r, g, b = _fill_for(class_name)
            fill = (r / 255.0, g / 255.0, b / 255.0)

            page = pdf.new_page(width=PAGE_WIDTH, height=PAGE_HEIGHT)
            # Bleed past the page edge so the fill covers the trim completely and
            # the rasteriser has no partially-covered border pixels to blend.
            rect = page.rect + (-4, -4, 4, 4)

            shape = page.new_shape()
            shape.draw_rect(rect)
            shape.finish(fill=fill, color=fill, width=0)
            shape.commit()

        pdf_path.parent.mkdir(parents=True, exist_ok=True)
        pdf.save(str(pdf_path), garbage=4, deflate=True)
    finally:
        pdf.close()

    logger.info("wrote %d-page document -> %s", doc.page_count, pdf_path)


def write_groundtruth(doc: GeneratedDoc, path: Path) -> None:
    payload = {
        "page_count": doc.page_count,
        "section_count": doc.section_count,
        "subsection_count": doc.subsection_count,
        "break_run_count": doc.break_run_count,
        "blank_class": BLANK_CLASS,
        "spec": doc.spec.as_dict(),
        "palette": {name: list(rgb) for name, rgb in palette.PALETTE.items()},
        "blank": list(palette.BLANK),
        "tolerance": palette.TOLERANCE,
        # Flat, page-indexed: pages[n] is the class of page n.
        "pages": doc.pages,
    }

    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2), encoding="utf-8")
    logger.info("wrote ground truth -> %s", path)


def generate(spec: DocSpec, pdf_path: Path, groundtruth_path: Path) -> GeneratedDoc:
    doc = plan_document(spec)
    paint(doc, pdf_path)
    write_groundtruth(doc, groundtruth_path)
    return doc


def build_arg_parser() -> argparse.ArgumentParser:
    here = Path(__file__).parent
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--seed", type=int, default=random.randrange(1 << 30),
                        help="RNG seed; the same seed always yields the same document")
    parser.add_argument("--max-pages", type=int, default=160,
                        help="upper bound on document length (default: 160)")
    parser.add_argument("--max-subsections", type=int, default=4,
                        help="upper bound on subsections per section (default: 4)")
    parser.add_argument("--max-pairs", type=int, default=4,
                        help="upper bound on figure/caption pairs per subsection (default: 4)")
    parser.add_argument("--max-break-pairs", type=int, default=3,
                        help="upper bound on pairs in a hierarchy-break run (default: 3)")
    parser.add_argument("--hierarchy-break-chance", type=float, default=0.35,
                        help="probability a section opens with a hierarchy break (default: 0.35)")
    parser.add_argument("--no-blank-after-section", action="store_true",
                        help="omit the blank page after each section")
    parser.add_argument("--out", type=Path, default=here / "test_data" / "colordoc.pdf",
                        help="path of the PDF to write")
    parser.add_argument("--groundtruth", type=Path, default=None,
                        help="path of the ground-truth JSON (default: <out>.json)")
    return parser


def spec_from_args(args: argparse.Namespace) -> DocSpec:
    return DocSpec(
        seed=args.seed,
        max_pages=args.max_pages,
        max_subsections=args.max_subsections,
        max_pairs=args.max_pairs,
        blank_after_section=not args.no_blank_after_section,
        hierarchy_break_chance=args.hierarchy_break_chance,
        max_break_pairs=args.max_break_pairs,
    )


def main() -> None:
    logging.basicConfig(level=logging.INFO, format="%(levelname)s: %(message)s")
    args = build_arg_parser().parse_args()

    groundtruth = args.groundtruth or args.out.with_suffix(".json")
    doc = generate(spec_from_args(args), args.out, groundtruth)

    print(f"seed={doc.spec.seed} pages={doc.page_count} "
          f"sections={doc.section_count} subsections={doc.subsection_count} "
          f"hierarchy_breaks={doc.break_run_count}")


if __name__ == "__main__":
    main()
