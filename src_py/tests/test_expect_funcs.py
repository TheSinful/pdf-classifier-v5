import sys 
import pytest 
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

from build import Builder
from globals import EXPECTED_CLASSIFY_FUNCTIONS, EXPECTED_EXTRACT_FUNCTIONS
from typing import Any

TEST_HEADER: str = f"""
#pragma once

#include <any> 

void* classify(const PageContext& ctx); 
void extract(const PageContext& ctx, void* shared);
"""    

def test_get_expected_funcs(): 
    path = Path.cwd() / "test_data" / "get_expected_funcs"
    path.mkdir(exist_ok=True)
    builder = Builder(path / "CMakeLists.txt", path) 
    
    with open(path / "test_header.h", "w") as f: 
        f.write(TEST_HEADER)
    
    funcs: list[dict[str, Any]] = builder._get_available_functions()
    
    assert funcs.__len__() == 2, "Expected two functions."
    assert funcs[0]["name"] == "classify", "Expected classify func" 
    assert funcs[0]["parameters"][0]["type"] == "const PageContext&", "Expected first param to be of PageContext type."
    assert funcs[0]["return_type"] == "void*", "Expected first param to be of void* return type."
    assert funcs[1]["name"] == "extract", "Expected extract func."
    assert funcs[1]["parameters"][0]["type"] == "const PageContext&", "Expected first param to be of PageContext type."
    assert funcs[1]["parameters"][1]["type"] == "void*", "Expected second param to be of void* type."
    assert funcs[1]["return_type"] == "void", "Expected extraction func return type to be void"
    
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