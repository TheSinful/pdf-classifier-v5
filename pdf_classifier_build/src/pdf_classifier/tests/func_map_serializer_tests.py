import pytest
import tempfile
from pathlib import Path
from pdf_classifier.func_map_validator import UserFuncValidator, ParsedFunc
from pdf_classifier.user_func import UserFunc, FuncSyntax


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

CLASSIFY_SYNTAX = FuncSyntax("Result*", ["uint32_t", "fz_context*", "fz_document*"])
EXTRACT_SYNTAX = FuncSyntax("Result*", ["uint32_t", "fz_context*", "fz_document*", "void*"])


def _params(*types: str) -> list[dict[str, str]]:
    """Build a parameters list where each entry has a dummy name."""
    return [{"type": t, "name": f"p{i}"} for i, t in enumerate(types)]


def _classify_func(name: str) -> ParsedFunc:
    return ParsedFunc(
        name=name,
        return_type="Result*",
        parameters=_params("uint32_t", "fz_context*", "fz_document*"),
        file="test.hpp",
    )


def _extract_func(name: str) -> ParsedFunc:
    return ParsedFunc(
        name=name,
        return_type="Result*",
        parameters=_params("uint32_t", "fz_context*", "fz_document*", "void*"),
        file="test.hpp",
    )


def _validator(
    classify: list[UserFunc] | None = None,
    extract: list[UserFunc] | None = None,
    tmp: Path | None = None,
) -> UserFuncValidator:
    if classify is None:
        classify = [UserFunc("test.hpp", "obj", "classify")]
    if extract is None:
        extract = [UserFunc("test.hpp", "obj", "extract")]
    if tmp is None:
        tmp = Path(tempfile.mkdtemp()) / "CMakeLists.txt"
    return UserFuncValidator(classify, extract, tmp)


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------

@pytest.fixture
def tmp_cmake() -> Path:
    return Path(tempfile.mkdtemp()) / "CMakeLists.txt"


@pytest.fixture
def val(tmp_cmake: Path) -> UserFuncValidator:
    return _validator(tmp=tmp_cmake)


# ---------------------------------------------------------------------------
# ParsedFunc dataclass
# ---------------------------------------------------------------------------

class TestParsedFunc:
    def test_fields_stored(self):
        params = _params("uint32_t")
        f = ParsedFunc(name="foo", return_type="int", parameters=params, file="a.h")
        assert f.name == "foo"
        assert f.return_type == "int"
        assert f.parameters == params
        assert f.file == "a.h"


# ---------------------------------------------------------------------------
# _validate_func_param_types
# ---------------------------------------------------------------------------

class TestValidateFuncParamTypes:
    def test_exact_match_returns_true(self, val: UserFuncValidator):
        params = _params("uint32_t", "fz_context*", "fz_document*")
        assert val._validate_func_param_types(["uint32_t", "fz_context*", "fz_document*"], params) is True

    def test_wrong_type_returns_false(self, val: UserFuncValidator):
        params = _params("int", "fz_context*", "fz_document*")
        assert val._validate_func_param_types(["uint32_t", "fz_context*", "fz_document*"], params) is False

    def test_empty_lists_return_true(self, val: UserFuncValidator):
        assert val._validate_func_param_types([], []) is True

    def test_partial_match_up_to_zip_length(self, val: UserFuncValidator):
        # zip stops at the shortest; extra params are ignored
        params = _params("uint32_t", "fz_context*")
        assert val._validate_func_param_types(["uint32_t", "fz_context*"], params) is True


# ---------------------------------------------------------------------------
# _validate_func (name / return type / param count / param types)
# ---------------------------------------------------------------------------

class TestValidateFunc:
    def test_valid_classify_func_returns_true(self, val: UserFuncValidator):
        func = _classify_func("classify")
        expected = [UserFunc("test.hpp", "obj", "classify")]
        assert val._validate_func(func, expected, CLASSIFY_SYNTAX) is True

    def test_wrong_name_returns_false(self, val: UserFuncValidator):
        func = _classify_func("wrong_name")
        expected = [UserFunc("test.hpp", "obj", "classify")]
        assert val._validate_func(func, expected, CLASSIFY_SYNTAX) is False

    def test_wrong_return_type_returns_false(self, val: UserFuncValidator):
        func = ParsedFunc(
            name="classify",
            return_type="void*",
            parameters=_params("uint32_t", "fz_context*", "fz_document*"),
            file="test.hpp",
        )
        expected = [UserFunc("test.hpp", "obj", "classify")]
        assert val._validate_func(func, expected, CLASSIFY_SYNTAX) is False

    def test_wrong_param_count_returns_false(self, val: UserFuncValidator):
        func = ParsedFunc(
            name="classify",
            return_type="Result*",
            parameters=_params("uint32_t"),  # too few
            file="test.hpp",
        )
        expected = [UserFunc("test.hpp", "obj", "classify")]
        assert val._validate_func(func, expected, CLASSIFY_SYNTAX) is False

    def test_wrong_param_types_returns_false(self, val: UserFuncValidator):
        func = ParsedFunc(
            name="classify",
            return_type="Result*",
            parameters=_params("int", "fz_context*", "fz_document*"),  # int instead of uint32_t
            file="test.hpp",
        )
        expected = [UserFunc("test.hpp", "obj", "classify")]
        assert val._validate_func(func, expected, CLASSIFY_SYNTAX) is False

    def test_matches_first_in_list(self, val: UserFuncValidator):
        func = _classify_func("classify")
        expected = [
            UserFunc("test.hpp", "obj", "other_func"),
            UserFunc("test.hpp", "obj", "classify"),
        ]
        assert val._validate_func(func, expected, CLASSIFY_SYNTAX) is True

    def test_empty_expected_list_returns_false(self, val: UserFuncValidator):
        func = _classify_func("classify")
        assert val._validate_func(func, [], CLASSIFY_SYNTAX) is False

    def test_extract_syntax_uses_four_params(self, val: UserFuncValidator):
        """Critical regression: extract funcs need void* as 4th param."""
        func = _extract_func("extract")
        expected = [UserFunc("test.hpp", "obj", "extract")]
        assert val._validate_func(func, expected, EXTRACT_SYNTAX) is True

    def test_classify_syntax_rejects_extract_params(self, val: UserFuncValidator):
        """Extract func (4 params) should fail classify syntax (3 params)."""
        func = _extract_func("classify")
        expected = [UserFunc("test.hpp", "obj", "classify")]
        assert val._validate_func(func, expected, CLASSIFY_SYNTAX) is False


# ---------------------------------------------------------------------------
# _validate_classify_func / _validate_extract_func
# ---------------------------------------------------------------------------

class TestValidateClassifyAndExtractFunc:
    def test_valid_classify_accepted(self, tmp_cmake: Path):
        val = UserFuncValidator(
            [UserFunc("test.hpp", "obj", "classify")],
            [UserFunc("test.hpp", "obj", "extract")],
            tmp_cmake,
        )
        assert val._validate_classify_func(_classify_func("classify")) is True

    def test_valid_extract_accepted(self, tmp_cmake: Path):
        val = UserFuncValidator(
            [UserFunc("test.hpp", "obj", "classify")],
            [UserFunc("test.hpp", "obj", "extract")],
            tmp_cmake,
        )
        assert val._validate_extract_func(_extract_func("extract")) is True

    def test_extract_func_rejected_by_classify_validator(self, tmp_cmake: Path):
        """An extract func (4 params) must not pass as a classify func (3 params)."""
        val = UserFuncValidator(
            [UserFunc("test.hpp", "obj", "extract")],
            [],
            tmp_cmake,
        )
        assert val._validate_classify_func(_extract_func("extract")) is False

    def test_classify_func_rejected_by_extract_validator(self, tmp_cmake: Path):
        """A classify func (3 params) must not pass as an extract func (4 params)."""
        val = UserFuncValidator(
            [],
            [UserFunc("test.hpp", "obj", "classify")],
            tmp_cmake,
        )
        assert val._validate_extract_func(_classify_func("classify")) is False


# ---------------------------------------------------------------------------
# _get_available_functions — parses real header files
# ---------------------------------------------------------------------------

class TestGetAvailableFunctions:
    def _write_header(self, tmp_dir: Path, name: str, content: str) -> Path:
        p = tmp_dir / name
        p.write_text(content)
        return p

    def test_parses_classify_signature(self):
        tmp = Path(tempfile.mkdtemp())
        self._write_header(tmp, "test.hpp", (
            "Result* classify(uint32_t page_num, fz_context* ctx, fz_document* doc);\n"
        ))
        val = _validator(tmp=tmp / "CMakeLists.txt")
        funcs = val._get_available_functions()
        names = [f.name for f in funcs]
        assert "classify" in names

    def test_parses_extract_signature(self):
        tmp = Path(tempfile.mkdtemp())
        self._write_header(tmp, "test.hpp", (
            "Result* extract(uint32_t page_num, fz_context* ctx, fz_document* doc, void* shared);\n"
        ))
        val = _validator(tmp=tmp / "CMakeLists.txt")
        funcs = val._get_available_functions()
        names = [f.name for f in funcs]
        assert "extract" in names

    def test_return_type_captured(self):
        tmp = Path(tempfile.mkdtemp())
        self._write_header(tmp, "test.hpp", (
            "Result* classify(uint32_t page_num, fz_context* ctx, fz_document* doc);\n"
        ))
        val = _validator(tmp=tmp / "CMakeLists.txt")
        funcs = val._get_available_functions()
        classify = next(f for f in funcs if f.name == "classify")
        assert classify.return_type == "Result*"

    def test_param_types_captured(self):
        tmp = Path(tempfile.mkdtemp())
        self._write_header(tmp, "test.hpp", (
            "Result* classify(uint32_t page_num, fz_context* ctx, fz_document* doc);\n"
        ))
        val = _validator(tmp=tmp / "CMakeLists.txt")
        funcs = val._get_available_functions()
        classify = next(f for f in funcs if f.name == "classify")
        types = [p["type"] for p in classify.parameters]
        assert types == ["uint32_t", "fz_context*", "fz_document*"]

    def test_empty_directory_returns_empty_list(self):
        tmp = Path(tempfile.mkdtemp())
        val = _validator(tmp=tmp / "CMakeLists.txt")
        assert val._get_available_functions() == []

    def test_multiple_headers_parsed(self):
        tmp = Path(tempfile.mkdtemp())
        self._write_header(tmp, "a.hpp", "Result* classify(uint32_t n, fz_context* c, fz_document* d);\n")
        self._write_header(tmp, "b.hpp", "Result* extract(uint32_t n, fz_context* c, fz_document* d, void* s);\n")
        val = _validator(tmp=tmp / "CMakeLists.txt")
        names = [f.name for f in val._get_available_functions()]
        assert "classify" in names
        assert "extract" in names

    def test_returns_parsed_func_instances(self):
        tmp = Path(tempfile.mkdtemp())
        self._write_header(tmp, "test.hpp", "Result* classify(uint32_t n, fz_context* c, fz_document* d);\n")
        val = _validator(tmp=tmp / "CMakeLists.txt")
        funcs = val._get_available_functions()
        assert all(isinstance(f, ParsedFunc) for f in funcs)


# ---------------------------------------------------------------------------
# _validate_expected_funcs_exist — integration
# ---------------------------------------------------------------------------

class TestValidateExpectedFuncsExist:
    def _make_header(self, tmp: Path) -> None:
        (tmp / "test.hpp").write_text(
            "Result* classify(uint32_t n, fz_context* ctx, fz_document* doc);\n"
            "Result* extract(uint32_t n, fz_context* ctx, fz_document* doc, void* shared);\n"
        )

    def test_passes_when_all_funcs_found(self):
        tmp = Path(tempfile.mkdtemp())
        self._make_header(tmp)
        val = UserFuncValidator(
            [UserFunc("test.hpp", "obj", "classify")],
            [UserFunc("test.hpp", "obj", "extract")],
            tmp / "CMakeLists.txt",
        )
        val.validate()  # must not raise

    def test_raises_when_classify_missing(self):
        tmp = Path(tempfile.mkdtemp())
        (tmp / "test.hpp").write_text(
            "Result* extract(uint32_t n, fz_context* ctx, fz_document* doc, void* shared);\n"
        )
        val = UserFuncValidator(
            [UserFunc("test.hpp", "obj", "classify")],
            [UserFunc("test.hpp", "obj", "extract")],
            tmp / "CMakeLists.txt",
        )
        with pytest.raises(RuntimeError, match="classify"):
            val.validate()

    def test_raises_when_extract_missing(self):
        tmp = Path(tempfile.mkdtemp())
        (tmp / "test.hpp").write_text(
            "Result* classify(uint32_t n, fz_context* ctx, fz_document* doc);\n"
        )
        val = UserFuncValidator(
            [UserFunc("test.hpp", "obj", "classify")],
            [UserFunc("test.hpp", "obj", "extract")],
            tmp / "CMakeLists.txt",
        )
        with pytest.raises(RuntimeError, match="extract"):
            val.validate()

    def test_deduplicates_expected_func_names(self):
        """Duplicate UserFunc entries for the same function name count as one expected."""
        tmp = Path(tempfile.mkdtemp())
        self._make_header(tmp)
        val = UserFuncValidator(
            [UserFunc("a.hpp", "obj", "classify"), UserFunc("b.hpp", "obj", "classify")],
            [UserFunc("test.hpp", "obj", "extract")],
            tmp / "CMakeLists.txt",
        )
        val.validate()  # must not raise — 1 unique name, 1 found
