cmake_minimum_required(VERSION 4.2.0)
project(my_project) # changeable

find_package(mupdf REQUIRED)
find_package(nlohmann_json 3.12.0 REQUIRED HINTS "${CMAKE_BINARY_DIR}/_deps/nlohmann_json-build")

set(CMAKE_CXX_STANDARD 20)

set(PDF_CLASSIFIER_LIB_TESTS OFF CACHE BOOL "" FORCE)
add_subdirectory(${CMAKE_CURRENT_SOURCE_DIR}/../pdf_classifier_lib pdf_classifier_lib_build) # ! temp 

add_library(classifier_intermediary STATIC chapter.cpp subchapter.cpp diagram.cpp table.cpp) # Insert .cpp files here 

target_link_libraries(classifier_intermediary PRIVATE mupdf::mupdf PUBLIC nlohmann_json::nlohmann_json)

target_include_directories(classifier_intermediary PRIVATE
    ${CLASSIFIER_INCLUDE_DIR} # provided during build via build.py 
    ${CMAKE_INSTALL_PREFIX}/include
)

target_sources(classifier_intermediary
    PUBLIC
    FILE_SET HEADERS
    BASE_DIRS ${CMAKE_CURRENT_SOURCE_DIR}
    FILES
    chapter.hpp
    subchapter.hpp
    diagram.hpp
    table.hpp
)


install(TARGETS classifier_intermediary
    FILE_SET HEADERS DESTINATION include/generated
    LIBRARY DESTINATION lib
    ARCHIVE DESTINATION lib
)

install(DIRECTORY ${nlohmann_json_SOURCE_DIR}/include/
    DESTINATION include
)

enable_testing()

add_subdirectory(tests)
