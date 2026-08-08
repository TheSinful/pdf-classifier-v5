cmake_minimum_required(VERSION 3.20)
project(pdf_classifier_lib CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

find_package(mupdf REQUIRED)

include(FetchContent)
FetchContent_Declare(nlohmann_json
    GIT_REPOSITORY "https://github.com/nlohmann/json"
    GIT_TAG "v3.12.0"
)
FetchContent_MakeAvailable(nlohmann_json)

add_library(pdf_classifier_lib STATIC string_utils.cpp)

target_link_libraries(pdf_classifier_lib
    PUBLIC mupdf::mupdf nlohmann_json::nlohmann_json)

target_include_directories(pdf_classifier_lib PUBLIC
    $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/..>
    $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}>
    $<INSTALL_INTERFACE:include>
    ${CMAKE_INSTALL_PREFIX}/include) # for <shared/result.h>

target_sources(pdf_classifier_lib
    PUBLIC FILE_SET HEADERS
    BASE_DIRS ${CMAKE_CURRENT_SOURCE_DIR}
    FILES object.hpp wrappers.hpp result.hpp util.hpp string_utils.hpp)

install(TARGETS pdf_classifier_lib
    EXPORT pdf_classifier_libTargets
    FILE_SET HEADERS DESTINATION include/pdf_classifier_lib
    ARCHIVE DESTINATION lib)

option(PDF_CLASSIFIER_LIB_TESTS "Build pdf_classifier_lib tests" ON)

if(PDF_CLASSIFIER_LIB_TESTS)
    enable_testing()
    add_subdirectory(tests)
endif()
