

from typing import TYPE_CHECKING
from pathlib import Path
if TYPE_CHECKING: 
    from object import Object

OBJECTS: list["Object"] = []
EXPECTED_CLASSIFY_FUNCTIONS: list[tuple[str, str, str]] = [] # (file obj_name func_name)
EXPECTED_EXTRACT_FUNCTIONS: list[tuple[str, str, str]] = []  # (file obj_name func_name)
