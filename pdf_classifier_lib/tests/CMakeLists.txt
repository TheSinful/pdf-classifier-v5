include(FetchContent)

FetchContent_Declare(
    googletest
    URL https://github.com/google/googletest/archive/refs/tags/v1.14.0.zip
)

# Match the CRT the rest of the build uses, or MSVC link errors follow.
set(gtest_force_shared_crt ON CACHE BOOL "" FORCE)
FetchContent_MakeAvailable(googletest)

set(TEST_PDF_PATH
    "${CMAKE_CURRENT_SOURCE_DIR}/../../data/large_test_doc.pdf"
    CACHE STRING "Absolute path to the PDF used for library tests")

add_executable(pdf_classifier_lib_tests
    test_wrappers.cpp
    test_string_utils.cpp
    test_result.cpp
    test_object.cpp)

set_target_properties(pdf_classifier_lib_tests PROPERTIES
    CXX_STANDARD 20
    CXX_STANDARD_REQUIRED ON)

target_compile_definitions(pdf_classifier_lib_tests PRIVATE
    TEST_PDF_PATH="${TEST_PDF_PATH}")

# mupdf::mupdf and its include dirs arrive transitively - pdf_classifier_lib
# links it PUBLIC because the public headers include <mupdf/fitz.h>.
target_link_libraries(pdf_classifier_lib_tests PRIVATE
    pdf_classifier_lib
    GTest::gtest
    GTest::gtest_main)

include(GoogleTest)
gtest_discover_tests(pdf_classifier_lib_tests)
