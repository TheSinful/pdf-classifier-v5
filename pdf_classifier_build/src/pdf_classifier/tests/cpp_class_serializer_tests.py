import tempfile
from pathlib import Path
from weakref import ref

import pytest

from pdf_classifier.cpp_class_serializer import CppClassSerializer
from pdf_classifier.object import Object


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _make_example_objects() -> list[Object]:
    """
    Mirrors examples/main.py without touching the global OBJECTS list.
    Produces: unknown(0), chapter(1), subchapter(2), diagram(3), datatable(4)
    """
    unknown = Object("unknown", "UNKNOWN_classify", "UNKNOWN_extract")
    chapter = Object("chapter", "classify", "extract", is_organizational=True)
    subchapter = Object("subchapter", "classify", "extract", is_organizational=True)
    diagram = Object("diagram", "classify", "extract")
    datatable = Object("datatable", "classify", "extract")

    chapter.children.append(ref(subchapter))
    subchapter.children.append(ref(diagram))
    subchapter.children.append(ref(datatable))
    diagram.pair = (ref(datatable), 1)
    datatable.pair = (ref(diagram), 2)

    return [unknown, chapter, subchapter, diagram, datatable]


def _serializer(objects: list[Object] | None = None, tmp: Path | None = None) -> CppClassSerializer:
    if objects is None:
        objects = _make_example_objects()
    if tmp is None:
        tmp = Path(tempfile.mkdtemp()) / "out"
    return CppClassSerializer(tmp, objects)


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------

@pytest.fixture
def objs() -> list[Object]:
    return _make_example_objects()


@pytest.fixture
def tmp_dir() -> Path:
    # Return a non-existent subdirectory so CppClassSerializer's mkdir() succeeds
    return Path(tempfile.mkdtemp()) / "out"


@pytest.fixture
def ser(objs: list[Object], tmp_dir: Path) -> CppClassSerializer:
    return CppClassSerializer(tmp_dir, objs)


# ---------------------------------------------------------------------------
# Guards and includes
# ---------------------------------------------------------------------------

class TestGuardsAndIncludes:
    def test_pragma_once_present(self, ser: CppClassSerializer):
        ser._guards_and_includes()
        assert "#pragma once" in ser.data

    def test_string_include_present(self, ser: CppClassSerializer):
        ser._guards_and_includes()
        assert "#include <string>" in ser.data

    def test_stdexcept_include_present(self, ser: CppClassSerializer):
        ser._guards_and_includes()
        assert "#include <stdexcept>" in ser.data


# ---------------------------------------------------------------------------
# Enum generation
# ---------------------------------------------------------------------------

class TestClassEnum:
    def test_enum_declaration_present(self, ser: CppClassSerializer):
        ser._class_enum()
        assert "enum KnownObject {" in ser.data

    def test_all_variant_names_lowercase(self, ser: CppClassSerializer):
        ser._class_enum()
        for name in ("unknown", "chapter", "subchapter", "diagram", "datatable"):
            assert name in ser.data

    def test_variants_are_comma_separated(self, ser: CppClassSerializer):
        ser._class_enum()
        assert "unknown, chapter, subchapter, diagram, datatable" in ser.data

    def test_custom_enum_name(self, objs: list[Object], tmp_dir: Path):
        s = CppClassSerializer(tmp_dir, objs, enum_name="PageType")
        s._class_enum()
        assert "enum PageType {" in s.data

    def test_single_variant_enum(self, tmp_dir: Path):
        s = CppClassSerializer(tmp_dir, [Object("unknown", "", "")])
        s._class_enum()
        assert "unknown" in s.data


# ---------------------------------------------------------------------------
# page_type_to_string
# ---------------------------------------------------------------------------

class TestToStringMethod:
    def test_function_signature_present(self, ser: CppClassSerializer):
        ser._to_string_method()
        assert "std::string page_type_to_string(KnownObject obj) {" in ser.data

    def test_switch_statement_present(self, ser: CppClassSerializer):
        ser._to_string_method()
        assert "switch (obj) {" in ser.data

    def test_all_case_arms_present(self, ser: CppClassSerializer):
        ser._to_string_method()
        for name in ("unknown", "chapter", "subchapter", "diagram", "datatable"):
            assert f'case KnownObject::{name}: return "{name}";' in ser.data

    def test_default_arm_present(self, ser: CppClassSerializer):
        ser._to_string_method()
        assert 'default: return "unknown";' in ser.data


# ---------------------------------------------------------------------------
# page_type_from_string
# ---------------------------------------------------------------------------

class TestFromStringMethod:
    def test_function_signature_present(self, ser: CppClassSerializer):
        ser._from_string_method()
        assert "KnownObject page_type_from_string(std::string obj) {" in ser.data

    def test_all_if_branches_present(self, ser: CppClassSerializer):
        ser._from_string_method()
        for name in ("unknown", "chapter", "subchapter", "diagram", "datatable"):
            assert f'if(obj == "{name}") {{ return KnownObject::{name};}}' in ser.data

    def test_throw_on_unknown_string(self, ser: CppClassSerializer):
        ser._from_string_method()
        assert "throw std::runtime_error" in ser.data

    def test_throw_message_content(self, ser: CppClassSerializer):
        ser._from_string_method()
        assert "Attempted to convert string to object that doesn't exist!" in ser.data


# ---------------------------------------------------------------------------
# Full generate() — writes to a temp dir and validates file content
# ---------------------------------------------------------------------------

class TestFullGenerate:
    def test_file_is_written(self, objs: list[Object], tmp_dir: Path):
        s = CppClassSerializer(tmp_dir, objs)
        s.generate()
        out = tmp_dir / "generated_page_types.h"
        assert out.exists()
        assert out.stat().st_size > 0

    def test_custom_filename(self, objs: list[Object], tmp_dir: Path):
        s = CppClassSerializer(tmp_dir, objs, file_name="my_types.h")
        s.generate()
        assert (tmp_dir / "my_types.h").exists()

    def test_generated_content_has_pragma(self, objs: list[Object], tmp_dir: Path):
        s = CppClassSerializer(tmp_dir, objs)
        s.generate()
        content = (tmp_dir / "generated_page_types.h").read_text()
        assert "#pragma once" in content

    def test_generated_content_has_enum(self, objs: list[Object], tmp_dir: Path):
        s = CppClassSerializer(tmp_dir, objs)
        s.generate()
        content = (tmp_dir / "generated_page_types.h").read_text()
        assert "enum KnownObject {" in content

    def test_generated_content_has_to_string(self, objs: list[Object], tmp_dir: Path):
        s = CppClassSerializer(tmp_dir, objs)
        s.generate()
        content = (tmp_dir / "generated_page_types.h").read_text()
        assert "std::string page_type_to_string(KnownObject obj) {" in content

    def test_generated_content_has_from_string(self, objs: list[Object], tmp_dir: Path):
        s = CppClassSerializer(tmp_dir, objs)
        s.generate()
        content = (tmp_dir / "generated_page_types.h").read_text()
        assert "KnownObject page_type_from_string(std::string obj) {" in content

    def test_data_attribute_populated_after_generate(self, objs: list[Object], tmp_dir: Path):
        s = CppClassSerializer(tmp_dir, objs)
        s.generate()
        assert len(s.data) > 0
