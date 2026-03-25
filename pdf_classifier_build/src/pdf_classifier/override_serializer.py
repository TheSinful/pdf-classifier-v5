from .serializer import Serializer
from .override import Override
from pathlib import Path 
import textwrap

class OverrideSerializer(Serializer): 
    overrides: list[Override]
    generated_mod_path: Path
    enum_class_name: str
    
    def __init__(self, overrides: list[Override], generated_mod_path: Path, enum_class_name: str = "KnownObject") -> None:
        self.overrides = overrides
        self.generated_mod_path = generated_mod_path
        self.enum_class_name = enum_class_name
            
    def serialize(self): 
        path = self.generated_mod_path / "overrides.rs"
        cases = [f"&{override.serialize()}," for override in self.overrides]
        
        data = textwrap.dedent(f"""
            use crate::constraints::overrides::*;
            use crate::generated::generated_object_types::{self.enum_class_name};
                   
            pub const OVERRIDES: [&'static dyn Override; {len(self.overrides)}] = [
                {self._fmt_payload(cases)}
            ];
        """)

        self._dump_data(path, data)
