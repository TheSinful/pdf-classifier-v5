from .cpp_class_serializer import CppClassSerializer
from .cpp_func_map_generator import CppFuncMapGenerator
from .cpp_user_builder import UserCppBuilder
from .func_map_validator import UserFuncValidator
from .header_serializer import HeaderCopier
from .mupdf_builder import MupdfBuilder
from .rust_class_serializer import RustClassSerializer
from .rust_mod_generator import RustModuleGenerator
from .object import Object, ObjectFactory
from .user_func import UserFunc
from .hierarchy_serializer import HierarchySerializer
from pathlib import Path
import logging

logger = logging.getLogger(__name__)

class Builder: 
    cpp_class_serializer: CppClassSerializer
    cpp_func_map_generator: CppFuncMapGenerator
    user_cpp_builder: UserCppBuilder
    func_map_validator: UserFuncValidator
    header_serializer: HeaderCopier 
    mupdf_builder: MupdfBuilder
    rust_class_serializer: RustClassSerializer
    rust_generator: RustModuleGenerator
    hierarchy_serializer: HierarchySerializer
    
    build_root: Path     
    shared_header_dir: Path
    include_dir: Path
    generated_dir: Path
    mupdf_build_dir: Path
    rs_core_generated_module_path: Path
    
    def __init__(self, build_dir: Path, factory: ObjectFactory,
                 user_cmake_lists_path: Path, **kwargs) -> None:
        objects = factory._objs
        expected_classify_funcs = factory._expected_classify_funcs
        expected_extract_funcs = factory._expected_extract_funcs
        
        logger.debug("Initializing Builder with build_dir=%s, %d objects, %d classify funcs, %d extract funcs",
                     build_dir, len(objects), len(expected_classify_funcs), len(expected_extract_funcs))
        self.build_root: Path = build_dir
        self.include_dir: Path = build_dir / "include"
        self.shared_header_dir: Path = self.include_dir / "shared" 
        self.generated_dir: Path = build_dir / "generated"
        self.mupdf_build_dir: Path = build_dir / "mupdf"
        self.rs_core_generated_module_path: Path = Path(__file__).parent.parent.parent.parent / "src" / "generated" # TODO: change to a defenitive path, currently Builder doesn't build its own version of classifier
        logger.debug("Resolved paths: shared_header_dir=%s, include_dir=%s, rs_generated=%s",
                     self.shared_header_dir, self.include_dir, self.rs_core_generated_module_path)

        self.cpp_class_serializer = CppClassSerializer(self.shared_header_dir, objects)
        self.cpp_func_map_generator = CppFuncMapGenerator(expected_classify_funcs, expected_extract_funcs, self.shared_header_dir)
        self.user_cpp_builder = UserCppBuilder(self.build_root, self.shared_header_dir, 
                                               user_cmake_lists_path, self.mupdf_build_dir, self.include_dir, kwargs)
        self.func_map_validator = UserFuncValidator(expected_classify_funcs, expected_extract_funcs, user_cmake_lists_path)
        self.header_serializer = HeaderCopier(self.shared_header_dir)
        self.mupdf_builder = MupdfBuilder(self.build_root)
        self.rust_class_serializer = RustClassSerializer(objects, self.rs_core_generated_module_path)
        self.rust_generator = RustModuleGenerator(self.rs_core_generated_module_path)
        self.hierarchy_serializer = HierarchySerializer(objects, self.rs_core_generated_module_path)
        logger.debug("Builder initialization complete")

    def build(self): 
        logger.info("Starting full build")

        logger.info("Step 1/9: Building MuPDF")
        self.mupdf_builder.build()

        logger.info("Step 2/9: Generating Rust module")
        self.rust_generator.generate()

        logger.info("Step 3/9: Generating Rust class serializer")
        self.rust_class_serializer.generate()

        logger.info("Step 4/9: Generating C++ class serializer")
        self.cpp_class_serializer.generate()

        logger.info("Step 5/9: Generating hierarchy serializer")
        self.hierarchy_serializer.generate()

        logger.info("Step 6/9: Copying headers")
        self.header_serializer.copy()

        logger.info("Step 7/9: Validating user function map")
        self.func_map_validator.validate()

        logger.info("Step 8/9: Generating C++ function map")
        self.cpp_func_map_generator.generate()

        logger.info("Step 9/9: Building user C++ project")
        self.user_cpp_builder.build()

        logger.info("Build complete")
        