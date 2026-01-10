mod constraints;
mod context;
mod defer;
mod result_map;
mod unknown;

use crate::classifier::context::ClassifierContext;
use crate::classifier::defer::{CompleteDeferBlock, IncompleteDeferBlock};
use crate::ffi::ClassificationResult;
use crate::generated::generated_object_types::{KnownObject, OBJECT_COUNT};
use crate::page::Page;
use std::rc::Rc;

pub struct Classifier {
    soft_constraints: Vec<constraints::SoftConstraints>,
    hard_constraints: Vec<constraints::HardConstraints>,
    context: ClassifierContext,
    pending_extraction: Vec<(KnownObject, Rc<ClassificationResult>)>,
    defers: Vec<CompleteDeferBlock>,
    current_defer: Option<IncompleteDeferBlock>,
    current_parent: Option<KnownObject>,
    current_page: Page,
}

impl Classifier {
    pub fn new(start_page: Page, end_page: Page) -> Self {
        Self {
            soft_constraints: vec![],
            hard_constraints: vec![],
            pending_extraction: vec![],
            context: ClassifierContext::new(start_page, end_page),
            defers: vec![],
            current_defer: None,
            current_parent: None,
            current_page: 0u32.into(),
        }
    }

    pub fn begin(&mut self) -> () {
        for page in 0..self.context.total_page_count {
            // i != page_num if start_page > 0
            // therefore we add start_page to offset it correctly.
            let page: Page = Page::new(page + self.context.start_page.num);

            if self.in_defer() {
                self.iter_with_deferrence(page);
            } else {
                self.iter_with_non_deferrence(page);
            }
        }
    }

    fn in_defer(&self) -> bool {
        self.current_defer.is_some()
    }

    fn iter_with_deferrence(&self, page: Page) -> () {
        let hypothesis = self.infer(page, true);
    }

    fn iter_with_non_deferrence(&self, page: Page) -> () {
        let inference = self.infer(page, false);
        
    }

    fn apply_soft_constraints(&self, class: KnownObject, page: Page) -> f32 {
        let mut total_score: f32 = 0.0;

        for soft in &self.soft_constraints {
            let score = soft.eval(&self.context, class, page);

            total_score += score;
        }

        total_score
    }

    fn eval_hard_constraints(&self, class: KnownObject) -> bool {
        for hard in &self.hard_constraints {
            // we assume any debug tracing happens within .eval()
            let result = hard.eval(&self.context, class, self.current_page);

            if !result {
                return false;
            }
        }

        true
    }

    fn infer(&self, page: Page, avoid_soft: bool) -> KnownObject {
        let mut seen_largest_obj: KnownObject = unsafe { std::mem::transmute(0u8) }; // see safety block below
        let mut seen_largest_score: f32 = 0.0;

        for obj_id in 0..OBJECT_COUNT {
            //  SAFETY:
            //      We can safely transmute obj_id into KnownObject
            //      Because the Python side ensures OBJECT_COUNT == the number of variants in Knownobject
            //      This therefore mitigates any chance of unnecessary branching
            let class: KnownObject = unsafe { std::mem::transmute(obj_id as u8) };

            if !self.eval_hard_constraints(class) {
                continue; // we do not give any leniancy to objects who don't meet structural constraints
            }

            if !avoid_soft {
                let score = self.apply_soft_constraints(class, page);
                if score > seen_largest_score {
                    seen_largest_score = score;
                    seen_largest_obj = class;
                }
            }
        }

        seen_largest_obj
    }

    // ! I vaguely remember deference ignoring soft constraints, but
    // ! With a bit of thought it doesn't make too much sense,
    // ! Since, right now this is unimportant I'll come back to it
    // /// Identical to [Classifier::infer], but doesn't apply any soft constraints
    // /// Since, otherwise we'd have to do a flag check which would be overhead in this context
    // /// (being that inferrence happens O(n) times per page, where n is the number of objects)
    // /// I assume the maintenence loss makes it worth it
    // fn infer_within_defer(&self, page: Page) -> KnownObject {
    //     let mut seen_largest_obj: KnownObject = unsafe { std::mem::transmute(0u8) }; // see safety block below
    //     let mut seen_largest_score: f32 = 0.0;

    //     for obj_id in 0..OBJECT_COUNT {
    //         //  SAFETY:
    //         //      We can safely transmute obj_id into KnownObject
    //         //      Because the Python side ensures OBJECT_COUNT == the number of variants in Knownobject
    //         //      This therefore mitigates any chance of unnecessary branching
    //         let class: KnownObject = unsafe { std::mem::transmute(obj_id as u8) };

    //         if !self.eval_hard_constraints(class) {
    //             continue; // we do not give any leniancy to objects who don't meet structural constraints
    //         }
    //     }

    //     seen_largest_obj
    // }

    /// Indicates that some page was classified incorrectly and caused a break (n)
    /// Treats n..? as "deferred" where they don't affect the dynamic weights and are hypotheses
    /// "?" refers to the next independent page (page of an independent type) like another subchapter
    fn defer(&mut self, page: Page) -> () {
        self.current_defer = Some(IncompleteDeferBlock::new(
            page,
            self.context.total_page_count as usize,
        ))
    }

    fn end_defer(&mut self, end_page: Page) -> () {
        let current_defer = self
            .current_defer
            .take()
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
