// mod context;
// pub mod defer;
// pub mod result_map;

// mod unknown;

// use crate::classifier::context::Context;
// use crate::classifier::defer::{CompleteDeferBlock, IncompleteDeferBlock};
// use crate::ffi::{ClassificationResult, UserResult};
// use crate::generated::generated_object_types::{KnownObject, OBJECT_COUNT};
// use crate::generated::reflected_objects::is_independent;
// use crate::page::Page;
// use crate::threading::pool::{JobResult, ThreadPool};
// use crate::score::Score;
// use log::{error, info, trace, warn};
// use std::cell::RefCell;
// use std::path::PathBuf;
// use std::rc::Rc;

// pub type IncompleteDeferBlockPtr = Rc<RefCell<Option<IncompleteDeferBlock>>>;

// const MIN_DEFER_INDEPENDENCE_SCORE: f32 = 90.0;

// #[derive(Clone, Copy)]
// struct ObjectScore {
//     class: KnownObject,
//     score: Score,
// }

// impl ObjectScore {
//     pub fn with_float(class: KnownObject, score: f32) -> Self {
//         Self {
//             class,
//             score: score.into(),
//         }
//     }

//     pub fn with_score(class: KnownObject, score: Score) -> Self {
//         ObjectScore { class, score }
//     }
// }

// // see https://github.com/TheSinful/pdf-classifier-v5/issues/3
// // type InferenceArray = [(AlwaysSome<KnownObject>, Score); OBJECT_COUNT];

// pub struct Classifier {
//     soft_constraints: Vec<SoftConstraints>,
//     hard_constraints: Vec<HardConstraints>,
//     context: Context,
//     pending_extraction: Vec<(KnownObject, Rc<ClassificationResult>)>,
//     defers: Vec<CompleteDeferBlock>,
//     current_defer: IncompleteDeferBlockPtr,
//     in_defer: Rc<bool>,
//     current_parent: Option<KnownObject>,
//     current_page: Page,
//     thread_pool: ThreadPool,
//     objects: Vec<KnownObject>,
// }

// impl Classifier {
//     pub fn new(start_page: Page, end_page: Page, num_threads: usize, doc_path: PathBuf) -> Self {
//         let current_defer = Rc::new(RefCell::new(None));
//         let mut objects = Vec::<KnownObject>::with_capacity(OBJECT_COUNT);
//         let defer_flag = Rc::new(false);

//         for obj_id in 0..OBJECT_COUNT {
//             //  SAFETY:
//             //      We can safely transmute obj_id into KnownObject
//             //      Because the Python side ensures OBJECT_COUNT == the number of variants in Knownobject
//             //      This therefore mitigates any chance of unnecessary branching
//             let class: KnownObject = unsafe { std::mem::transmute(obj_id as u8) };
//             objects.push(class);
//         }

//         Self {
//             soft_constraints: vec![],
//             hard_constraints: vec![],
//             pending_extraction: vec![],
//             defers: vec![],
//             context: Context::new(start_page, end_page, defer_flag.clone()),
//             in_defer: defer_flag,
//             current_defer: current_defer.clone(),
//             current_parent: None,
//             current_page: 0u32.into(),
//             thread_pool: ThreadPool::new(num_threads, doc_path, current_defer),
//             objects,
//         }
//     }

//     pub fn begin(&mut self) -> () {
//         for page in 0..self.context.total_page_count {
//             info!("Running state machine upon page {}", page);
//             // i != page_num if start_page > 0
//             // therefore we add start_page to offset it correctly.
//             let page = Page::new(page + self.context.start_page.num);

//             if self.in_defer() {
//                 trace!("Running page {} within deference!", page.num);
//                 self.iter_with_deference(page);
//             } else {
//                 trace!("Running page {} without deference!", page.num);
//                 self.iter_with_non_deference(page);
//             }

//             let results = self.thread_pool.poll();
//             info!("Recieved {} results when polling.", results.len());
//             self.handle_jobs(results);
//         }
//     }

//     fn handle_jobs(&mut self, results: Vec<JobResult>) -> () {
//         for result in results {
//             match result {
//                 JobResult::Classification {
//                     page,
//                     res,
//                     as_class,
//                 } => self.handle_classification_result(page, res, as_class),
//                 JobResult::Extraction {
//                     page,
//                     res,
//                     as_class,
//                 } => self.handle_extraction_result(page, res, as_class),
//             }
//         }
//     }

//     fn handle_classification_result(
//         &mut self,
//         page: Page,
//         res: Result<(), String>,
//         class: KnownObject,
//     ) -> () {
//         match res {
//             Ok(_) => {
//                 info!(
//                     "Page {} classified as class {}",
//                     page.num,
//                     class.to_string()
//                 );
//                 self.context.classify(page, class);
//             }
//             Err(e) => {
//                 warn!(
//                     "Page {} failed to classify as {}\n With given error string: {}\nEntering deference...",
//                     page.num,
//                     class.to_string(),
//                     e
//                 );
//                 self.defer(page);
//             }
//         }
//     }

//     fn handle_extraction_result(
//         &mut self,
//         page: Page,
//         res: UserResult<()>,
//         class: KnownObject,
//     ) -> () {
//         match res {
//             UserResult::Ok(_) => {
//                 // todo: in the future, once we propogate extracted data we should pass it back to Python here.
//                 info!(
//                     "Successfully extracted page {} as class {}",
//                     page.num,
//                     class.to_string()
//                 );
//             }
//             UserResult::Fail(e) => {
//                 error!(
//                     "Page {} classified sucessfully as class {}\nYet failed extraction with error {}!\n**Page will be left as unknown.**",
//                     page.num,
//                     class.to_string(),
//                     e.extract_fail_rsn()
//                 );
//                 // todo: logic to leave page {page} as unknown.
//             }
//         }
//     }

//     fn in_defer(&self) -> bool {
//         *self.in_defer.as_ref()
//     }

//     fn iter_with_deference(&self, page: Page) -> () {
//         let hypotheses: Vec<ObjectScore> = self
//             .infer(page, true)
//             .into_iter()
//             .filter(|o| self.filter_dependent(o))
//             .collect();

//         /*
//             the issue i'm thinking about, is that we're forcing hyptoheses
//             as single "decisions" within context, as only independents
//             this is an issue because then we waste resources running inference
//             while attempting to find the next independent

//         */
//     }

//     fn filter_dependent(&self, o: &ObjectScore) -> bool {
//         is_independent(o.class) && o.score > MIN_DEFER_INDEPENDENCE_SCORE.into()
//     }

//     fn iter_with_non_deference(&mut self, page: Page) -> () {
//         let inferences = self.infer(page, false);
//         let best_inference = inferences.last().expect("Infer returned an empty Vec!");

//         self.context.classify(page, best_inference.class);
//     }

//     fn apply_soft_constraints(&self, class: KnownObject, page: Page) -> Score {
//         let mut total_score: Score = 0.0.into();

//         for soft_constraint in &self.soft_constraints {
//             let score = soft_constraint.eval(&self.context, class, page);

//             total_score += score.into();
//         }

//         total_score
//     }

//     fn eval_hard_constraints(&self, class: KnownObject) -> bool {
//         for hard_constraint in &self.hard_constraints {
//             // we assume any debug tracing happens within .eval()
//             let result = hard_constraint.eval(&self.context, class, self.current_page);

//             if !result {
//                 return false;
//             }
//         }

//         true
//     }

//     fn infer(&self, page: Page, avoid_soft: bool) -> Vec<ObjectScore> {
//         let mut vec = Vec::<ObjectScore>::with_capacity(OBJECT_COUNT);

//         for class in &self.objects {
//             let class = class.clone();

//             if self.eval_hard_constraints(class) {
//                 vec.push(ObjectScore::with_float(class, 0.0));
//             } else {
//                 vec.push(ObjectScore::with_score(class, Score::NEG_INFINITY()));
//                 continue;
//             }

//             if !avoid_soft {
//                 vec.push(ObjectScore::with_score(
//                     class,
//                     self.apply_soft_constraints(class, page),
//                 ));
//             }
//         }

//         vec.sort_unstable_by_key(|f| f.score);

//         vec
//     }

//     /// Indicates that some page was classified incorrectly and caused a break (n)
//     /// Treats n..? as "deferred" where they don't affect the dynamic weights and are hypotheses
//     /// "?" refers to the next independent page (page of an independent type) like another subchapter
//     fn defer(&mut self, page: Page) -> () {
//         let mut borrow = self.current_defer.borrow_mut();
//         *borrow = Some(IncompleteDeferBlock::new(
//             page,
//             self.context.total_page_count as usize,
//         ))
//     }

//     fn end_defer(&mut self, end_page: Page) -> () {
//         let current_defer = self
//             .current_defer
//             .take()
//             .expect("Attempted to end a defer block without being in a defer block.");

//         let start_page = current_defer.get_start_page();

//         for page in start_page.num..end_page.num {
//             // re-iterate over pages that would have only been hypotheses at this point
//             // todo: run classify over each page in the window, disegard independents since we would have already found them
//         }

//         self.defers.push(current_defer.complete(end_page));
//         *self.current_defer.borrow_mut() = None;
//     }
// }
