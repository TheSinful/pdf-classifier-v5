include(FetchContent)

FetchContent_Declare(
    googletest
    URL https://github.com/google/googletest/archive/refs/tags/v1.14.0.zip
)
set(gtest_force_shared_crt ON CACHE BOOL "" FORCE)
FetchContent_MakeAvailable(googletest)


set(TEST_PDF_PATH
    "${CMAKE_CURRENT_SOURCE_DIR}/../../data/large_test_doc.pdf"
    CACHE STRING "Absolute path to the PDF used for classifier tests")

set(TEST_BOUNDARY_OUTPUT_PATH
    "${CMAKE_CURRENT_BINARY_DIR}/cell_boundaries.json"
    CACHE STRING "Output path for the cell boundary JSON written by TestDumpCellBoundaries")

add_executable(classifier_tests
    test_chapter.cpp
    test_diagram.cpp
    test_table.cpp
    test_object.cpp
)

set_target_properties(classifier_tests PROPERTIES CXX_STANDARD 20)

target_compile_definitions(classifier_tests PRIVATE
    TEST_PDF_PATH="${TEST_PDF_PATH}"
    TEST_BOUNDARY_OUTPUT_PATH="${TEST_BOUNDARY_OUTPUT_PATH}"
)

target_link_libraries(classifier_tests PRIVATE
    classifier_intermediary
    mupdf::mupdf
    GTest::gtest
    GTest::gtest_main
    nlohmann_json::nlohmann_json
)

target_include_directories(classifier_tests PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}/.. # chapter.hpp, diagram.hpp, table.hpp, object.hpp
    ${CLASSIFIER_INCLUDE_DIR} # shared/result.h (build-time generated)
    ${CMAKE_INSTALL_PREFIX}/include # shared/result.h (install-time copy)
)

include(GoogleTest)
gtest_discover_tests(classifier_tests)
