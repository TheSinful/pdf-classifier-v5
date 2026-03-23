from pathlib import Path
from pdf_classifier import Builder, ObjectFactory, BlankAfterClassOverride
import logging

logging.basicConfig(
    level=logging.DEBUG,
    format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
)

factory = ObjectFactory("test.hpp")

factory.new().name("chapter").classify("classify").extract("extract").organizational().build()
factory.new().name("subchapter").classify("classify").extract("extract").child_of("chapter").organizational().build()
factory.new().name("diagram").classify("classify").extract("extract").child_of("subchapter").pair_to("datatable", 1).build()
factory.new().name("datatable").classify("classify").extract("extract").child_of("subchapter").pair_to("diagram", 2).build()

examples_root = Path(__file__).parent

build = Builder(examples_root / "build", factory, examples_root / "CMakeLists.txt", )
build.override(BlankAfterClassOverride("chapter")).build()
