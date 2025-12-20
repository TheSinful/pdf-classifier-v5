import sys 
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent / "src_py"))

from build import Builder 
from object import *

classify = ObjectFunc("test.h", "obj", "classify")
extract = ObjectFunc("test.h", "obj", "extract")

chapter = def_obj("chapter", classify, extract)
subchapter = def_obj("subchapter", classify, extract, chapter) 
(diagram, datatable) = def_pair("diagram", classify, extract, "datatable", classify, extract, subchapter)

build = Builder(Path(__file__).parent / "CMakeLists.txt", Path(__file__).parent / "build")
build.build()