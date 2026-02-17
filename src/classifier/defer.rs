use super::Page;
use super::unknown::Unknown;
use crate::classifier::result_map::ClassifierResultMap;
use crate::ffi::ClassificationResult;
use crate::generated::generated_object_types::KnownObject;
use std::rc::Rc;

#[derive(Clone, Copy)]
pub struct CompleteDeferBlock {
    start_page: Page,
    end_page: Page,
}

impl CompleteDeferBlock {
    pub fn new(start_page: Page, end_page: Page) -> Self {
        Self {
            start_page,
            end_page,
        }
    }

    pub fn get_start_page(&self) -> Page {
        self.start_page
    }
}

#[derive(Clone)]
pub struct IncompleteDeferBlock {
    start_page: Page,
    end_page: Unknown<Page>,
    hypotheses: ClassifierResultMap<(KnownObject, Rc<ClassificationResult>)>,
}

impl IncompleteDeferBlock {
    pub fn new(start_page: Page, total_page_count: usize) -> Self {
        Self {
            start_page,
            end_page: Unknown::pending(),
            hypotheses: ClassifierResultMap::with_capacity(total_page_count),
        }
    }

    pub fn complete(mut self, end_page: Page) -> CompleteDeferBlock {
        self.end_page.define(end_page);
        // .define ensures self.end_page is Some
        CompleteDeferBlock::new(self.start_page, self.end_page.unwrap())
    }

    pub fn get_start_page(&self) -> Page {
        self.start_page
    }
}
