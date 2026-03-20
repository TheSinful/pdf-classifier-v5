from pathlib import Path
import textwrap
import logging

logger = logging.getLogger(__name__)

class RustModuleGenerator: 
    core_generated_module_path: Path
    
    def __init__(self, core_generated_module_path: Path): 
        self.core_generated_module_path = core_generated_module_path
    
    def generate(self): 
        logger.info("Generating Rust module file -> %s", self.core_generated_module_path / "mod.rs")
        self._gen_module_rs()
    
    def _gen_module_rs(self): 
        self.core_generated_module_path.mkdir(exist_ok=True, parents=True)
        mod_rs_path = self.core_generated_module_path / "mod.rs"
        logger.debug("Writing mod.rs to %s", mod_rs_path)
        
        data = textwrap.dedent("""
            pub mod reflected_objects;
            pub mod generated_object_types;    
        """)
    
        with open(mod_rs_path, "w") as f: 
            f.write(data)
        logger.debug("mod.rs written")

