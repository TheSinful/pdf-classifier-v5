from abc import abstractmethod
    
class Override: 
        
    @abstractmethod
    def serialize(self) -> str:
        pass


class BlankAfterClassOverride(Override):
    """
        Indicates that after page "n" is classifed as class "x",
        page n+1 WILL be blank (and therefore classified as "unknown") 
    """
    for_class: str
    
    def __init__(self, for_class: str) -> None:
        self.for_class = for_class
        super().__init__()
    
    def serialize(self) -> str:
        return f"BlankAfter{{ config: KnownObject::{self.for_class.upper()} }}"