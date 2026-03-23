from .serializer import Serializer
from .override import Override
from pathlib import Path 
import textwrap

class OverrideSerializer(Serializer): 
    overrides: list[Override]
    generated_mod_path: Path
    
    def __init__(self, overrides: list[Override], generated_mod_path: Path) -> None:
        self.overrides = overrides
        self.generated_mod_path = generated_mod_path
            
    def serialize(self): 
        path = self.generated_mod_path / "overrides.rs"
        cases = [f"\"{override.serialize()}\"," for override in self.overrides]
        data = textwrap.dedent(f"""
            pub const OVERRIDES: [&'static str; {len(self.overrides)}] = [
                {self._fmt_payload(cases)}
            ];
        """)

        self._dump_data(path, data)
