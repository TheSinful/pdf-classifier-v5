from typing import Optional, Literal
from weakref import ReferenceType, ref
from .user_func import UserFunc
import logging
import textwrap

logger = logging.getLogger(__name__)

PAIR_ORDER = Literal[1, 2] 
PAIR_TYPE = tuple[ReferenceType["Object"], PAIR_ORDER] # PAIR_ORDER is where THIS object is in the pair, not PAIR_TYPE[0] 

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
        
        return textwrap.dedent(f"""
            Node {{
                name: KnownObject::{self.name.upper()},
                children: &[{children}],
                pair: {pair_str}
            }}
        """)
            
class ObjectFactory: 
    _objs: list[Object]
    _expected_classify_funcs: list[UserFunc]
    _expected_extract_funcs: list[UserFunc]
    _header: str = ""
    
    def __init__(self, header: str = "") -> None:
        if not header == "": 
            self._header = header  
        
        self._objs = []
        self._expected_classify_funcs = []
        self._expected_extract_funcs = []        
    
    def new(self) -> "ObjectBuilder": 
        o = ObjectBuilder(self)
        if not self._header == "": 
            o.header(self._header)
            
        return o
    
    def _finalize(self, obj: Object, classify: UserFunc, extract: UserFunc): 
        self._objs.append(obj)
        self._expected_classify_funcs.append(classify)
        self._expected_extract_funcs.append(extract)

    def _find_obj(self, name: str) -> Optional[ReferenceType[Object]]: 
        return next((ref(obj) for obj in self._objs if obj.name == name), None)
        

class ObjectBuilder: 
    _name: str 
    _parent: Optional[ReferenceType[Object]] = None
    _pair: Optional[PAIR_TYPE] = None
    _classify_func: UserFunc 
    _extract_func: UserFunc
    _organizational: bool
    _factory: ObjectFactory 
    _header: str
    
    def __init__(self, factory: ObjectFactory): 
        self._factory = factory
    
    def name(self, name: str) -> "ObjectBuilder": 
        self._name = name
        return self
    
    def child_of(self, parent_name: str) -> "ObjectBuilder": 
        match = self._factory._find_obj(parent_name)
        if match is None: 
            raise RuntimeError("No object exists with name " + parent_name)
        
        self._parent = match
        return self
    
    def pair_to(self, pair_name: str, order: PAIR_ORDER) -> "ObjectBuilder": 
        if order == 1: 
            return self # todo: need some validation to ensure that the pair is defined later
        
        match = self._factory._find_obj(pair_name)
        if match is None: 
            raise RuntimeError("No object exists with name " + pair_name)
        
        self._pair = (match, 2)
        return self
    
    def header(self, name: str) -> "ObjectBuilder": 
        self._header = name 
        return self    
    
    def classify(self, name: str, file_name: str = "") -> "ObjectBuilder": 
        if not hasattr(self, "_name"): 
            raise RuntimeError("Attempted to define an object without a name!")        
        
        if not hasattr(self, "_header") and file_name == "": 
            raise RuntimeError("Attempted to define a classify/extract func without a header path!")        

        if not file_name == "":
            self._classify_func = UserFunc(self._header, self._name, name)
        else: 
            self._classify_func = UserFunc(self._header, self._name, name)
        
        return self
        
    def extract(self, name: str, file_name: str = "") -> "ObjectBuilder": 
        if not hasattr(self, "_name"): 
            raise RuntimeError("Attempted to define an object without a name!")        
        
        if not hasattr(self, "_header") and file_name == "": 
            raise RuntimeError("Attempted to define a classify/extract func without a header path!")        

        if not file_name == "":
            self._extract_func = UserFunc(self._header, self._name, name)
        else:
            self._extract_func = UserFunc(self._header, self._name, name)
        
        return self
    
    def organizational(self) -> "ObjectBuilder": 
        self._organizational = True
        return self
    
    def build(self): 
        if not hasattr(self, "_name"): 
            raise RuntimeError("Attempted to define an object without a name!")
        
        if not hasattr(self, "_classify_func"):
            raise RuntimeError("Attempted to define an object without a classify function!")
        
        if not hasattr(self, "_extract_func"):
            raise RuntimeError("Attempted to define an object without an extraction function!")
        
        if not hasattr(self, "_organizational"): 
            self._organizational = False
        
        obj: Object = Object(self._name, self._classify_func.name, self._extract_func.name, self._organizational)
        if self._pair is not None: 
            match = self._factory._find_obj(self._pair[0]().name)  # type: ignore (safe)
            match()._pair = (ref(obj), 1) # type: ignore
            
        return self._factory._finalize(
            obj,
            self._classify_func, 
            self._extract_func
        )