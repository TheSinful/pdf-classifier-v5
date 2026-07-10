from .serializer import Serializer
from .user_func import UserFunc
from pathlib import Path
import textwrap
import logging

logger = logging.getLogger(__name__)

class CppFuncMapGenerator(Serializer): 
    expected_classify_funcs: list[UserFunc]
    expected_extract_funcs: list[UserFunc]
    all_expected_funcs: list[UserFunc]
    data: str
    out_path: Path
    
    def __init__(self, expected_classify_funcs: list[UserFunc],
                expected_extract_funcs: list[UserFunc], shared_header_path: Path, 
                file_name: str = "func_map.h") -> None:
        self.data = ""
        self.expected_classify_funcs = expected_classify_funcs
        self.expected_extract_funcs = expected_extract_funcs
        self.out_path = shared_header_path / file_name
        self.all_expected_funcs = self.expected_classify_funcs + self.expected_extract_funcs
    
    def generate(self) -> None:
        logger.info("Generating C++ function map -> %s", self.out_path)
        logger.debug("%d classify funcs, %d extract funcs",
                     len(self.expected_classify_funcs), len(self.expected_extract_funcs))
        self._include_files()
        self._func_struct()
        self._classify_func_map()
        self._extract_func_map()
        self._dump_data(self.out_path, self.data)
        logger.debug("C++ function map generation done")
    
    def _include_files(self): 
        include_files = [
            f'#include <generated/{func}>'
            for func in set(func.file_name for func in self.all_expected_funcs)
        ]
        
        self.data += textwrap.dedent(f"""
            #pragma once

            #include <string>
            #include <vector> 
            {self._fmt_payload(include_files)}
        """)
        
    def _func_struct(self): 
        self.data += """
            struct Func
            {
                std::string obj_name;
                void* ptr; 
            };
        """
    
    def _classify_func_map(self): 
        classify_func_entries = [
            f'{{ "{func.for_class}",  &{func.name}}},'
            for func in self.expected_classify_funcs
        ]
        
        self.data += textwrap.dedent(f"""
            static const std::vector<Func> ClassifyFuncMap = {{
                {self._fmt_payload(classify_func_entries)}
            }};
            
            static const std::string classify_func_names = "{self._fmt_payload([func.for_class for func in self.expected_classify_funcs], " ,")}";
        """)
    
    def _extract_func_map(self): 
        extract_func_entries = [
            f'{{ "{func.for_class}",  &{func.name}}},'
            for func in self.expected_extract_funcs
        ]
        
        self.data += textwrap.dedent(f"""
            static const std::vector<Func> ExtractFuncMap = {{
                {self._fmt_payload(extract_func_entries)}
            }};
            
            static const std::string extract_func_names = "{self._fmt_payload([func.for_class for func in self.expected_extract_funcs], " ,")}";            
        """)
