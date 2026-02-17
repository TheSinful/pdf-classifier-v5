cmake_minimum_required(VERSION 4.2.0)
project(my_project) # changeable

find_package(mupdf REQUIRED)

set(CMAKE_CXX_STANDARD 20)
#             shouldn't be change
add_library(classifier_intermediary STATIC test.cpp) # Insert .cpp files here 

target_link_libraries(classifier_intermediary PRIVATE mupdf::mupdf)

target_include_directories(classifier_intermediary PRIVATE 
    ${CLASSIFIER_INCLUDE_DIR} # provided during build via build.py 
    ${CMAKE_INSTALL_PREFIX}/include
)

set_target_properties(classifier_intermediary PROPERTIES
    PUBLIC_HEADER test.hpp #insert other header files here 
)

install(TARGETS classifier_intermediary
    LIBRARY DESTINATION lib
    ARCHIVE DESTINATION lib
    PUBLIC_HEADER DESTINATION include/generated
)
