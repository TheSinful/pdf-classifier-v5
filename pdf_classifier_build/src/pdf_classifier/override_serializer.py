from .serializer import Serializer
from .override import Override, OverrideStream
from pathlib import Path 
import textwrap

class OverrideSerializer(Serializer): 
    overrides: list[Override]
    override_streams: list[OverrideStream]
    generated_mod_path: Path
    enum_class_name: str
    
    def __init__(self, overrides: list[Override], override_streams: list[OverrideStream], generated_mod_path: Path, enum_class_name: str = "KnownObject") -> None:
        self.overrides = overrides
        self.override_streams = override_streams
        self.generated_mod_path = generated_mod_path
        self.enum_class_name = enum_class_name
    
    def serialize(self): 
        path = self.generated_mod_path / "overrides.rs"
        base_cases = [f"&{override.serialize()}," for override in self.overrides]
        stream_cases = [f"Mutex::new(Box::new({stream.serialize()}))," for stream in self.override_streams]
        
        data = textwrap.dedent(f"""
            use std::sync::{{LazyLock, Mutex}};
            use crate::constraints::overrides::*;
            use crate::generated::generated_object_types::{self.enum_class_name};
            
                   
            pub const OVERRIDES: [&'static dyn Override; {len(self.overrides)}] = [
                {self._fmt_payload(base_cases)}
            ];
            
            pub static OVERRIDE_STREAMS: LazyLock<[Mutex<Box<dyn OverrideStream>>; {len(self.override_streams)}]> = LazyLock::new(|| {{[
                {self._fmt_payload(stream_cases)}
            ]}});
        """)

        self._dump_data(path, data)
