from .cpp_class_serializer import CppClassSerializer
from .cpp_func_map_generator import CppFuncMapGenerator
from .cpp_user_builder import UserCppBuilder
from .func_map_validator import UserFuncValidator
from .header_serializer import HeaderCopier
from .mupdf_builder import MupdfBuilder
from .rust_class_serializer import RustClassSerializer
from .rust_mod_generator import RustModuleGenerator
from .object import Object, ObjectFactory
from .hierarchy_serializer import HierarchySerializer
from .override import Override, OverrideStream
from .override_serializer import OverrideSerializer
from .stream import Stream
from pathlib import Path
import logging
import shutil
import subprocess
import os 
import asyncio

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
    override_serializer: OverrideSerializer
    
    build_root: Path     
    shared_header_dir: Path
    include_dir: Path
    generated_dir: Path
    mupdf_build_dir: Path
    rs_core_root_path: Path
    rs_core_generated_module_path: Path
    objects: list[Object]
    overrides: list[Override]
    override_streams: list[OverrideStream]
    current_step: int
    
    in_debug: bool
    
    def __init__(self, build_dir: Path, factory: ObjectFactory,
                 user_cmake_lists_path: Path, **kwargs) -> None:
        self.in_debug = True
        self.overrides = []
        self.override_streams = []
        self.current_step = 0
        self.objects = factory._objs
        self._inject_unknown_obj()
        expected_classify_funcs = factory._expected_classify_funcs
        expected_extract_funcs = factory._expected_extract_funcs
        
        logger.debug(f"Initializing Builder with build_dir={build_dir}, {len(self.objects)} objects, {len(expected_classify_funcs)} classify funcs, {len(expected_extract_funcs)} extract funcs")
        self.build_root: Path = build_dir
        self.include_dir: Path = build_dir / "include"
        self.shared_header_dir: Path = self.include_dir / "shared" 
        self.generated_dir: Path = self.include_dir / "generated"
        self.mupdf_build_dir: Path = build_dir / "mupdf"
        self.rs_core_root_path = Path(__file__).parent.parent.parent.parent / "pdf_classifier_core"
        self.rs_core_generated_module_path: Path =  self.rs_core_root_path / "src" / "generated" # TODO: change to a defenitive path, currently Builder doesn't build its own version of classifier
        logger.debug(f"Resolved paths: shared_header_dir={self.shared_header_dir}, include_dir={self.include_dir}, rs_generated={self.rs_core_generated_module_path}")

        self.cpp_class_serializer = CppClassSerializer(self.shared_header_dir, self.objects)
        self.cpp_func_map_generator = CppFuncMapGenerator(expected_classify_funcs, expected_extract_funcs, self.shared_header_dir)
        self.user_cpp_builder = UserCppBuilder(self.build_root, self.shared_header_dir, 
                                               user_cmake_lists_path, self.mupdf_build_dir, self.include_dir, kwargs)
        self.func_map_validator = UserFuncValidator(expected_classify_funcs, expected_extract_funcs, user_cmake_lists_path)
        self.header_serializer = HeaderCopier(self.shared_header_dir)
        self.mupdf_builder = MupdfBuilder(self.build_root)
        self.rust_class_serializer = RustClassSerializer(self.objects, self.rs_core_generated_module_path)
        self.rust_generator = RustModuleGenerator(self.rs_core_generated_module_path)
        self.hierarchy_serializer = HierarchySerializer(self.objects, self.rs_core_generated_module_path)
        self.override_serializer = OverrideSerializer(self.overrides, self.override_streams, self.rs_core_generated_module_path)
        logger.debug("Builder initialization complete")

    def _wipe_stale_generated(self): 
        if self.generated_dir.exists(): 
            shutil.rmtree(self.generated_dir)

    def _inject_unknown_obj(self): 
        self.objects.insert(0, Object("unknown", "UNKNOWN_classify", "UNKNOWN_extract"))

    def override(self, o: Override):
        logger.info("Adding override " + o.serialize())
        if isinstance(o, OverrideStream): 
            self.override_streams.append(o)
        else: 
            self.overrides.append(o)
        
        return self 

    def _step(self, step_info: str) -> None: 
        self.current_step += 1
        logger.info(f"Step {self.current_step}: {step_info}")
        
    def _build_core(self): 
        if self.in_debug: 
            build_command = ["cargo", "build"]
        else: 
            build_command = ["cargo", "build", "--release"]

        subprocess.run(build_command, cwd=self.rs_core_root_path, check=True)
        
    def build(self, skip_user_build: bool = False) -> Stream: 
        logger.info("Starting full build (skip_user_build=%s)", skip_user_build)

        self._step("Building MuPDF")
        self.mupdf_builder.build()

        self._step("Generating Rust module")
        self.rust_generator.generate()

        self._step("Generating Rust class serializer")
        self.rust_class_serializer.generate()

        self._step("Generating C++ class serializer")        
        self.cpp_class_serializer.generate()

        self._step("Generating hierarchy serializer")
        self.hierarchy_serializer.generate()
        
        self._step("Transferring overrides to core")
        self.override_serializer.serialize()

        self._step("Copying headers")
        self.header_serializer.copy()

        self._step("Validating user function map")
        self.func_map_validator.validate()

        self._step("Generating C++ function map")
        self.cpp_func_map_generator.generate()

        if skip_user_build:
            logger.info("Skipping user C++ build (skip_user_build=True); keeping existing installed headers and lib")
        else:
            self._step("Wiping stale generated headers")
            self._wipe_stale_generated()

            self._step("Building user C++ project")
            self.user_cpp_builder.build(self.in_debug)
        
        self._step("Building classifier...")
        self._build_core()
        
        self._step("Creating stream...")
        stream = Stream()
                
        logger.info("Build complete")
        return stream

    async def spawn_classifier(self, start_page: int, end_page: int, thread_count: int, doc_path: Path, verbose: bool) -> None: 
        if self.in_debug: 
            built_classifier_path = self.rs_core_root_path.parent / "target" / "debug" / "pdf_classifier_core.exe" 
        else: 
            built_classifier_path = self.rs_core_root_path.parent / "target" / "release" / "pdf_classifier_core.exe" 
            
        
        if not built_classifier_path.exists(): 
            raise RuntimeError("Attempted to spawn classifier before it was built!")    
    
        args = [
            built_classifier_path,
            "-s", str(start_page),
            "-e", str(end_page),
            "-t", str(thread_count),
            "-p", str(doc_path),
        ]
        
        if verbose: 
            args.append('-v')

        await asyncio.create_subprocess_exec(
            *args,
            env=os.environ.copy()
        )
        