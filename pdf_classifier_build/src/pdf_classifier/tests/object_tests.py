import pytest
import warnings
from pdf_classifier.object import ObjectFactory, ObjectBuilder

# cpp_class() supersedes classify()/extract(), which are deprecated. The legacy
# tests below call them deliberately.
pytestmark = pytest.mark.filterwarnings("ignore::DeprecationWarning")


def _legacy(builder: ObjectBuilder, obj: str) -> ObjectBuilder:
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        return builder.classify(f"classify_{obj}").extract(f"extract_{obj}")

class TestCppClassDerivesShims:
    def test_build_succeeds_without_classify_or_extract(self):
        f = ObjectFactory("test.hpp")
        f.new().cpp_class("DataTable").build()  # must not raise
        assert len(f._objs) == 1

    def test_derived_shim_names_follow_define_object(self):
        """DEFINE_OBJECT(datatable, DataTable) -> classify_datatable/extract_datatable."""
        f = ObjectFactory("test.hpp")
        f.new().cpp_class("DataTable").build()
        assert f._expected_classify_funcs[0].name == "classify_datatable"
        assert f._expected_extract_funcs[0].name == "extract_datatable"

    def test_cpp_class_recorded_on_both_funcs(self):
        f = ObjectFactory("test.hpp")
        f.new().cpp_class("DataTable").build()
        assert f._expected_classify_funcs[0].cpp_class == "DataTable"
        assert f._expected_extract_funcs[0].cpp_class == "DataTable"

    def test_object_name_defaults_to_the_lowercased_class(self):
        f = ObjectFactory("test.hpp")
        f.new().cpp_class("SubChapter").build()
        assert f._objs[0].name == "subchapter"

    def test_explicit_name_wins_over_the_derived_one(self):
        f = ObjectFactory("test.hpp")
        f.new().name("datatable").cpp_class("DataTable").build()
        assert f._objs[0].name == "datatable"

    def test_shims_follow_a_name_set_after_cpp_class(self):
        """Derivation happens at build(), so the call order cannot matter."""
        f = ObjectFactory("test.hpp")
        f.new().cpp_class("DataTable").name("table").build()
        assert f._expected_classify_funcs[0].name == "classify_table"

    def test_header_set_after_cpp_class_is_still_used(self):
        f = ObjectFactory()
        f.new().cpp_class("DataTable").header("table.hpp").build()
        assert f._expected_classify_funcs[0].file_name == "table.hpp"

    def test_explicit_classify_overrides_the_derived_shim(self):
        f = ObjectFactory("test.hpp")
        b = f.new().name("datatable").cpp_class("DataTable")
        with warnings.catch_warnings():
            warnings.simplefilter("ignore")
            b.classify("my_own_classify")
        b.build()
        assert f._expected_classify_funcs[0].name == "my_own_classify"
        assert f._expected_extract_funcs[0].name == "extract_datatable"

    def test_missing_header_raises_a_named_error(self):
        f = ObjectFactory()
        with pytest.raises(RuntimeError, match="no header"):
            f.new().cpp_class("DataTable").build()

    def test_without_cpp_class_the_funcs_are_still_required(self):
        f = ObjectFactory("test.hpp")
        with pytest.raises(RuntimeError, match="classify function"):
            f.new().name("datatable").build()

class TestPairTo:
    def _paired(self) -> ObjectFactory:
        f = ObjectFactory("test.hpp")
        f.new().cpp_class("Diagram").pair_to("datatable", 1).build()
        f.new().cpp_class("DataTable").pair_to("diagram", 2).build()
        return f

    def test_both_sides_are_linked(self):
        diagram, datatable = self._paired()._objs
        assert diagram.pair is not None and datatable.pair is not None
        assert diagram.pair[0]().name == "datatable"
        assert datatable.pair[0]().name == "diagram"

    def test_order_is_recorded_relative_to_each_object(self):
        """PAIR_TYPE[1] is where THIS object sits, not its partner."""
        diagram, datatable = self._paired()._objs
        assert diagram.pair is not None and datatable.pair is not None
        assert diagram.pair[1] == 1
        assert datatable.pair[1] == 2

    def test_order_one_alone_leaves_both_unpaired(self):
        """Documented hazard: the order=1 call records nothing on its own."""
        f = ObjectFactory("test.hpp")
        f.new().cpp_class("Diagram").pair_to("datatable", 1).build()
        f.new().cpp_class("DataTable").build()
        assert all(o.pair is None for o in f._objs)

    def test_order_one_does_not_validate_its_partner(self):
        f = ObjectFactory("test.hpp")
        f.new().cpp_class("Diagram").pair_to("no_such_object", 1).build()  # must not raise
        assert f._objs[0].pair is None

    def test_order_two_does_validate_its_partner(self):
        f = ObjectFactory("test.hpp")
        with pytest.raises(RuntimeError, match="no_such_object"):
            f.new().cpp_class("DataTable").pair_to("no_such_object", 2)

class TestHierarchy:
    def test_child_of_registers_on_the_parent(self):
        f = ObjectFactory("test.hpp")
        f.new().cpp_class("Chapter").organizational().build()
        f.new().cpp_class("SubChapter").child_of("chapter").build()
        chapter = f._objs[0]
        assert [c().name for c in chapter.children] == ["subchapter"]

    def test_child_of_unknown_parent_raises(self):
        f = ObjectFactory("test.hpp")
        with pytest.raises(RuntimeError, match="nope"):
            f.new().cpp_class("SubChapter").child_of("nope")

    def test_organizational_flag_is_set(self):
        f = ObjectFactory("test.hpp")
        f.new().cpp_class("Chapter").organizational().build()
        assert f._objs[0].is_organizational is True

    def test_objects_default_to_not_organizational(self):
        f = ObjectFactory("test.hpp")
        f.new().cpp_class("Diagram").build()
        assert f._objs[0].is_organizational is False

    def test_factory_header_is_the_default_for_new_builders(self):
        f = ObjectFactory("shared.hpp")
        f.new().cpp_class("Diagram").build()
        assert f._expected_classify_funcs[0].file_name == "shared.hpp"

class TestLegacyForm:
    def test_classify_and_extract_without_cpp_class(self):
        f = ObjectFactory("test.hpp")
        _legacy(f.new().name("chapter"), "chapter").build()
        assert f._expected_classify_funcs[0].name == "classify_chapter"
        assert f._expected_classify_funcs[0].cpp_class == ""

    def test_name_is_required_before_classify(self):
        f = ObjectFactory("test.hpp")
        with pytest.raises(RuntimeError, match="without a name"):
            _legacy(f.new(), "chapter")

    def test_per_call_file_name_overrides_the_header(self):
        f = ObjectFactory("test.hpp")
        b = f.new().name("chapter")
        with warnings.catch_warnings():
            warnings.simplefilter("ignore")
            b.classify("classify_chapter", "other.hpp").extract("extract_chapter")
        b.build()
        assert f._expected_classify_funcs[0].file_name == "other.hpp"
        assert f._expected_extract_funcs[0].file_name == "test.hpp"
