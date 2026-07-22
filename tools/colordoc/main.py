"""End-to-end driver for the colordoc classifier test harness.

Generates a random monochromatic document, builds the classifier against the
colordoc schema, runs it over the document, and diffs what came back against
the ground truth the generator emitted.

Because every page is objectively one class, any disagreement reported here is
an engine bug - there is no ambiguity to blame.

    python main.py                        # random document, full build
    python main.py --seed 7               # reproduce a specific document
    python main.py --skip-user-build      # C++ unchanged, reuse the installed lib
    python main.py --run-tests            # also run the C++ gtest suite

NOTE: the Rust core has one generated schema directory and one binary, shared
with `examples/`. Running this rebuilds the core against the colordoc schema;
re-run `python examples/main.py` to switch back.
"""

from pathlib import Path
import argparse
import asyncio
import logging
import os
import subprocess
import sys

HERE = Path(__file__).parent
BUILD_DIR = HERE / "build"

# build.rs reads this to locate MuPDF, the generated headers and the user lib.
# It must be set before the Builder shells out to `cargo build`.
os.environ["CLASSIFIER_BUILD_DIR"] = str(BUILD_DIR)

from pdf_classifier import (  # noqa: E402  (import after the env var is set)
    Builder,
    ObjectFactory,
    BlankAfterClassOverride,
    MultiPageHierarchyBreakOverride,
    Stream,
)

import generate_doc  # noqa: E402
import palette  # noqa: E402

logger = logging.getLogger("colordoc")

RUN_PDF = HERE / "test_data" / "colordoc.pdf"
RUN_GROUNDTRUTH = HERE / "test_data" / "colordoc.json"

# Deterministic and small: this is what the C++ gtest suite classifies against.
FIXTURE_PDF = HERE / "test_data" / "fixture.pdf"
FIXTURE_GROUNDTRUTH = HERE / "test_data" / "fixture.json"
FIXTURE_SEED = 7
FIXTURE_MAX_PAGES = 60


def define_schema() -> ObjectFactory:
    """The colordoc schema - deliberately the same shape as `examples/`.

    Same hierarchy, same pairing, same organizational flags, so the engine walks
    the same constraint paths; only the classification itself is trivial.
    """
    factory = ObjectFactory("classes.hpp")

    factory.new().name("section") \
        .classify("classify_section").extract("extract_section") \
        .organizational().build()

    factory.new().name("subsection") \
        .classify("classify_subsection").extract("extract_subsection") \
        .child_of("section").organizational().build()

    factory.new().name("figure") \
        .classify("classify_figure").extract("extract_figure") \
        .child_of("subsection").pair_to("caption", 1).build()

    factory.new().name("caption") \
        .classify("classify_caption").extract("extract_caption") \
        .child_of("subsection").pair_to("figure", 2).build()

    return factory


def run_cpp_tests() -> bool:
    from cmake import CMAKE_BIN_DIR

    ctest = os.path.join(CMAKE_BIN_DIR, "ctest")
    logger.info("running C++ test suite in %s", BUILD_DIR)

    # UserCppBuilder builds the user project with --config Release, and
    # gtest_discover_tests only registers tests for the config that was actually
    # built - so ctest has to be pointed at the same one.
    completed = subprocess.run(
        [ctest, "--test-dir", str(BUILD_DIR), "--output-on-failure", "-C", "Release"],
        cwd=str(BUILD_DIR),
    )
    return completed.returncode == 0


async def collect_results(stream: Stream) -> dict[int, str]:
    """Drain the extraction stream into page -> class.

    A page can be re-decided (deferral backfill re-records a page it already
    guessed), so the last verdict for a page wins.
    """
    decided: dict[int, str] = {}

    async for result in stream.stream_extraction_results():
        page = int(result.page)
        class_id = result.class_id
        if isinstance(class_id, (bytes, bytearray)):
            class_id = class_id.decode("utf-8")

        decided[page] = class_id

    return decided


def report(truth: list[str], decided: dict[int, str], blank_class: str, max_detail: int) -> int:
    """Diff the engine's output against ground truth. Returns the defect count."""
    correct = 0
    mismatched: list[tuple[int, str, str]] = []
    missing: list[tuple[int, str]] = []
    unexpected: list[tuple[int, str]] = []

    per_class: dict[str, list[int]] = {}  # class -> [correct, total]

    for page, expected in enumerate(truth):
        actual = decided.get(page)
        tally = per_class.setdefault(expected, [0, 0])
        tally[1] += 1

        if expected == blank_class:
            # Blank pages never classify, so they never reach the extraction
            # stream. Their absence is the correct outcome.
            if actual is None:
                correct += 1
                tally[0] += 1
            else:
                unexpected.append((page, actual))
        elif actual is None:
            missing.append((page, expected))
        elif actual == expected:
            correct += 1
            tally[0] += 1
        else:
            mismatched.append((page, expected, actual))

    total = len(truth)
    defects = len(mismatched) + len(missing) + len(unexpected)

    print()
    print("=" * 68)
    print(f"colordoc verification: {correct}/{total} pages correct "
          f"({100.0 * correct / total:.1f}%)")
    print("=" * 68)

    print("\nper class:")
    for name in sorted(per_class):
        got, want = per_class[name]
        print(f"  {name:<12} {got:>4}/{want:<4} ({100.0 * got / want:5.1f}%)")

    def dump(title: str, rows: list, fmt) -> None:
        if not rows:
            return
        print(f"\n{title} ({len(rows)}):")
        for row in rows[:max_detail]:
            print(f"  {fmt(row)}")
        if len(rows) > max_detail:
            print(f"  ... and {len(rows) - max_detail} more")

    dump("MISMATCHED - engine decided a different class",
         mismatched, lambda r: f"page {r[0]:>4}: expected {r[1]:<12} got {r[2]}")
    dump("MISSING - page never reached the extraction stream",
         missing, lambda r: f"page {r[0]:>4}: expected {r[1]}")
    dump("UNEXPECTED - blank page produced an extraction",
         unexpected, lambda r: f"page {r[0]:>4}: expected nothing, got {r[1]}")

    if defects == 0:
        print("\nno defects.")
    else:
        print(f"\n{defects} defect(s). Re-run with the same --seed to reproduce, "
              f"or replay the trace in tools/visualizer.")

    return defects


async def main() -> int:
    args = build_arg_parser().parse_args()

    logging.basicConfig(
        level=getattr(logging, args.log_level.upper()),
        format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
    )

    # 1. Palette first: the generated header is what the C++ compiles against,
    #    and the generator paints from the same table.
    palette.write_header(HERE / palette.HEADER_NAME)

    # 2. The document under test, plus the fixed-seed fixture the gtest suite uses.
    spec = generate_doc.DocSpec(
        seed=args.seed,
        max_pages=args.max_pages,
        max_subsections=args.max_subsections,
        max_pairs=args.max_pairs,
        blank_after_section=not args.no_blank_after_section,
        hierarchy_break_chance=args.hierarchy_break_chance,
    )

    doc = generate_doc.generate(spec, RUN_PDF, RUN_GROUNDTRUTH)
    generate_doc.generate(
        generate_doc.DocSpec(seed=FIXTURE_SEED, max_pages=FIXTURE_MAX_PAGES),
        FIXTURE_PDF, FIXTURE_GROUNDTRUTH,
    )

    logger.info("document: seed=%d pages=%d sections=%d subsections=%d hierarchy_breaks=%d",
                doc.spec.seed, doc.page_count, doc.section_count,
                doc.subsection_count, doc.break_run_count)

    # 3. Compile the schema and build both native sides.
    blanks = not args.no_blank_after_section

    build = Builder(BUILD_DIR, define_schema(), HERE / "CMakeLists.txt")
    if blanks:
        # Only valid while the generator actually emits the blank page; without
        # one this would force a real page to UNKNOWN.
        build.override(BlankAfterClassOverride("section"))
    build.override(MultiPageHierarchyBreakOverride("section", blanks,
                                                   "subsection", ["figure", "caption"]))

    stream: Stream = build.build(skip_user_build=args.skip_user_build)

    if args.run_tests and not run_cpp_tests():
        print("\nC++ test suite failed; skipping the classifier run.", file=sys.stderr)
        return 1

    if args.build_only:
        return 0

    # 4. Run it. end_page is exclusive, so page_count covers the whole document.
    await build.spawn_classifier(0, doc.page_count, args.threads, RUN_PDF, False)

    decided = await collect_results(stream)

    defects = report(doc.pages, decided, generate_doc.BLANK_CLASS, args.max_detail)
    return 1 if defects else 0


def build_arg_parser() -> argparse.ArgumentParser:
    import random

    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--seed", type=int, default=random.randrange(1 << 30),
                        help="document seed; the same seed always yields the same document")
    parser.add_argument("--max-pages", type=int, default=160,
                        help="upper bound on document length (default: 160)")
    parser.add_argument("--max-subsections", type=int, default=4)
    parser.add_argument("--max-pairs", type=int, default=4)
    parser.add_argument("--hierarchy-break-chance", type=float, default=0.35,
                        help="probability a section opens with a hierarchy break (default: 0.35)")
    parser.add_argument("--no-blank-after-section", action="store_true",
                        help="omit blank pages, disabling the BlankAfter override path")
    parser.add_argument("--threads", type=int, default=4,
                        help="classifier worker threads (default: 4)")
    parser.add_argument("--skip-user-build", action="store_true",
                        help="reuse the installed C++ lib instead of rebuilding it")
    parser.add_argument("--run-tests", action="store_true",
                        help="run the C++ gtest suite after building")
    parser.add_argument("--build-only", action="store_true",
                        help="build (and optionally test) without running the classifier")
    parser.add_argument("--max-detail", type=int, default=25,
                        help="how many defects of each kind to list (default: 25)")
    parser.add_argument("--log-level", default="info",
                        choices=["debug", "info", "warning", "error"])
    return parser


if __name__ == "__main__":
    sys.exit(asyncio.run(main()))
