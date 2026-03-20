import textwrap
import logging
from pathlib import Path
from .object import Object
from .serializer import Serializer

logger = logging.getLogger(__name__)

class RustClassSerializer(Serializer): 
    core_generated_module_path: Path    
    data: str
    objects: list[Object]
    enum_name: str
    output_file_name: str
    
    def __init__(self, objects: list[Object], core_generated_module_path: Path, 
                 enum_name: str = "{self.enum_name}", output_file_name: str = "generated_object_types.rs"): 
        self.core_generated_module_path = core_generated_module_path
        self.objects = objects 
        self.enum_name = enum_name
        self.output_file_name = output_file_name
        self.data = ""
    
    def generate(self): 
        logger.info("Generating Rust object types -> %s", self.core_generated_module_path / self.output_file_name)
        logger.debug("Serializing %d objects: %s", len(self.objects), [o.name for o in self.objects])
        self._object_count_const()
        self._class_enum()
        self._begin_impl_block()
        self._has_children_method()
        self._has_pair_method()
        self._is_first_in_pair_method()
        self._is_second_in_pair_method()
        self._end_impl_block()
        self._to_str_impl()
        self._obj_cast_err_enum()
        self._from_str_impl()
        self._from_u8_impl()
        self._default_impl()
        self._into_u8_impl()
        self._dump_data(self.core_generated_module_path / self.output_file_name, self.data)
        logger.debug("Rust class serializer done")
        
    def _object_count_const(self) -> None: 
        self.data += f"pub const OBJECT_COUNT: u8 = {len(self.objects)};"         
        
    def _class_enum(self) -> None: 
        payload = [
            obj.name.upper() + f"= {i}" 
            for i, obj in enumerate(self.objects)
        ]
        
        self.data += textwrap.dedent(f"""
            #[repr(u8)]
            #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
            pub enum {self.enum_name} {{
                {self._fmt_payload(payload, ", ")}
            }}
        """)
    
    def _to_str_impl(self) -> None:
        to_str_cases = [
            f'{self.enum_name}::{obj.name.upper()} => "{obj.name}".to_string(),'
            for obj in self.objects
        ]

        self.data += textwrap.dedent(f"""
            impl ToString for {self.enum_name} {{
                fn to_string(&self) -> String {{
                    match self {{
                        {self._fmt_payload(to_str_cases)}
                    }}
                }}
            }}
        """)
        
    def _obj_cast_err_enum(self) -> None: 
        self.data += textwrap.dedent("""
            #[derive(thiserror::Error, Debug)]
            pub enum ObjectCastError {{
                #[error(
                    "Attempted to cast {{0}} into a {self.enum_name}, but no object corresponds with said string!"
                )]
                StringCastError(String),

                #[error("Attempted to cast {{0}} into a {self.enum_name}, but no object holds said discriminant!")]
                UIntCastError(u8),
            }}
        """)
    
    def _from_str_impl(self) -> None: 
        from_str_cases = [
            f'"{obj.name}" => Ok({self.enum_name}::{obj.name.upper()}),'
            for obj in self.objects
        ]
        
        self.data += textwrap.dedent(f"""
            impl TryFrom<&str> for {self.enum_name} {{
                type Error = ObjectCastError;

                fn try_from(value: &str) -> Result<Self, Self::Error> {{
                    match value {{
                        {self._fmt_payload(from_str_cases)}
                        _ => Err(ObjectCastError::StringCastError(value.to_string())),
                    }}
                }}
            }}
        """)
    
    def _has_children_method(self) -> None: 
        has_children_cases = [
            f'{self.enum_name}::{obj.name.upper()} => {str(hasattr(obj, "children") and len(obj.children) > 0).lower()},'
            for obj in self.objects
        ]
        
        self.data += textwrap.dedent(f"""
            #[inline]
            pub const fn has_children(&self) -> bool {{
                match self {{
                    {self._fmt_payload(has_children_cases)}
                }}
            }}

        """)
    
    def _has_pair_method(self) -> None: 
        has_pair_cases = [
            f'{self.enum_name}::{obj.name.upper()} => {str(hasattr(obj, "pair") and obj.pair is not None).lower()},'
            for obj in self.objects
        ]
        
        self.data += textwrap.dedent(f"""
            #[inline]
            pub const fn has_pair(&self) -> bool {{
                match self {{
                    {self._fmt_payload(has_pair_cases)}
                }}
            }}  
        """)
  
    def _is_first_in_pair_method(self) -> None: 
        is_first_in_pair_cases = [
            f'{self.enum_name}::{obj.name.upper()} => {str(hasattr(obj, "pair") and obj.pair is not None and obj.pair[1] == 1).lower()},'
            for obj in self.objects
        ]
        
        self.data += textwrap.dedent(f"""
            #[inline]
            pub const fn is_first_in_pair(&self) -> bool {{
                match self {{
                    {self._fmt_payload(is_first_in_pair_cases)}
                }}
            }}                         
        """)
        
    def _is_second_in_pair_method(self) -> None: 
        is_second_in_pair_cases = [
            f'{self.enum_name}::{obj.name.upper()} => {str(hasattr(obj, "pair") and obj.pair is not None and obj.pair[1] == 2).lower()},'
            for obj in self.objects
        ]
        
        self.data += textwrap.dedent(f"""
            #[inline]
            pub const fn is_second_in_pair(&self) -> bool {{
                match self {{
                    {self._fmt_payload(is_second_in_pair_cases)}
                }}
            }}              
        """)
        
    def _begin_impl_block(self) -> None: 
        self.data += f"impl {self.enum_name} " + "{{"
    
    def _end_impl_block(self) -> None: 
        self.data += "}}"
    
    def _from_u8_impl(self) -> None: 
        from_u8_cases = [
            f"{i} => " + "Self::" + object.name.upper() 
            for i, object in enumerate(self.objects)
        ]
        
        self.data += textwrap.dedent(f"""
            impl TryFrom<u8> for {self.enum_name} {{
                type Error = ObjectCastError;

                fn try_from(value: u8) -> Result<Self, Self::Error> {{
                    if value > OBJECT_COUNT {{
                        return Err(ObjectCastError::UIntCastError(value));
                    }}

                    Ok(match value {{
                        {self._fmt_payload(from_u8_cases, ", ")},
                        _ => unreachable!("should've been checked at beginning of try_from"),
                    }})
                }}
            }}
        """)
    
    def _default_impl(self) -> None: 
        self.data += textwrap.dedent(f"""                              
            impl Default for {self.enum_name} {{
                fn default() -> Self {{
                    Self::UNKNOWN
                }}
            }}                 
        """)
    
    def _into_u8_impl(self) -> None: 
        into_u8_cases = [
            "Self::" + object.name.upper() + f" => {i}" 
            for i, object in enumerate(self.objects)
        ]
        
        self.data += textwrap.dedent(f"""
            impl Into<u8> for {self.enum_name} {{
                fn into(self) -> u8 {{
                    match self {{
                        {self._fmt_payload(into_u8_cases, ", ")}
                    }}
                }}
            }}
        """)
    
