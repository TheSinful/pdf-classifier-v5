mod context;
mod defer;
mod unknown;
mod weights;

use crate::page::Page;
use crate::{classifier::defer::DeferBlock, generated::generated_object_types::KnownObject};
use weights::structural;

struct Classifier {
    structure_weights: structural::StructuralWeights,
    current_page: Page,
    defers: Vec<DeferBlock>,
    current_defer: Option<DeferBlock>,
    total_pages: u32,
}

impl Classifier {
    fn begin(&mut self) -> () {
        for page in 0..self.total_pages {
            let defer_condition: bool = true; // placeholder since no logic to detect defers exist 
            if defer_condition {
                self.defer(Page::from(page));
            }

            // in other functions, maybe hold a flag for if we're deferred? so as to not update dynamic variables
        }
    }

    fn infer(&self, page: Page) -> KnownObject {
        // apply structural bounds

        todo!()
    }

    /// Indicates that some page was classified incorrectly and caused a break (n)
    /// Treats n..? as "deferred" where they don't affect the dynamic weights and are hypotheses
    /// "?" refers to the next independent page (page of an independent type) like another subchapter
    fn defer(&mut self, page: Page) -> () {
        self.current_defer = Some(DeferBlock::new(page))
    }

    fn end_defer(&mut self, end_page: Page) -> () {
        let current_defer = self
            .current_defer
            .expect("Attempted to end a defer block without being in a defer block.");

        let start_page = current_defer.get_start_page();

        for page in start_page.num..end_page.num {
            // re-iterate over pages that would have only been hypotheses at this point
            // todo: run classify over each page in the window, disegard independents since we would have already found them
        }

        self.defers.push(current_defer.complete(end_page));
        self.current_defer = None;
    }
}
