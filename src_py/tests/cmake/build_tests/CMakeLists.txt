cmake_minimum_required(VERSION 4.2.0)
project(build_test)

add_library(cmake_test STATIC main.cpp test.hpp)

install(TARGETS cmake_test 
    LIBRARY DESTINATION lib
    ARCHIVE DESTINATION lib
)

install(FILES test.hpp DESTINATION include/generated)