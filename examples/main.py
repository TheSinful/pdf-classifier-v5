import sys 
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent / "src_py"))

from build import Builder 
from object import *

classify = ObjectFunc("test.h", "obj", "classify")
extract = ObjectFunc("test.h", "obj", "extract")

# Organizational objects (independent) - can serve as anchors during recovery
chapter = def_obj("chapter", classify, extract, is_organizational=True)
subchapter = def_obj("subchapter", classify, extract, chapter, is_organizational=True) 

# Data objects (dependent) - rely on organizational structure
(diagram, datatable) = def_pair("diagram", classify, extract, "datatable", classify, extract, subchapter, is_organizational=False)

build = Builder(Path(__file__).parent / "CMakeLists.txt", Path(__file__).parent / "build")
build.build()