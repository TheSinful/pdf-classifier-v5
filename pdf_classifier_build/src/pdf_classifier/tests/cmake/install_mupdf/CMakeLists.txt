
cmake_minimum_required(VERSION 4.2.0)
project(cmake_test LANGUAGES CXX)

add_library(cmake_test STATIC main.cpp test.h)

include(../../cmake/install_mupdf.cmake)
