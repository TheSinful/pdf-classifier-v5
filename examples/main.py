from pathlib import Path
from pdf_classifier import Builder, ObjectFactory, BlankAfterClassOverride, MultiPageHierarchyBreakOverride
import logging

logging.basicConfig(
    level=logging.DEBUG,
    format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
)

factory = ObjectFactory("test.hpp")

factory.new().name("chapter").header("chapter.hpp").classify("classify_chapter").extract("extract_chapter").organizational().build()
factory.new().name("subchapter").header("subchapter.hpp").classify("classify_subchapter").extract("extract_subchapter").child_of("chapter").organizational().build()
factory.new().name("diagram").header("diagram.hpp").classify("classify_diagram").extract("extract_diagram").child_of("subchapter").pair_to("datatable", 1).build()
factory.new().name("datatable").header("table.hpp").classify("classify_datatable").extract("extract_datatable").child_of("subchapter").pair_to("diagram", 2).build()

examples_root = Path(__file__).parent

build = Builder(examples_root / "build", factory, examples_root / "CMakeLists.txt", )
build.override(BlankAfterClassOverride("chapter"))
build.override(MultiPageHierarchyBreakOverride("chapter", True, "subchapter", ["diagram", "datatable"]))
build.build()