cmake_minimum_required(VERSION 4.2.0)
project(my_project) # changeable

find_package(mupdf REQUIRED)

set(CMAKE_CXX_STANDARD 20)
#             shouldn't be change
add_library(classifier_intermediary STATIC test.cpp chapter.cpp subchapter.cpp diagram.cpp table.cpp) # Insert .cpp files here 

target_link_libraries(classifier_intermediary PRIVATE mupdf::mupdf)

target_include_directories(classifier_intermediary PRIVATE
    ${CLASSIFIER_INCLUDE_DIR} # provided during build via build.py 
    ${CMAKE_INSTALL_PREFIX}/include
)

target_sources(classifier_intermediary
    PUBLIC
    FILE_SET HEADERS
    BASE_DIRS ${CMAKE_CURRENT_SOURCE_DIR}
    FILES
    test.hpp
    object.hpp
    chapter.hpp
    util.hpp
    subchapter.hpp
    diagram.hpp
    table.hpp
)


install(TARGETS classifier_intermediary
    FILE_SET HEADERS DESTINATION include/generated
    LIBRARY DESTINATION lib
    ARCHIVE DESTINATION lib
)

enable_testing()

add_subdirectory(tests)
