
cmake_minimum_required(VERSION 4.2)
project(pdf_classifier LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)

set(SRC_FILES
    src_cpp/ffi.cpp
)

add_executable(pdf_classifier ${SRC_FILES})

include_directories(src/headers)

target_include_directories(pdf_classifier PRIVATE include)
target_link_directories(pdf_classifier PRIVATE "${CMAKE_SOURCE_DIR}/lib")
