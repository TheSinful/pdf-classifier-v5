cmake_minimum_required(VERSION 4.2.0)
project(my_project)

find_package(mupdf REQUIRED)

set(CMAKE_CXX_STANDARD 20)


add_library(my_lib STATIC test.cpp) # Insert .cpp files here 

target_link_libraries(my_lib PRIVATE mupdf::mupdf)

target_include_directories(my_lib PUBLIC
    ${CLASSIFIER_INCLUDE_DIR} # provided during build via build.py 
    ${CMAKE_INSTALL_PREFIX}/include
)

set_target_properties(my_lib PROPERTIES
    PUBLIC_HEADER test.h #insert other header files here 
)

install(TARGETS my_lib
    LIBRARY DESTINATION lib
    ARCHIVE DESTINATION lib
    PUBLIC_HEADER DESTINATION include/generated
)
