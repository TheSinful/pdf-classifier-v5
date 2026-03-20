from dataclasses import dataclass

@dataclass
class FuncSyntax: 
    return_type: str 
    param_types: list[str]
    
    def __init__(self, return_type: str, param_types: list[str]): 
        self.return_type = return_type 
        self.param_types = param_types

@dataclass 
class UserFunc: 
    file_name: str 
    for_class: str
    name: str 
    
    def __init__(self, file_name: str, for_class: str, name: str) -> None:
        self.file_name = file_name
        self.for_class = for_class 
        self.name = name
        