import pytest
from pathlib import Path

from pdf_classifier.cpp_func_map_generator import CppFuncMapGenerator
from pdf_classifier.user_func import UserFunc

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

BUILD_DIR = Path(__file__).parent.parent.parent.parent.parent / "pdf_classifier_build" / "build"


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _chapter_funcs() -> tuple[list[UserFunc], list[UserFunc]]:
    """One classify + one extract func, both for 'chapter'."""
    classify = [UserFunc("test.hpp", "chapter", "classify_chapter")]
    extract = [UserFunc("test.hpp", "chapter", "extract_chapter")]
    return classify, extract


def _multi_funcs() -> tuple[list[UserFunc], list[UserFunc]]:
    """Three objects across two header files."""
    classify = [
        UserFunc("test.hpp", "chapter", "classify_chapter"),
        UserFunc("test.hpp", "subchapter", "classify_subchapter"),
        UserFunc("extra.hpp", "diagram", "classify_diagram"),
    ]
    extract = [
        UserFunc("test.hpp", "chapter", "extract_chapter"),
        UserFunc("test.hpp", "subchapter", "extract_subchapter"),
        UserFunc("extra.hpp", "diagram", "extract_diagram"),
    ]
    return classify, extract


def _generator(
    classify: list[UserFunc] | None = None,
    extract: list[UserFunc] | None = None,
    out_dir: Path | None = None,
) -> CppFuncMapGenerator:
    if classify is None:
        classify, extract = _chapter_funcs()
    if extract is None:
        _, extract = _chapter_funcs()
    if out_dir is None:
        out_dir = BUILD_DIR
    out_dir.mkdir(parents=True, exist_ok=True)
    return CppFuncMapGenerator(classify, extract, out_dir)


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------

@pytest.fixture
def gen() -> CppFuncMapGenerator:
    return _generator()


@pytest.fixture
def multi_gen() -> CppFuncMapGenerator:
    classify, extract = _multi_funcs()
    return _generator(classify, extract)


# ---------------------------------------------------------------------------
# _include_files
# ---------------------------------------------------------------------------

class TestIncludeFiles:
    def test_pragma_once(self, gen: CppFuncMapGenerator):
        gen._include_files()
        assert "#pragma once" in gen.data

    def test_string_include(self, gen: CppFuncMapGenerator):
        gen._include_files()
        assert "#include <string>" in gen.data

    def test_vector_include(self, gen: CppFuncMapGenerator):
        gen._include_files()
        assert "#include <vector>" in gen.data

    def test_user_header_included(self, gen: CppFuncMapGenerator):
        gen._include_files()
        assert "#include <generated/test.hpp>" in gen.data

    def test_multiple_headers_included(self, multi_gen: CppFuncMapGenerator):
        multi_gen._include_files()
        assert "#include <generated/test.hpp>" in multi_gen.data
        assert "#include <generated/extra.hpp>" in multi_gen.data

    def test_duplicate_headers_deduplicated(self, multi_gen: CppFuncMapGenerator):
        """test.hpp appears on multiple funcs but should only be included once."""
        multi_gen._include_files()
        assert multi_gen.data.count("#include <generated/test.hpp>") == 1


# ---------------------------------------------------------------------------
# _func_struct
# ---------------------------------------------------------------------------

class TestFuncStruct:
    def test_struct_keyword(self, gen: CppFuncMapGenerator):
        gen._func_struct()
        assert "struct Func" in gen.data

    def test_obj_name_field(self, gen: CppFuncMapGenerator):
        gen._func_struct()
        assert "std::string obj_name" in gen.data

    def test_ptr_field(self, gen: CppFuncMapGenerator):
        gen._func_struct()
        assert "void* ptr" in gen.data


# ---------------------------------------------------------------------------
# _classify_func_map
# ---------------------------------------------------------------------------

class TestClassifyFuncMap:
    def test_map_name_present(self, gen: CppFuncMapGenerator):
        gen._classify_func_map()
        assert "ClassifyFuncMap" in gen.data

    def test_entry_obj_name(self, gen: CppFuncMapGenerator):
        gen._classify_func_map()
        assert '"chapter"' in gen.data

    def test_entry_func_ptr(self, gen: CppFuncMapGenerator):
        gen._classify_func_map()
        assert "&classify_chapter" in gen.data

    def test_multiple_entries(self, multi_gen: CppFuncMapGenerator):
        multi_gen._classify_func_map()
        assert "&classify_chapter" in multi_gen.data
        assert "&classify_subchapter" in multi_gen.data
        assert "&classify_diagram" in multi_gen.data

    def test_no_extract_funcs_present(self, gen: CppFuncMapGenerator):
        gen._classify_func_map()
        assert "extract" not in gen.data


# ---------------------------------------------------------------------------
# _extract_func_map
# ---------------------------------------------------------------------------

class TestExtractFuncMap:
    def test_map_name_present(self, gen: CppFuncMapGenerator):
        gen._extract_func_map()
        assert "ExtractFuncMap" in gen.data

    def test_entry_obj_name(self, gen: CppFuncMapGenerator):
        gen._extract_func_map()
        assert '"chapter"' in gen.data

    def test_entry_func_ptr(self, gen: CppFuncMapGenerator):
        gen._extract_func_map()
        assert "&extract_chapter" in gen.data

    def test_multiple_entries(self, multi_gen: CppFuncMapGenerator):
        multi_gen._extract_func_map()
        assert "&extract_chapter" in multi_gen.data
        assert "&extract_subchapter" in multi_gen.data
        assert "&extract_diagram" in multi_gen.data

    def test_no_classify_funcs_present(self, gen: CppFuncMapGenerator):
        gen._extract_func_map()
        assert "classify" not in gen.data


# ---------------------------------------------------------------------------
# generate (full output)
# ---------------------------------------------------------------------------

class TestGenerate:
    def test_output_file_created(self, gen: CppFuncMapGenerator):
        gen.generate()
        assert gen.out_path.exists()

    def test_output_contains_all_sections(self, gen: CppFuncMapGenerator):
        gen.generate()
        content = gen.out_path.read_text()
        assert "#pragma once" in content
        assert "struct Func" in content
        assert "ClassifyFuncMap" in content
        assert "ExtractFuncMap" in content

    def test_output_is_non_empty(self, gen: CppFuncMapGenerator):
        gen.generate()
        assert gen.out_path.stat().st_size > 0

    def test_custom_file_name(self):
        classify, extract = _chapter_funcs()
        out_dir = BUILD_DIR
        out_dir.mkdir(parents=True, exist_ok=True)
        gen = CppFuncMapGenerator(classify, extract, out_dir, file_name="custom_map.h")
        gen.generate()
        assert (out_dir / "custom_map.h").exists()

    def test_all_expected_names_in_output(self, multi_gen: CppFuncMapGenerator):
        multi_gen.generate()
        content = multi_gen.out_path.read_text()
        for name in ["chapter", "subchapter", "diagram"]:
            assert f'"{name}"' in content
