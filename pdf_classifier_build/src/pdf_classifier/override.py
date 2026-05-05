from abc import abstractmethod
from .serializer import Serializer
    
class Override: 
        
    @abstractmethod
    def serialize(self) -> str:
        pass
    
class OverrideStream(Override): 
    ...

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
    
class MultiPageHierarchyBreakOverride(OverrideStream, Serializer): 
    """
        Indicates that after class "after_class" is successfully classified
        that until "end_of_class" is found everything in-between will follow the 
        order of "fill_with" 
        
        For example, if after_class is "chapter", end_on_class is "subchapter"
        and fill_with is ["diagram", "datatable"]
        
        When page(0) == "chapter" (by classification),
        and page (10) == "subchapter" 
        pages 1..9 will alternate between "diagram" and "datatable"
        till eventually page 10 fails classification on either,
        where it is then subsequently classified as "subchapter"
        
        This signifies an expected hierarchy break, where otherwise 
        we would enter deferral mode, and waste cycles on classifying through
        hierarchial means. When hierarchy should be disregarded. 
        
        Takes BlankAfterClass into account.
    """
    
    after_class: str
    impl_blank_after: bool 
    end_on_class: str
    fill_with: list[str] 
    
    def __init__(self, after_class: str, impl_blank_after: bool, end_on_class: str, fill_with: list[str]) -> None:
        self.after_class = after_class
        self.impl_blank_after = impl_blank_after
        self.end_on_class = end_on_class
        self.fill_with = fill_with
        super().__init__()
        
    def serialize(self) -> str:
        fill_with_to_vec = [ f"KnownObject::{obj.upper()}" for obj in self.fill_with ]
        
        return f"""MultiPageHierarchyBreak::new( 
                KnownObject::{self.after_class.upper()}, 
                {str(self.impl_blank_after).lower()}, 
                KnownObject::{self.end_on_class.upper()}, 
                &[{self._fmt_payload(fill_with_to_vec, ",")}]
            )"""