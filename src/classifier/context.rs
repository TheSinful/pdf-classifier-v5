use crate::classifier::result_map::ClassifierResultMap;
use crate::generated::generated_object_types::KnownObject;
use crate::page::Page;
use std::rc::Rc;

/// Although not explicitly defined elsewhere,
/// the term "decision" will be utilized as an umbrella-term
/// of classification, inference, and hypotheses
/// Since the user (User::classify/User::extract) doesn't care
/// about how the decision was found, they instead act like the context is truth.
/// Or in other words, the user's code will obey regardless of the confidence a decision.
/// So  
pub struct Context {
    pub total_page_count: u32,
    // can either be inference or classifications
    // but the user doesn't need to know that
    // rather, we just concretely update the map with classifications
    pub start_page: Page,
    pub end_page: Page,
    pub current_parent: KnownObject,
    decisions: ClassifierResultMap<KnownObject>,
    hypotheses: ClassifierResultMap<KnownObject>,
    deferring: Rc<bool>,
}

impl Context {
    pub fn new(start_page: Page, end_page: Page, defer_flag: Rc<bool>) -> Self {
        let total_pages = (end_page - start_page).num;

        Self {
            // current_page: start_page,
            total_page_count: total_pages,
            decisions: ClassifierResultMap::with_capacity(total_pages as usize),
            start_page,
            hypotheses: ClassifierResultMap::with_capacity(total_pages as usize), // can be optimized by setting a smaller size
            end_page,
            deferring: defer_flag,
            current_parent: crate::generated::reflected_objects::OBJECTS[0].name,
        }
    }

    pub fn classify(&mut self, page: Page, class: KnownObject) -> () {
        if self.in_defer() {
            panic!("Attempted to decide on a page while defering.");
        }

        self.decisions.set_page(page, class);
    }

    pub fn hypothesize(&mut self, page: Page, class: KnownObject) -> () {
        if !self.in_defer() {
            panic!("Attempted to hypothesize on a page while not defering.");
        }

        self.hypotheses.set_page(page, class);
    }

    fn in_defer(&self) -> bool {
        *self.deferring.as_ref()
    }

    fn flush_hypotheses(&mut self) -> () {
        self.hypotheses = ClassifierResultMap::with_capacity(self.total_page_count as usize);
    }

    pub fn decisions(&self) -> &ClassifierResultMap<KnownObject> {
        if self.in_defer() {
            &self.hypotheses
        } else {
            &self.decisions
        }
    }

    pub fn previous_page_inference(&self, current_page: Page) -> &KnownObject {
        let prev_page = Page::from(current_page.num - 1);

        self.decisions()
            .get_page_results(prev_page)
            .last()
            .expect("Attempted to reference previous page, but no inference members existed")
    }

    pub fn is_first_page(&self, current_page: Page) -> bool {
        current_page == self.start_page
    }
}
