import sys 
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))

from build import Builder, SHARED_HEADER_PATH
from globals import OBJECTS
from object import Object

def test_generate_shared_object_enum(): 
    global OBJECTS
    OBJECTS.append(Object("Test", "", ""))
    OBJECTS.append(Object("Test2", "", ""))
    OBJECTS.append(Object("Test3", "", ""))
    
    build = Builder(Path("CMakeLists.txt"), Path(__file__).parent.parent.parent / "include")
    build._serialize_object_names_into_enum_header()

    # must be manually checked but if no exceptions should be fine
    
if __name__ == "__main__": 
    test_generate_shared_object_enum() 
    