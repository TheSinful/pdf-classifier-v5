import textwrap
import logging
from pathlib import Path 
from .serializer import Serializer
from .object import Object

logger = logging.getLogger(__name__)

class CppClassSerializer(Serializer): 
    out_file_path: Path    
    data: str
    objects: list[Object]
    enum_name: str
    
    def __init__(self, shared_header_path: Path, objects: list[Object], 
                 file_name: str = "generated_page_types.h", 
                 enum_name: str = "KnownObject") -> None:
        self.objects = objects
        self.out_file_path = shared_header_path / file_name
        self.enum_name = enum_name
        self.data = ""
        self.out_file_path.parent.mkdir(exist_ok=True, parents=True)
    
    def generate(self):
        logger.info("Generating C++ page types header -> %s", self.out_file_path)
        logger.debug("Serializing %d objects: %s", len(self.objects), [o.name for o in self.objects])
        self._guards_and_includes()
        self._class_enum()
        self._to_string_method()
        self._from_string_method()
        self._dump_data(self.out_file_path, self.data)
        logger.debug("C++ class serializer done")
    
    def _guards_and_includes(self): 
        self.data += textwrap.dedent(f"""
            #pragma once 
            #include <string> 
            #include <stdexcept>      
        """)
        
    def _class_enum(self) -> None: 
        payload = [object.name for object in self.objects]
        
        self.data += textwrap.dedent(f"""
            enum {self.enum_name} {{
                {self._fmt_payload(payload, ", ")}
            }}
        """)
        
    def _to_string_method(self) -> None: 
        to_str_cases = [
            f'case {self.enum_name}::{object.name}: return "{object.name}";'
            for object in self.objects
        ]

        self.data += textwrap.dedent(f"""
            std::string page_type_to_string({self.enum_name} obj) {{
                switch (obj) {{
                    {to_str_cases}
                    default: return "unknown"; 
                }}
            }}
        """)
        
    def _from_string_method(self) -> None: 
        from_str_cases = [
            f'if(obj == "{object.name}") {{ return KnownObject::{object.name};}}'
            for object in self.objects
        ]
        
        self.data += textwrap.dedent(f"""
            {self.enum_name} page_type_from_string(std::string obj) {{
                {from_str_cases}
                throw std::runtime_error("Attempted to convert string to object that doesn't exist!"); 
            }}
        """)