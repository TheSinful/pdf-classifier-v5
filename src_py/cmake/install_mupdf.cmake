cmake_minimum_required(VERSION 3.12.0)

include(FetchContent)

FetchContent_Declare(
    mupdf
    GIT_REPOSITORY https://github.com/ArtifexSoftware/mupdf.git
    GIT_TAG 4b34e2e # 1.26.12
)
FetchContent_MakeAvailable(mupdf)

include(ExternalProject)
if(WIN32)
    ExternalProject_Add(mupdf_build
        SOURCE_DIR ${mupdf_SOURCE_DIR}
        CONFIGURE_COMMAND ""
        BUILD_COMMAND msbuild ${mupdf_SOURCE_DIR}/platform/win32/mupdf.sln /p:Configuration=Release /p:Platform=x64 /t:libmupdf
        INSTALL_COMMAND ${CMAKE_COMMAND} -E copy_directory 
            ${mupdf_SOURCE_DIR}/platform/win32/x64/Release 
            ${CMAKE_INSTALL_PREFIX}/lib
        COMMAND ${CMAKE_COMMAND} -E copy_directory
            ${mupdf_SOURCE_DIR}/include
            ${CMAKE_INSTALL_PREFIX}/include
    )
else()
    ExternalProject_Add(mupdf_build
        SOURCE_DIR ${mupdf_SOURCE_DIR}
        BUILD_COMMAND make
        CONFIGURE_COMMAND ""
        INSTALL_COMMAND make install prefix=${CMAKE_INSTALL_PREFIX}
    )
endif()