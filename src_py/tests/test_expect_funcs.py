import sys 
import pytest 
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

from build import Builder, EXPECTED_CLASSIFY_FUNC_SYNTAX, EXPECTED_EXTRACT_FUNC_SYNTAX
from globals import EXPECTED_CLASSIFY_FUNCTIONS, EXPECTED_EXTRACT_FUNCTIONS
from typing import Any

TEST_HEADER: str = f"""
#pragma once

#include <any>
#include <mupdf/fitz.h>

void* classify(fz_context* ctx, fz_document* doc);
void extract(fz_context* ctx, fz_document* doc, void* shared);
"""    

def test_get_expected_funcs(): 
    path = Path.cwd() / "test_data" / "get_expected_funcs"
    path.mkdir(exist_ok=True)
    builder  = Builder(path / "CMakeLists.txt", path) 
    
    with open(path / "test_header.h", "w") as f: 
        f.write(TEST_HEADER)
    
    funcs: list[dict[str, Any]] = builder._get_available_functions()
    expected_classify: list[tuple[str,str,str]] = [] 
    expected_extract: list[tuple[str,str,str]] = [] 
    
    expected_classify.append(("test_header.h", "", "classify"))
    expected_extract.append(("test_header.h", "", "extract"))
    
    assert builder._validate_func(funcs[0], expected_classify, EXPECTED_CLASSIFY_FUNC_SYNTAX) 
    assert builder._validate_func(funcs[1], expected_extract, EXPECTED_EXTRACT_FUNC_SYNTAX) 
        
def test_validate_expected_funcs(): 
    global EXPECTED_CLASSIFY_FUNCTIONS
    global EXPECTED_EXTRACT_FUNCTIONS
    
    path = Path.cwd() / "test_data" / "get_expected_funcs"
    path.mkdir(exist_ok=True)
    builder = Builder(path / "CMakeLists.txt", path) 
    
    with open(path / "test_header.h", "w") as f: 
        f.write(TEST_HEADER)
        
    EXPECTED_CLASSIFY_FUNCTIONS.append(("test_header", "stuff", "classify"))
    EXPECTED_EXTRACT_FUNCTIONS.append(("test_header", "stuff", "extract"))
    builder._validate_expected_funcs_exist()