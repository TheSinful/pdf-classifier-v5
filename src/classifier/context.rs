use crate::classifier::result_map::ClassifierResultMap;
use crate::generated::generated_object_types::KnownObject;
use crate::page::Page;

pub struct ClassifierContext {
    pub total_page_count: u32,
    // can either be inference or classifications
    // but the user doesn't need to know that
    // rather, we just concretely update the map with classifications
    pub decisions: ClassifierResultMap<KnownObject>,
    pub start_page: Page,
    pub end_page: Page,
    pub current_parent: KnownObject,
}

impl ClassifierContext {
    pub fn new(start_page: Page, end_page: Page) -> Self {
        let total_pages = (end_page - start_page).num;

        Self {
            // current_page: start_page,
            total_page_count: total_pages,
            decisions: ClassifierResultMap::new(total_pages as usize),
            start_page,
            end_page,
            current_parent: crate::generated::reflected_objects::OBJECTS[0].name,
        }
    }

    pub fn decide(&mut self, page: Page, class: KnownObject) -> () {
        self.decisions.set_page(page, class);
    }

    pub fn previous_page_inference(&self, current_page: Page) -> &KnownObject {
        let prev_page = Page::from(current_page.num - 1);

        self.decisions
            .get_page_results(prev_page)
            .last()
            .expect("Attempted to reference previous page, but no inference members existed")
    }

    pub fn is_first_page(&self, current_page: Page) -> bool {
        current_page == self.start_page
    }
}
