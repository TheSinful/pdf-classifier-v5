use crate::generated::generated_object_types::KnownObject;


pub struct ClassifierContext {
    pub current_parent: KnownObject    
}

impl ClassifierContext {
    pub fn previous_page(&self) -> KnownObject {
        todo!()
    } 
}