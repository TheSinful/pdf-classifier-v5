use crate::{
    generated::generated_object_types::{KnownObject, ObjectCastError},
    page::Page,
    result_map::ClassifierResultMap,
};
use std::{collections::HashMap, ops::Index};

pub struct Context {
    pub pages: HashMap<Page, KnownObject>,
    pub page_count: usize,
    pub start_page: Page,
    pub end_page: Page,
    pub current_parent: KnownObject,
    pub prev_parents: ClassifierResultMap<KnownObject>,
}

pub enum ContextUpdate {
    Decision(Page, KnownObject),
    NewParent(KnownObject),
}

#[derive(thiserror::Error, Debug)]
pub enum ContextError {
    #[error(transparent)]
    ClassOutOfBounds(#[from] ObjectCastError),
}

pub type ContextUpdateHistory = Vec<ContextUpdate>;

impl Context {
    pub fn previous_page_inference(&self, from_page: Page) -> &KnownObject {
        if from_page.num == 0 {
            panic!("Attempted to access previous page of page 0 (no negative pages exist)")
        } else if from_page.num as usize > self.page_count {
            panic!("Attempted to access previous page of a page outside page bounds!");
        }

        self.pages.index(&(from_page - 1u32.into()))
    }

    pub fn is_first_page(&self, page: Page) -> bool {
        self.start_page == page
    }

    pub fn decide(
        &mut self,
        page: Page,
        class: KnownObject,
        difference_history: &mut ContextUpdateHistory,
    ) -> Result<(), ContextError> {
        self.pages.insert(page, class);

        difference_history.push(ContextUpdate::Decision(page, class));

        // todo: need fallback to ensure that if on this decision we're incorrect, that we can revert correctly.
        let current_discrim: u8 = class.into();
        let parent_discrim: u8 = self.current_parent.into();
        if current_discrim > parent_discrim {
            self.current_parent = class;
        }

        Ok(())
    }
}
