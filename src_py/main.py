from object import *


chapter = def_obj("chapter", "" , "") 
subchapter = def_obj("subchapter", "", "", chapter) 
(diagram, datatable) = def_pair("diagram", "", "", "datatable", "", "", subchapter)

build = Builder()
build._build_mupdf()

