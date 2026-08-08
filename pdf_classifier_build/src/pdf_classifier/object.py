from typing import Optional, Literal
from weakref import ReferenceType, ref
from .user_func import UserFunc
from typing_extensions import deprecated
import logging
import textwrap

logger = logging.getLogger(__name__)

PAIR_ORDER = Literal[1, 2]
PAIR_TYPE = tuple[ReferenceType["Object"], PAIR_ORDER] # PAIR_ORDER is where THIS object is in the pair, not PAIR_TYPE[0]

# `DEFINE_OBJECT(datatable, DataTable)` expands to `classify_datatable` and
# `extract_datatable` - the shim names are the object name with these prefixes,
# which is what lets cpp_class() derive them instead of being told.
CLASSIFY_SHIM_PREFIX = "classify_"
EXTRACT_SHIM_PREFIX = "extract_"

class Object: 
    name: str 
    children: list[ReferenceType["Object"]]
    pair: Optional[PAIR_TYPE]
    _classify_func_name: str 
    _extract_func_name: str
    is_organizational: bool
    
    def __init__(self, name: str, _classify_func_name: str, _extracted_func_name: str, is_organizational: bool = False, pair: Optional[PAIR_TYPE] = None) -> None:
        self.name = name 
        self.children = []
        self.pair = pair
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
    _parent: Optional[ReferenceType[Object]]
    _pair: Optional[PAIR_TYPE]
    _classify_func: UserFunc 
    _extract_func: UserFunc
    _organizational: bool
    _factory: ObjectFactory
    _header: str
    _cpp_class: str

    def __init__(self, factory: ObjectFactory):
        self._factory = factory
        self._parent = None
        self._pair = None
        self._cpp_class = ""

    def name(self, name: str) -> "ObjectBuilder": 
        """
            States the name of self.
            Note that this does not refer to the C++ class, which is instead defined within [cpp_class()]
            which will also subsequently define [name] assuming this method is not called.
        """
        
        self._name = name
        return self
    
    def child_of(self, parent_name: str) -> "ObjectBuilder": 
        """
            States that self is a child of [parent_name]. 
            
            If the document is envisioned as a hierarchy, this method states that [parent_name] is a node above self.
        """
        
        match: Optional[ReferenceType[Object]] = self._factory._find_obj(parent_name)
        if match is None: 
            raise RuntimeError("No object exists with name " + parent_name)
        
        self._parent = match
        
        return self
    
    def pair_to(self, pair_name: str, order: PAIR_ORDER) -> "ObjectBuilder": 
        """
            States that self is a pair to another Object, [pair_name]. 
            Further expects the order of the pair relative to the current Object, 
            if order=1, then self is the first Object in the pair. 
            For instance, if we have objects Datatable and Diagram where:
                Diagram.order=1
                Datatable.order=2
            Diagrams are **always** expected to show up before Datatables. 
            
            Note that pairs are explicit, there should be no ambiguity of pairs
            with the sole exception potentially being defined within future OverrideStreams.

            **Both** objects must declare the pair. The order=2 call is the one that
            records it - it looks its partner up and writes the link onto both sides.
            The order=1 call is currently a no-op that exists to document intent: it
            neither records anything nor checks that [pair_name] is ever defined, so a
            pair declared from only the order=1 side silently leaves both objects unpaired.
        """


        if order == 1: 
            return self # todo: need some validation to ensure that the pair is defined later
        match = self._factory._find_obj(pair_name)
        if match is None: 
            raise RuntimeError("No object exists with name " + pair_name)
        
        self._pair = (match, 2)
        return self
    
    def header(self, name: str) -> "ObjectBuilder":
        """
            The C++ header declaring this object's class, as an include path relative
            to the BASE_DIRS of the project's `FILE_SET HEADERS` - it is emitted verbatim
            as `#include <generated/[name]>` in the generated function map.

            Note this is the *declaration*: `DEFINE_OBJECT` and the classify/extract
            bodies belong in the matching .cpp, since the macro expands to non-inline
            free functions.
        """
        self._header = name
        return self

    def cpp_class(self, name: str) -> "ObjectBuilder":
        """
            The name of the C++ class deriving from pdf_classifier_lib's 'Object',
            which implements this object's classify()/extract() overrides.

            The class is registered on the C++ side with `DEFINE_OBJECT(<object name>, [name])`,
            which generates the two standalone shims wrapping those overrides. Their
            names follow from the object name, so this method makes calling
            [classify()] or [extract()] unnecessary - they are derived at [build()].

            Note that you may omit calling [name()] since this method will define it
            as its lowercase equivalent if and only if it is undefined.
        """
        self._cpp_class = name
        if not hasattr(self, "_name"):
            self._name = name.lower()

        return self

    @deprecated("""
                    Define an object using 'cpp_class' instead which automatically
                    locates its classification function.
                """)        
    def classify(self, name: str, file_name: str = "") -> "ObjectBuilder": 
        """
            Declares the standalone classification shim by its C++ function name [name]
            (e.g. "classify_chapter" - not the object's name, which comes from [name()]),
            found in the header [file_name] or in the one already set via [header()].
        """
        
        if not hasattr(self, "_name"): 
            raise RuntimeError("Attempted to define an object without a name!")        
        
        if not hasattr(self, "_header") and file_name == "": 
            raise RuntimeError("Attempted to define a classify/extract func without a header path!")        

        if file_name == "":
            self._classify_func = UserFunc(self._header, self._name, name)
        else: 
            self._classify_func = UserFunc(file_name, self._name, name)
        
        return self

    @deprecated("""
                    Define an object using 'cpp_class' instead which automatically
                    locates its extraction function.
                """)        
    def extract(self, name: str, file_name: str = "") -> "ObjectBuilder": 
        """
            Declares the standalone extraction shim by its C++ function name [name]
            (e.g. "extract_chapter" - not the object's name, which comes from [name()]),
            found in the header [file_name] or in the one already set via [header()].
        """
        
        if not hasattr(self, "_name"): 
            raise RuntimeError("Attempted to define an object without a name!")        
        
        if not hasattr(self, "_header") and file_name == "": 
            raise RuntimeError("Attempted to define a classify/extract func without a header path!")        

        if file_name == "":
            self._extract_func = UserFunc(self._header, self._name, name)
        else:
            self._extract_func = UserFunc(file_name, self._name, name)
        
        return self
    
    def organizational(self) -> "ObjectBuilder":
        """
            Defines self as 'organizational' 
            An organizational Object is defined as an Object which 
            is an anchor within the document. For instance, a Chapter Object (from the examples)
            is an organizational Object since it organizes everything underneath it (subchapters, diagrams and datatables).
            
            Organizational Objects are the objects that the classifier searches for when in a deferral state.
            When a deferral is triggered, the classifier searches for the next organizational -regardless of hierarchy.

            Once found it fills in the pages in between with that organizational's *dependents*,
            which are not necessarily its children: the search descends through organizational
            children until it reaches one that actually has non-organizational children, and
            uses those. Anchoring on a chapter therefore fills with diagrams and datatables,
            since subchapter is itself organizational.
        """
        self._organizational = True
        return self
    
    def _derive_funcs_from_cpp_class(self) -> None:
        """Fill in the shims `DEFINE_OBJECT` generates for [cpp_class].

        Deferred to build() rather than done in cpp_class(): [name()] and
        [header()] may be called after it, and the derived names depend on both.
        An explicit [classify()]/[extract()] still wins.
        """
        if not self._cpp_class:
            return

        if not hasattr(self, "_header"):
            raise RuntimeError(
                f"Object '{self._name}' uses cpp_class('{self._cpp_class}') but has no header - "
                "call header() or pass one to ObjectFactory()."
            )

        if not hasattr(self, "_classify_func"):
            self._classify_func = UserFunc(self._header, self._name,
                                           CLASSIFY_SHIM_PREFIX + self._name, self._cpp_class)
        if not hasattr(self, "_extract_func"):
            self._extract_func = UserFunc(self._header, self._name,
                                          EXTRACT_SHIM_PREFIX + self._name, self._cpp_class)

    def build(self):
        if not hasattr(self, "_name"):
            raise RuntimeError("Attempted to define an object without a name!")

        self._derive_funcs_from_cpp_class()

        if not hasattr(self, "_classify_func"):
            raise RuntimeError("Attempted to define an object without a classify function!")

        if not hasattr(self, "_extract_func"):
            raise RuntimeError("Attempted to define an object without an extraction function!")

        if not hasattr(self, "_organizational"):
            self._organizational = False

        # cpp_class() may be called after classify()/extract(), so bind it here
        # rather than at the point the UserFuncs are built.
        self._classify_func.cpp_class = self._cpp_class
        self._extract_func.cpp_class = self._cpp_class

        obj: Object = Object(self._name, self._classify_func.name, self._extract_func.name, self._organizational, self._pair)
        
        if self._pair is not None: 
            second_in_pair_obj = self._pair[0]()
            assert second_in_pair_obj is not None, "inner pair should've been looked up"
            
            match = self._factory._find_obj(second_in_pair_obj.name)
            assert match is not None, f"obj {second_in_pair_obj.name} should be defined prior to {obj.name}"
            
            match_deref = match()
            assert match_deref is not None, f"obj {second_in_pair_obj.name} should be defined prior to {obj.name}"
                        
            match_deref.pair = (ref(obj), 1)
            
        if self._parent is not None: 
            parent_deref = self._parent()
            assert parent_deref is not None, "inner pair should've been looked up" 
            
            match = self._factory._find_obj(parent_deref.name) 
            assert match is not None, f"obj {parent_deref.name} should be defined prior to {obj.name}"
            
            match_deref = match()
            assert match_deref is not None, f"obj {parent_deref.name} should be defined prior to {obj.name}"
            match_deref.children.append(ref(obj))
        
        return self._factory._finalize(
            obj,
            self._classify_func, 
            self._extract_func
        )