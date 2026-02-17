from typing import Optional, Literal, Any
from pathlib import Path
from globals import OBJECTS,  EXPECTED_CLASSIFY_FUNCTIONS, EXPECTED_EXTRACT_FUNCTIONS
import subprocess
from build import Builder
import os 
from weakref import ReferenceType, ref
from dataclasses import dataclass

PAIR_ORDER = Literal[1, 2] 
PAIR_TYPE = tuple[ReferenceType["Object"], PAIR_ORDER] # PAIR_ORDER is where THIS object is in the pair, not PAIR_TYPE[0] 

def deref(any: ReferenceType[Any], name: str) -> Any: 
    __any = any() 
    assert __any, f"Failed to dereference {name}"
    return __any

class Object: 
    name: str 
    children: list[ReferenceType["Object"]]
    pair: Optional[PAIR_TYPE]
    _classify_func_name: str 
    _extract_func_name: str
    is_organizational: bool
    
    def __init__(self, name: str, _classify_func_name: str, _extracted_func_name: str, is_organizational: bool = False) -> None:
        self.name = name 
        self.children = []
        self.pair = None
        self._classify_func_name = _classify_func_name
        self._extract_func_name = _extracted_func_name
        self.is_organizational = is_organizational
    
    def __serialize_to_cpp_str__(self, visited=None) -> str: 
        if visited is None: 
            visited = set()
        
        if id(self) in visited: 
            return ""
        
        visited.add(id(self))
    
        ser_children = []
        for child in self.children:
            deref = child()
            
            if deref is None: 
                raise RuntimeError("Failed to dereference child object.")
            
            ser_children.append(deref.__serialize_to_cpp_str__(visited))
                
        pair_str = "None"
        if self.pair is not None:
            pair_obj = self.pair[0]()
            if pair_obj is not None:
                pair_str = f"Some(\"{pair_obj.name}\")"
        
        children: str = ", ".join(child.strip() for child in ser_children)
        
        return f"""
Node {{
    name: KnownObject::{self.name.upper()},
    children: &[{children}],
    pair: {pair_str}
}}
""".strip()

@dataclass
class ObjectFunc: 
    file: str 
    obj: str
    name: str
            
def def_obj(name: str, classify: ObjectFunc, 
            extract: ObjectFunc, parent: Optional[ReferenceType["Object"]] = None,
            is_organizational: bool = False
            ) -> ReferenceType[Object]: 
    global OBJECTS
    global EXPECTED_CLASSIFY_FUNCTIONS
    global EXPECTED_EXTRACT_FUNCTIONS
    
    _classify_func_name = classify.name 
    _extract_func_name = extract.name
    
    assert _classify_func_name != "", "Expected a classify function name."
    assert _extract_func_name != "", "Expected an extraction function name." 

    EXPECTED_CLASSIFY_FUNCTIONS.append((classify.file, name, _classify_func_name))    
    EXPECTED_EXTRACT_FUNCTIONS.append((extract.file, name, _extract_func_name))
            
    obj = Object(name, _classify_func_name, _extract_func_name, is_organizational)
    
    if parent is not None:
        
        deref = parent() 
        
        assert deref is not None, "Failed to dereference parent object." 
        deref.children.append(ref(obj))

    OBJECTS.append(obj)
    
    
    return ref(obj) 
    
def def_pair(name_1: str, obj_1_classify: ObjectFunc,  obj_1_extract: ObjectFunc,  
             name_2: str, obj_2_classify: ObjectFunc, obj_2_extract: ObjectFunc, 
             parent: ReferenceType["Object"], is_organizational: bool = False) -> tuple[ReferenceType[Object], ReferenceType[Object]]:
    """
    Defines a pair object, and its pair object. 
    Paired objects typically are data objects (dependent), so is_organizational defaults to False.
    """
    first = def_obj(name_1, obj_1_classify, obj_1_extract, parent, is_organizational)
    second = def_obj(name_2, obj_2_classify, obj_2_extract, parent, is_organizational)
    
    __first = deref(first, "first_pair")
    __second = deref(second, "second_pair")
    
    __first.pair = (second, 1)
    __second.pair = (first, 2)
    
    return (first, second)
