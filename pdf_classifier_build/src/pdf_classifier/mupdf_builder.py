from pathlib import Path
from cmake import CMAKE_BIN_DIR
import os
import subprocess
import textwrap
import logging

logger = logging.getLogger(__name__)

class MupdfBuilder: 
    build_root_dir: Path
    mupdf_build_dir: Path
    cmake_install_mupdf_template_path: Path
    mupdf_config_cmake_path: Path
    
    def __init__(self, build_root: Path): 
        self.build_root_dir = build_root
        self.mupdf_build_dir = build_root / "mupdf_build"
        self.mupdf_config_cmake_path = build_root / "lib" / "cmake" / "mupdf" / "mupdfConfig.cmake"
        self.cmake_install_mupdf_template_path = Path(__file__).parent / "cmake" / "install_mupdf.cmake"
        self.mupdf_build_dir.mkdir(exist_ok=True, parents=True)
        self.mupdf_config_cmake_path.parent.mkdir(exist_ok=True, parents=True) 
        
    def build(self) -> None:
        if self._is_built(): 
            logger.info("MuPDF already built at %s, skipping", self.mupdf_build_dir)
            return
        logger.info("Building MuPDF into %s", self.mupdf_build_dir)
        self._write_temp_cmake()
        self._configure()
        self._build()
        self._install()
        self._create_mupdf_config()
    
    def _is_built(self) -> bool: 
        if (self.mupdf_build_dir / "CMakeCache.txt").exists():
            return True

        return False
    
    def _write_temp_cmake(self): 
        temp_cmake = self.mupdf_build_dir / "CMakeLists.txt"
        logger.debug("Writing temporary CMakeLists.txt to %s", temp_cmake)
        
        cmake_content = textwrap.dedent(f"""
            cmake_minimum_required(VERSION 3.12.0)
            project(mupdf_build)

            set(CMAKE_INSTALL_LIBDIR "lib")
            set(CMAKE_INSTALL_INCLUDEDIR "include/mupdf")

            include({str(self.cmake_install_mupdf_template_path).replace("\\", "/")})
        """)
        
        with open(temp_cmake, "w") as f:
            f.write(cmake_content)
            
    def _configure(self): 
        config_cmd = [
            os.path.join(CMAKE_BIN_DIR, "cmake"),
            f"-DCMAKE_INSTALL_PREFIX={str(self.build_root_dir)}",
            str(self.mupdf_build_dir)
        ]
        logger.debug("MuPDF configure: %s", config_cmd)
        subprocess.run(config_cmd, cwd=str(self.mupdf_build_dir), check=True)
        logger.debug("MuPDF configuration complete")

    def _build(self): 
        build_cmd = [os.path.join(CMAKE_BIN_DIR, "cmake"), "--build", str(self.mupdf_build_dir), "--config Release"]
        logger.debug("MuPDF build: %s", build_cmd)
        subprocess.run(build_cmd, check=True)
        logger.debug("MuPDF build complete")

    def _install(self): 
        install_cmd = [os.path.join(CMAKE_BIN_DIR, "cmake"), "--install", str(self.mupdf_build_dir)]
        logger.debug("MuPDF install: %s", install_cmd)
        subprocess.run(install_cmd, check=True)
        logger.debug("MuPDF install complete")

    def _create_mupdf_config(self) -> None:
        """Create mupdfConfig.cmake for find_package() support"""
        logger.debug("Creating mupdfConfig.cmake at %s", self.mupdf_config_cmake_path)
        config_content = textwrap.dedent("""
            include(FindPackageHandleStandardArgs)

            set(MUPDF_LIBRARIES "${CMAKE_INSTALL_PREFIX}/lib/libmupdf.lib")
            set(MUPDF_INCLUDE_DIR "${CMAKE_INSTALL_PREFIX}/include/mupdf")


            if(NOT TARGET mupdf::mupdf)
            add_library(mupdf::mupdf STATIC IMPORTED)
            set_target_properties(mupdf::mupdf PROPERTIES
                IMPORTED_LOCATION "${CMAKE_INSTALL_PREFIX}/lib/libmupdf.lib"
                INTERFACE_INCLUDE_DIRECTORIES "${CMAKE_INSTALL_PREFIX}/include/mupdf"
            )
            endif()

            find_package_handle_standard_args(mupdf DEFAULT_MSG
            MUPDF_LIBRARIES MUPDF_INCLUDE_DIR
            )

            set(mupdf_FOUND TRUE)
        """)
        
        with open(self.mupdf_config_cmake_path, "w") as f:
            f.write(config_content)
        
        logger.info("Wrote mupdfConfig.cmake to %s", self.mupdf_config_cmake_path)