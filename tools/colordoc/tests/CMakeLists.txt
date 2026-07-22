include(FetchContent)

FetchContent_Declare(
    googletest
    URL https://github.com/google/googletest/archive/refs/tags/v1.14.0.zip
)
set(gtest_force_shared_crt ON CACHE BOOL "" FORCE)
FetchContent_MakeAvailable(googletest)

# Both are produced by `python generate_doc.py`; main.py regenerates them on
# every run. The tests skip themselves if either is missing.
set(COLORDOC_TEST_PDF_PATH
    "${CMAKE_CURRENT_SOURCE_DIR}/../test_data/fixture.pdf"
    CACHE STRING "Absolute path to the generated colordoc fixture PDF")

set(COLORDOC_TEST_GROUNDTRUTH_PATH
    "${CMAKE_CURRENT_SOURCE_DIR}/../test_data/fixture.json"
    CACHE STRING "Absolute path to the fixture's ground-truth JSON")

add_executable(colordoc_tests
    test_color_page.cpp
    test_classes.cpp
)

set_target_properties(colordoc_tests PROPERTIES CXX_STANDARD 20)

target_compile_definitions(colordoc_tests PRIVATE
    COLORDOC_TEST_PDF_PATH="${COLORDOC_TEST_PDF_PATH}"
    COLORDOC_TEST_GROUNDTRUTH_PATH="${COLORDOC_TEST_GROUNDTRUTH_PATH}"
)

target_link_libraries(colordoc_tests PRIVATE
    classifier_intermediary
    mupdf::mupdf
    GTest::gtest
    GTest::gtest_main
    nlohmann_json::nlohmann_json
)

target_include_directories(colordoc_tests PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}/.. # classes.hpp, color_page.hpp, palette.hpp, util.hpp
    ${CLASSIFIER_INCLUDE_DIR}      # shared/result.h (build-time generated)
    ${CMAKE_INSTALL_PREFIX}/include # shared/result.h (install-time copy)
)

include(GoogleTest)
gtest_discover_tests(colordoc_tests)
