use crate::page::Page;
use std::ops::{Index, IndexMut};

#[derive(Clone)]
pub struct ClassifierResultMap<T> {
    /// Equivalent to a key-value structure because:
    /// page_num == i (when i is an index of inner)
    /// Where the value is just an array of results
    inner: Vec<Vec<T>>,
}

impl<T> ClassifierResultMap<T> {
    pub fn new(total_pages: usize) -> Self {
        let mut vec: Vec<Vec<T>> = Vec::with_capacity(total_pages);

        for _ in 0..total_pages {
            vec.push(Vec::new());
        }

        Self { inner: vec }
    }

    pub fn get_page_results(&self, page: Page) -> &Vec<T> {
        let index: usize = page.into();

        self.inner.index(index)
    }

    pub fn set_page(&mut self, page: Page, insert: T) -> () {
        let wrapper = self.get_page_results_mut(page);
        wrapper.push(insert);
    }

    pub fn get_page_best_result(&self, page: Page) -> Option<&T> {
        self.get_page_results(page).last()
    }

    fn get_page_results_mut(&mut self, page: Page) -> &mut Vec<T> {
        let index: usize = page.into();

        self.inner.index_mut(index)
    }
}
