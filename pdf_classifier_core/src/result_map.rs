use crate::{generated::generated_object_types::KnownObject, page::Page};
use std::collections::HashMap;
pub struct ClassifierResultMap(pub HashMap<Page, Vec<KnownObject>>);

impl ClassifierResultMap {
    pub fn insert_in_page(&mut self, page: Page, class: KnownObject) -> () {
        if let Some(list) = self.0.get_mut(&page) {
            list.push(class);
        } else {
            self.0.insert(page, vec![class]);
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            0: HashMap::with_capacity(capacity),
        }
    }
}
