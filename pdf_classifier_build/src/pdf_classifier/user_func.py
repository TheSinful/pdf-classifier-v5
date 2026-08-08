from dataclasses import dataclass

@dataclass
class FuncSyntax:
    return_type: str
    param_types: list[str]
    method_name: str

    def __init__(self, return_type: str, param_types: list[str], method_name: str = ""):
        """`method_name` selects how a declaration is matched.

        When set (e.g. "classify"), the declaration must be a member function of
        that exact name on the object's C++ class - `Chapter::classify(...)`.
        When empty, the legacy free-function rule applies: the declaration must
        be at namespace scope and named after `UserFunc.name`.
        """
        self.return_type = return_type
        self.param_types = param_types
        self.method_name = method_name

@dataclass
class UserFunc:
    file_name: str
    for_class: str
    name: str
    cpp_class: str

    def __init__(self, file_name: str, for_class: str, name: str, cpp_class: str = "") -> None:
        """`cpp_class` is the C++ class implementing the object, e.g. "DataTable"
        for the object named "datatable". Leave it empty to match any class whose
        name equals `for_class` ignoring case and underscores.
        """
        self.file_name = file_name
        self.for_class = for_class
        self.name = name
        self.cpp_class = cpp_class
