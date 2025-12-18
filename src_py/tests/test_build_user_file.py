import sys 
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

from build import Builder
import globals

def test_successful_build():
    cmake_path = Path(__file__).parent  / "cmake" / "build_tests" 
    build_dir = cmake_path / "build"
    builder = Builder(cmake_path / "CMakeLists.txt", build_dir)
    builder._build_user_project()
    