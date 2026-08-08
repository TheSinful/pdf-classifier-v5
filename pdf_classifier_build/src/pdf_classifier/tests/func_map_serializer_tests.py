import pytest
import tempfile
from pathlib import Path
from pdf_classifier.func_map_validator import (
    UserFuncValidator, ParsedFunc, CLASSIFY_SYNTAX, EXTRACT_SYNTAX,
)
from pdf_classifier.user_func import UserFunc, FuncSyntax


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

# The free-function form the shim used before classify/extract became virtuals
# on Object. Still supported by passing the syntaxes explicitly.
LEGACY_CLASSIFY_SYNTAX = FuncSyntax("Result*", ["uint32_t", "fz_context*", "fz_document*"])
LEGACY_EXTRACT_SYNTAX = FuncSyntax("Result*", ["uint32_t", "fz_context*", "fz_document*", "void*"])

MEMBER_HEADER = (
    "class Widget : public Object {\n"
    "public:\n"
    "  explicit Widget(uint32_t page) : Object(page) {}\n"
    "  ~Widget() = default;\n"
    "  ClassificationResult classify(Attached& att) override;\n"
    "  ExtractionResult extract(Attached& att) override;\n"
    "};\n"
)


def _params(*types: str) -> list[dict[str, str]]:
    """Build a parameters list where each entry has a dummy name."""
    return [{"type": t, "name": f"p{i}"} for i, t in enumerate(types)]


def _classify_func(name: str) -> ParsedFunc:
    """A legacy free-function classify declaration."""
    return ParsedFunc(
        name=name,
        return_type="Result*",
        parameters=_params("uint32_t", "fz_context*", "fz_document*"),
        file="test.hpp",
    )


def _extract_func(name: str) -> ParsedFunc:
    """A legacy free-function extract declaration."""
    return ParsedFunc(
        name=name,
        return_type="Result*",
        parameters=_params("uint32_t", "fz_context*", "fz_document*", "void*"),
        file="test.hpp",
    )


def _member_classify(class_name: str = "Widget") -> ParsedFunc:
    return ParsedFunc(
        name="classify",
        return_type="ClassificationResult",
        parameters=_params("Attached&"),
        file="test.hpp",
        class_name=class_name,
    )


def _member_extract(class_name: str = "Widget") -> ParsedFunc:
    return ParsedFunc(
        name="extract",
        return_type="ExtractionResult",
        parameters=_params("Attached&"),
        file="test.hpp",
        class_name=class_name,
    )


def _validator(
    classify: list[UserFunc] | None = None,
    extract: list[UserFunc] | None = None,
    tmp: Path | None = None,
    classify_syntax: FuncSyntax = CLASSIFY_SYNTAX,
    extract_syntax: FuncSyntax = EXTRACT_SYNTAX,
) -> UserFuncValidator:
    if classify is None:
        classify = [UserFunc("test.hpp", "widget", "classify_widget")]
    if extract is None:
        extract = [UserFunc("test.hpp", "widget", "extract_widget")]
    if tmp is None:
        tmp = Path(tempfile.mkdtemp()) / "CMakeLists.txt"
    return UserFuncValidator(classify, extract, tmp, classify_syntax, extract_syntax)


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

    def test_class_name_defaults_to_none(self):
        assert ParsedFunc(name="foo", return_type="int", parameters=[], file="a.h").class_name is None

    def test_qualified_name_includes_class(self):
        assert _member_classify("Widget").qualified_name == "Widget::classify"

    def test_qualified_name_is_bare_for_free_functions(self):
        assert _classify_func("classify_widget").qualified_name == "classify_widget"


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

    def test_reference_spacing_is_normalized(self, val: UserFuncValidator):
        """`Attached &` and `Attached&` name the same type."""
        assert val._validate_func_param_types(["Attached&"], _params("Attached &")) is True


# ---------------------------------------------------------------------------
# _validate_func — member form (the current contract)
# ---------------------------------------------------------------------------

class TestValidateMemberFunc:
    def test_valid_classify_method_returns_true(self, val: UserFuncValidator):
        expected = [UserFunc("test.hpp", "widget", "classify_widget")]
        assert val._validate_func(_member_classify(), expected, CLASSIFY_SYNTAX) is True

    def test_valid_extract_method_returns_true(self, val: UserFuncValidator):
        expected = [UserFunc("test.hpp", "widget", "extract_widget")]
        assert val._validate_func(_member_extract(), expected, EXTRACT_SYNTAX) is True

    def test_wrong_class_returns_false(self, val: UserFuncValidator):
        expected = [UserFunc("test.hpp", "widget", "classify_widget")]
        assert val._validate_func(_member_classify("Gadget"), expected, CLASSIFY_SYNTAX) is False

    def test_class_matched_ignoring_case_and_underscores(self, val: UserFuncValidator):
        """DataTable satisfies the object named "datatable" with no extra config."""
        expected = [UserFunc("test.hpp", "datatable", "classify_datatable")]
        assert val._validate_func(_member_classify("DataTable"), expected, CLASSIFY_SYNTAX) is True

    def test_explicit_cpp_class_is_matched_exactly(self, val: UserFuncValidator):
        expected = [UserFunc("test.hpp", "widget", "classify_widget", "Doohickey")]
        assert val._validate_func(_member_classify("Doohickey"), expected, CLASSIFY_SYNTAX) is True

    def test_explicit_cpp_class_rejects_the_normalized_fallback(self, val: UserFuncValidator):
        """Naming the class explicitly turns off the fuzzy comparison."""
        expected = [UserFunc("test.hpp", "widget", "classify_widget", "Doohickey")]
        assert val._validate_func(_member_classify("Widget"), expected, CLASSIFY_SYNTAX) is False

    def test_free_function_does_not_satisfy_member_expectation(self, val: UserFuncValidator):
        """A namespace-scope classify() is not an override of Object::classify."""
        func = ParsedFunc(
            name="classify",
            return_type="ClassificationResult",
            parameters=_params("Attached&"),
            file="test.hpp",
            class_name=None,
        )
        expected = [UserFunc("test.hpp", "widget", "classify_widget")]
        assert val._validate_func(func, expected, CLASSIFY_SYNTAX) is False

    def test_wrong_return_type_returns_false(self, val: UserFuncValidator):
        func = ParsedFunc(
            name="classify",
            return_type="ExtractionResult",
            parameters=_params("Attached&"),
            file="test.hpp",
            class_name="Widget",
        )
        expected = [UserFunc("test.hpp", "widget", "classify_widget")]
        assert val._validate_func(func, expected, CLASSIFY_SYNTAX) is False

    def test_wrong_param_count_returns_false(self, val: UserFuncValidator):
        func = ParsedFunc(
            name="classify",
            return_type="ClassificationResult",
            parameters=_params("Attached&", "int"),
            file="test.hpp",
            class_name="Widget",
        )
        expected = [UserFunc("test.hpp", "widget", "classify_widget")]
        assert val._validate_func(func, expected, CLASSIFY_SYNTAX) is False

    def test_extract_method_rejected_by_classify_syntax(self, val: UserFuncValidator):
        expected = [UserFunc("test.hpp", "widget", "classify_widget")]
        assert val._validate_func(_member_extract(), expected, CLASSIFY_SYNTAX) is False

    def test_matches_first_in_list(self, val: UserFuncValidator):
        expected = [
            UserFunc("test.hpp", "gadget", "classify_gadget"),
            UserFunc("test.hpp", "widget", "classify_widget"),
        ]
        assert val._validate_func(_member_classify(), expected, CLASSIFY_SYNTAX) is True

    def test_empty_expected_list_returns_false(self, val: UserFuncValidator):
        assert val._validate_func(_member_classify(), [], CLASSIFY_SYNTAX) is False


# ---------------------------------------------------------------------------
# _validate_func — legacy free-function form
# ---------------------------------------------------------------------------

class TestValidateLegacyFunc:
    def test_valid_classify_func_returns_true(self, val: UserFuncValidator):
        func = _classify_func("classify")
        expected = [UserFunc("test.hpp", "obj", "classify")]
        assert val._validate_func(func, expected, LEGACY_CLASSIFY_SYNTAX) is True

    def test_wrong_name_returns_false(self, val: UserFuncValidator):
        func = _classify_func("wrong_name")
        expected = [UserFunc("test.hpp", "obj", "classify")]
        assert val._validate_func(func, expected, LEGACY_CLASSIFY_SYNTAX) is False

    def test_wrong_return_type_returns_false(self, val: UserFuncValidator):
        func = ParsedFunc(
            name="classify",
            return_type="void*",
            parameters=_params("uint32_t", "fz_context*", "fz_document*"),
            file="test.hpp",
        )
        expected = [UserFunc("test.hpp", "obj", "classify")]
        assert val._validate_func(func, expected, LEGACY_CLASSIFY_SYNTAX) is False

    def test_wrong_param_count_returns_false(self, val: UserFuncValidator):
        func = ParsedFunc(
            name="classify",
            return_type="Result*",
            parameters=_params("uint32_t"),  # too few
            file="test.hpp",
        )
        expected = [UserFunc("test.hpp", "obj", "classify")]
        assert val._validate_func(func, expected, LEGACY_CLASSIFY_SYNTAX) is False

    def test_wrong_param_types_returns_false(self, val: UserFuncValidator):
        func = ParsedFunc(
            name="classify",
            return_type="Result*",
            parameters=_params("int", "fz_context*", "fz_document*"),  # int instead of uint32_t
            file="test.hpp",
        )
        expected = [UserFunc("test.hpp", "obj", "classify")]
        assert val._validate_func(func, expected, LEGACY_CLASSIFY_SYNTAX) is False

    def test_member_does_not_satisfy_legacy_expectation(self, val: UserFuncValidator):
        """A method that happens to share the name is still not a free function."""
        func = ParsedFunc(
            name="classify",
            return_type="Result*",
            parameters=_params("uint32_t", "fz_context*", "fz_document*"),
            file="test.hpp",
            class_name="Widget",
        )
        expected = [UserFunc("test.hpp", "obj", "classify")]
        assert val._validate_func(func, expected, LEGACY_CLASSIFY_SYNTAX) is False

    def test_extract_syntax_uses_four_params(self, val: UserFuncValidator):
        """Critical regression: extract funcs need void* as 4th param."""
        func = _extract_func("extract")
        expected = [UserFunc("test.hpp", "obj", "extract")]
        assert val._validate_func(func, expected, LEGACY_EXTRACT_SYNTAX) is True

    def test_classify_syntax_rejects_extract_params(self, val: UserFuncValidator):
        """Extract func (4 params) should fail classify syntax (3 params)."""
        func = _extract_func("classify")
        expected = [UserFunc("test.hpp", "obj", "classify")]
        assert val._validate_func(func, expected, LEGACY_CLASSIFY_SYNTAX) is False


# ---------------------------------------------------------------------------
# _validate_classify_func / _validate_extract_func
# ---------------------------------------------------------------------------

class TestValidateClassifyAndExtractFunc:
    def test_valid_classify_accepted(self, tmp_cmake: Path):
        val = _validator(tmp=tmp_cmake)
        assert val._validate_classify_func(_member_classify()) is True

    def test_valid_extract_accepted(self, tmp_cmake: Path):
        val = _validator(tmp=tmp_cmake)
        assert val._validate_extract_func(_member_extract()) is True

    def test_extract_method_rejected_by_classify_validator(self, tmp_cmake: Path):
        val = _validator(tmp=tmp_cmake)
        assert val._validate_classify_func(_member_extract()) is False

    def test_classify_method_rejected_by_extract_validator(self, tmp_cmake: Path):
        val = _validator(tmp=tmp_cmake)
        assert val._validate_extract_func(_member_classify()) is False


# ---------------------------------------------------------------------------
# _get_available_functions — parses real header files
# ---------------------------------------------------------------------------

class TestGetAvailableFunctions:
    def _write_header(self, tmp_dir: Path, name: str, content: str) -> Path:
        p = tmp_dir / name
        p.write_text(content)
        return p

    def _funcs(self, content: str, name: str = "test.hpp") -> list[ParsedFunc]:
        tmp = Path(tempfile.mkdtemp())
        self._write_header(tmp, name, content)
        return _validator(tmp=tmp / "CMakeLists.txt")._get_available_functions()

    def test_parses_member_classify(self):
        funcs = self._funcs(MEMBER_HEADER)
        assert "Widget::classify" in [f.qualified_name for f in funcs]

    def test_parses_member_extract(self):
        funcs = self._funcs(MEMBER_HEADER)
        assert "Widget::extract" in [f.qualified_name for f in funcs]

    def test_member_return_type_captured(self):
        classify = next(f for f in self._funcs(MEMBER_HEADER) if f.name == "classify")
        assert classify.return_type == "ClassificationResult"

    def test_member_param_types_captured(self):
        classify = next(f for f in self._funcs(MEMBER_HEADER) if f.name == "classify")
        assert [p["type"] for p in classify.parameters] == ["Attached&"]

    def test_unnamed_parameter_is_parsed(self):
        """Overrides are often declared as classify(Attached&) with no arg name."""
        funcs = self._funcs("class Widget { ClassificationResult classify(Attached&) override; };")
        classify = next(f for f in funcs if f.name == "classify")
        assert [p["type"] for p in classify.parameters] == ["Attached&"]

    def test_pure_virtual_is_parsed(self):
        funcs = self._funcs("class Object { virtual ClassificationResult classify(Attached&) = 0; };")
        classify = next(f for f in funcs if f.name == "classify")
        assert classify.class_name == "Object"

    def test_constructor_and_destructor_are_ignored(self):
        """Neither has a return type, so there is nothing here to validate - and
        `~Widget()` must not be mis-parsed as `Widge* t()`."""
        names = [f.name for f in self._funcs(MEMBER_HEADER)]
        assert "Widget" not in names
        assert "t" not in names

    def test_methods_attributed_to_their_own_class(self):
        funcs = self._funcs(
            "class A { ClassificationResult classify(Attached&) override; };\n"
            "class B { ClassificationResult classify(Attached&) override; };\n"
        )
        assert sorted(f.qualified_name for f in funcs) == ["A::classify", "B::classify"]

    def test_free_function_has_no_class(self):
        funcs = self._funcs("Result* classify_widget(uint32_t n, fz_context* c, fz_document* d);\n")
        assert next(f for f in funcs if f.name == "classify_widget").class_name is None

    def test_declarations_after_a_class_are_not_attributed_to_it(self):
        funcs = self._funcs(
            "class Widget { ClassificationResult classify(Attached&) override; };\n"
            "Result* classify_widget(uint32_t n, fz_context* c, fz_document* d);\n"
        )
        assert next(f for f in funcs if f.name == "classify_widget").class_name is None

    def test_inline_definitions_do_not_leak_scope(self):
        """A method body opens a brace of its own; the class must survive it."""
        funcs = self._funcs(
            "class Widget {\n"
            "  bool ready() const { return true; }\n"
            "  ClassificationResult classify(Attached&) override;\n"
            "};\n"
        )
        assert next(f for f in funcs if f.name == "classify").class_name == "Widget"

    def test_comments_are_not_parsed(self):
        funcs = self._funcs(
            "/// Call ClassificationResult classify(Attached& att); to do the work.\n"
            "// Result* classify_widget(uint32_t n, fz_context* c, fz_document* d);\n"
        )
        assert funcs == []

    def test_preprocessor_lines_are_ignored(self):
        """An #if branch can open a brace that never closes in that branch."""
        funcs = self._funcs(
            "#pragma once\n"
            "#define WRAP(x) do { x; } while (0)\n"
            "class Widget { ClassificationResult classify(Attached&) override; };\n"
        )
        assert next(f for f in funcs if f.name == "classify").class_name == "Widget"

    def test_enum_class_does_not_open_a_class_scope(self):
        funcs = self._funcs(
            "enum class Color { RED, GREEN };\n"
            "class Widget { ClassificationResult classify(Attached&) override; };\n"
        )
        assert next(f for f in funcs if f.name == "classify").class_name == "Widget"

    def test_templated_param_commas_are_not_split(self):
        funcs = self._funcs("class Widget { void take(std::pair<int, int> p, int q); };")
        take = next(f for f in funcs if f.name == "take")
        assert [p["type"] for p in take.parameters] == ["std::pair<int, int>", "int"]

    def test_multi_word_builtin_param_type(self):
        funcs = self._funcs("class Widget { bool white(const unsigned char* pixel); };")
        white = next(f for f in funcs if f.name == "white")
        assert [p["type"] for p in white.parameters] == ["const unsigned char*"]

    def test_empty_directory_returns_empty_list(self):
        tmp = Path(tempfile.mkdtemp())
        val = _validator(tmp=tmp / "CMakeLists.txt")
        assert val._get_available_functions() == []

    def test_multiple_headers_parsed(self):
        tmp = Path(tempfile.mkdtemp())
        self._write_header(tmp, "a.hpp", "class A { ClassificationResult classify(Attached&) override; };")
        self._write_header(tmp, "b.hpp", "class B { ExtractionResult extract(Attached&) override; };")
        val = _validator(tmp=tmp / "CMakeLists.txt")
        assert sorted(f.qualified_name for f in val._get_available_functions()) == ["A::classify", "B::extract"]

    def test_returns_parsed_func_instances(self):
        assert all(isinstance(f, ParsedFunc) for f in self._funcs(MEMBER_HEADER))


# ---------------------------------------------------------------------------
# validate() — integration
# ---------------------------------------------------------------------------

class TestValidate:
    def _expected(self, obj: str = "widget") -> tuple[list[UserFunc], list[UserFunc]]:
        return ([UserFunc("test.hpp", obj, f"classify_{obj}")],
                [UserFunc("test.hpp", obj, f"extract_{obj}")])

    def test_passes_when_both_methods_found(self):
        tmp = Path(tempfile.mkdtemp())
        (tmp / "test.hpp").write_text(MEMBER_HEADER)
        classify, extract = self._expected()
        _validator(classify, extract, tmp / "CMakeLists.txt").validate()  # must not raise

    def test_raises_when_classify_missing(self):
        tmp = Path(tempfile.mkdtemp())
        (tmp / "test.hpp").write_text("class Widget { ExtractionResult extract(Attached&) override; };")
        classify, extract = self._expected()
        with pytest.raises(RuntimeError, match="classify"):
            _validator(classify, extract, tmp / "CMakeLists.txt").validate()

    def test_raises_when_extract_missing(self):
        tmp = Path(tempfile.mkdtemp())
        (tmp / "test.hpp").write_text("class Widget { ClassificationResult classify(Attached&) override; };")
        classify, extract = self._expected()
        with pytest.raises(RuntimeError, match="extract"):
            _validator(classify, extract, tmp / "CMakeLists.txt").validate()

    def test_raises_when_class_is_missing_entirely(self):
        tmp = Path(tempfile.mkdtemp())
        (tmp / "test.hpp").write_text(MEMBER_HEADER)
        classify, extract = self._expected("gadget")
        with pytest.raises(RuntimeError, match="gadget"):
            _validator(classify, extract, tmp / "CMakeLists.txt").validate()

    def test_error_names_the_object_and_its_header(self):
        tmp = Path(tempfile.mkdtemp())
        (tmp / "test.hpp").write_text("class Widget {};")
        classify, extract = self._expected()
        with pytest.raises(RuntimeError) as exc:
            _validator(classify, extract, tmp / "CMakeLists.txt").validate()
        assert "widget" in str(exc.value)
        assert "test.hpp" in str(exc.value)

    def test_raises_when_override_signature_is_wrong(self):
        """The whole point of the check: a wrong override is caught here rather
        than as a "cannot instantiate abstract class" error from the compiler."""
        tmp = Path(tempfile.mkdtemp())
        (tmp / "test.hpp").write_text(
            "class Widget {\n"
            "  ClassificationResult classify(Attached&, int extra) override;\n"
            "  ExtractionResult extract(Attached&) override;\n"
            "};\n"
        )
        classify, extract = self._expected()
        with pytest.raises(RuntimeError, match="classify"):
            _validator(classify, extract, tmp / "CMakeLists.txt").validate()

    def test_deduplicates_expected_objects(self):
        """Duplicate UserFunc entries for the same object count as one expected."""
        tmp = Path(tempfile.mkdtemp())
        (tmp / "test.hpp").write_text(MEMBER_HEADER)
        val = _validator(
            [UserFunc("a.hpp", "widget", "classify_widget"), UserFunc("b.hpp", "widget", "classify_widget")],
            [UserFunc("test.hpp", "widget", "extract_widget")],
            tmp / "CMakeLists.txt",
        )
        val.validate()  # must not raise — 1 unique object, 1 found

    def test_legacy_free_function_project_still_validates(self):
        """Passing the old syntaxes keeps a not-yet-migrated project buildable."""
        tmp = Path(tempfile.mkdtemp())
        (tmp / "test.hpp").write_text(
            "Result* classify_widget(uint32_t n, fz_context* ctx, fz_document* doc);\n"
            "Result* extract_widget(uint32_t n, fz_context* ctx, fz_document* doc, void* shared);\n"
        )
        classify, extract = self._expected()
        val = _validator(classify, extract, tmp / "CMakeLists.txt",
                         LEGACY_CLASSIFY_SYNTAX, LEGACY_EXTRACT_SYNTAX)
        val.validate()  # must not raise


# ---------------------------------------------------------------------------
# Inheritance — an override may live on a base, which emits no declaration in
# the derived class for the parser to find.
# ---------------------------------------------------------------------------

INHERITED_HEADER = (
    "class ColorObject : public Object {\n"
    "public:\n"
    "  ColorObject(uint32_t page, Rgb expected, const char* name);\n"
    "  ClassificationResult classify(Attached& att) override;\n"
    "  ExtractionResult extract(Attached& att) override;\n"
    "};\n"
    "class Widget : public ColorObject {\n"
    "public:\n"
    "  explicit Widget(uint32_t page) : ColorObject(page, RED, \"widget\") {}\n"
    "};\n"
)


class TestInheritance:
    def _validator_for(self, content: str, obj: str = "widget") -> UserFuncValidator:
        tmp = Path(tempfile.mkdtemp())
        (tmp / "test.hpp").write_text(content)
        return _validator(
            [UserFunc("test.hpp", obj, f"classify_{obj}")],
            [UserFunc("test.hpp", obj, f"extract_{obj}")],
            tmp / "CMakeLists.txt",
        )

    def test_base_clause_is_parsed(self):
        _, hierarchy = self._validator_for(INHERITED_HEADER)._scan_project()
        assert hierarchy["Widget"] == ["ColorObject"]
        assert hierarchy["ColorObject"] == ["Object"]

    def test_access_specifiers_stripped_from_bases(self):
        _, hierarchy = self._validator_for("class A : private virtual B, public C {};")._scan_project()
        assert hierarchy["A"] == ["B", "C"]

    def test_template_bases_reduced_to_their_name(self):
        _, hierarchy = self._validator_for("class A : public detail::Mixin<int, char>, public B {};")._scan_project()
        assert hierarchy["A"] == ["Mixin", "B"]

    def test_inherited_override_satisfies_the_derived_object(self):
        self._validator_for(INHERITED_HEADER).validate()  # must not raise

    def test_inherited_override_across_two_levels(self):
        self._validator_for(
            INHERITED_HEADER + "class Gadget : public Widget {};\n", "gadget"
        ).validate()  # must not raise

    def test_base_with_wrong_signature_still_fails(self):
        """Inheritance widens where we look, it does not weaken what we accept."""
        with pytest.raises(RuntimeError, match="classify"):
            self._validator_for(
                "class ColorObject : public Object {\n"
                "  ClassificationResult classify(Attached&, int extra) override;\n"
                "  ExtractionResult extract(Attached&) override;\n"
                "};\n"
                "class Widget : public ColorObject {};\n"
            ).validate()

    def test_error_names_the_chain_that_was_searched(self):
        with pytest.raises(RuntimeError) as exc:
            self._validator_for(
                "class ColorObject : public Object {};\n"
                "class Widget : public ColorObject {};\n"
            ).validate()
        assert "Widget -> ColorObject -> Object" in str(exc.value)

    def test_reaching_the_library_base_is_a_definite_failure(self):
        """Object declares both as pure virtuals, so ending there means nobody
        implemented them - that is an error, not an unresolved lookup."""
        with pytest.raises(RuntimeError, match="classify"):
            self._validator_for("class Widget : public Object {};\n").validate()

    def test_unknown_base_downgrades_to_a_warning(self, caplog):
        """A base we cannot see may carry the override; we cannot prove a failure,
        so the compiler gets the last word."""
        val = self._validator_for("class Widget : public SomeExternalBase {};\n")
        val.validate()  # must not raise
        assert "SomeExternalBase" in caplog.text

    def test_missing_class_is_reported_as_such(self):
        with pytest.raises(RuntimeError, match="no such class"):
            self._validator_for(INHERITED_HEADER, "gadget").validate()

    def test_sibling_classes_do_not_satisfy_each_other(self):
        with pytest.raises(RuntimeError, match="classify"):
            self._validator_for(
                "class Other : public Object {\n"
                "  ClassificationResult classify(Attached&) override;\n"
                "  ExtractionResult extract(Attached&) override;\n"
                "};\n"
                "class Widget : public Object {};\n"
            ).validate()

    def test_reference_bound_to_the_name_is_the_same_type(self):
        """`Attached &att` and `Attached& att` differ only in formatting."""
        self._validator_for(
            "class Widget : public Object {\n"
            "  ClassificationResult classify(Attached &att) override;\n"
            "  ExtractionResult extract(Attached &att) override;\n"
            "};\n"
        ).validate()  # must not raise
