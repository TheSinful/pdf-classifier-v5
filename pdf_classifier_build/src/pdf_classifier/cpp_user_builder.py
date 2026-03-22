from typing import Any
from cmake import CMAKE_BIN_DIR
from pathlib import Path
import subprocess
import os
import logging

logger = logging.getLogger(__name__)

class UserCppBuilder: 
    build_dir: Path
    shared_header_path: Path
    user_cmake_lists_path: Path
    mupdf_build_dir: Path
    include_dir: Path
    user_build_args: Any
    
    def __init__(self, build_dir: Path, shared_header_path: Path, 
                 user_cmake_lists_path: Path, mupdf_build_dir: Path, 
                 include_dir: Path, user_build_args: Any) -> None:
        self.build_dir = build_dir
        self.shared_header_path = shared_header_path
        self.user_cmake_lists_path = user_cmake_lists_path
        self.mupdf_build_dir = mupdf_build_dir
        self.include_dir = include_dir
        self.user_build_args = user_build_args
    
    def build(self):         
        logger.info("Building user C++ project from %s", self.user_cmake_lists_path)
        self.user_build_args["BUILD_SHARED_LIBS"] = "OFF"
        self.user_build_args["CLASSIFIER_INCLUDE_DIR"] = str(self.shared_header_path)
        self.user_build_args["mupdf_DIR"] = self.build_dir / "lib" / "cmake" / "mupdf"
        self.user_build_args["CMAKE_INSTALL_PREFIX"] = str(self.build_dir)
        self.build_dir.mkdir(exist_ok=True, parents=True)
        
        # configuration
        config_cmd = [os.path.join(CMAKE_BIN_DIR, "cmake")]
        for key, value in self.user_build_args.items():
            config_cmd.append(f"-D{key}={value}")
        config_cmd.append(str(self.user_cmake_lists_path.parent))
        
        logger.debug("CMake configure command: %s", config_cmd)
        subprocess.run(config_cmd, cwd=str(self.build_dir), check=True)
        logger.debug("CMake configuration complete")
        
        # build 
        build_cmd = [os.path.join(CMAKE_BIN_DIR, "cmake"), "--build", str(self.build_dir), "--config Release"]
        logger.debug("CMake build command: %s", build_cmd)
        subprocess.run(build_cmd, check=True)
        logger.debug("CMake build complete")
    
        # install headers
        install_cmd = [os.path.join(CMAKE_BIN_DIR, "cmake"), "--install", str(self.build_dir), "--prefix", str(self.include_dir.parent)]
        logger.debug("CMake install command: %s", install_cmd)
        subprocess.run(install_cmd, check=True)
        logger.info("User C++ project built and installed to %s", self.include_dir.parent)
