use crate::{
    generated::{
        generated_object_types::{KnownObject, OBJECT_COUNT, ObjectCastError},
        reflected_objects::{OBJECTS, is_independent},
    },
    page::Page,
    result_map::ClassifierResultMap,
};
use std::{collections::HashMap, ops::Index};

#[derive(Debug)]
pub struct Context {
    pub pages: HashMap<Page, KnownObject>,
    pub page_count: usize,
    pub start_page: Page,
    pub end_page: Page,
    pub current_parent: KnownObject,
    pub prev_parents: ClassifierResultMap,
    pub guarantee_failures: ClassifierResultMap,
}

#[derive(Debug)]
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
    pub fn new(start_page: Page, end_page: Page) -> Self {
        let page_count = (end_page.0 - start_page.0) as usize;

        tracing::info!(
            "initialized context with page range: [{},{}]",
            start_page,
            end_page
        );
        Self {
            pages: HashMap::new(),
            page_count,
            start_page,
            end_page,
            current_parent: OBJECTS[0].name,
            prev_parents: ClassifierResultMap::with_capacity(OBJECT_COUNT as usize),
            guarantee_failures: ClassifierResultMap::with_capacity(page_count),
        }
    }

    pub fn previous_page_inference(&self, current_page: Page) -> &KnownObject {
        if self.is_first_page(current_page) {
            panic!(
                "Attempted to access previous page of page {} (no negative pages exist)",
                current_page
            )
        } else if current_page.0 > self.end_page.0 {
            panic!(
                "Attempted to access previous page of a page outside page bounds! ({} > {})",
                current_page.0, self.page_count
            );
        }

        self.pages.index(&current_page.previous())
    }

    pub fn is_first_page(&self, page: Page) -> bool {
        self.start_page == page
    }

    pub fn guarantee_failure_of(&mut self, class: KnownObject, page: Page) -> () {
        self.guarantee_failures.insert_in_page(page, class);
    }

    pub fn get_decision(&self, page: Page) -> Option<&KnownObject> {
        self.pages.get(&page)
    }

    fn update_current_parent(
        &mut self,
        page: Page,
        class: KnownObject,
        difference_history: &mut ContextUpdateHistory,
    ) -> () {
        tracing::debug!("current parent updated to {}", class.to_string());
        self.prev_parents.insert_in_page(page, self.current_parent);
        self.current_parent = class;
        difference_history.push(ContextUpdate::NewParent(class));
    }

    pub fn decide(
        &mut self,
        page: Page,
        class: KnownObject,
        difference_history: &mut ContextUpdateHistory,
    ) -> Result<(), ContextError> {
        self.pages.insert(page, class);

        difference_history.push(ContextUpdate::Decision(page, class));

        if self.is_first_page(page) {
            return Ok(());
        }

        if !is_independent(class) {
            return Ok(());
        }

        // todo: need fallback to ensure that if on this decision we're incorrect, that we can revert correctly.
        let current_discrim: u8 = class.into();
        let parent_discrim: u8 = self.current_parent.into();
        if current_discrim != parent_discrim {
            self.update_current_parent(page, class, difference_history);
        }

        return Ok(());
    }
}
