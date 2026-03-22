use crate::debug_assert_if;
use crate::ffi::UserResult;
use crate::generated::generated_object_types::KnownObject;
use crate::threading::pool::JobResult;
use crate::{
    context::{Context, ContextError, ContextUpdateHistory},
    inferencer::{InferenceError, Inferencer},
    page::Page,
    threading::pool::ThreadPool,
};
use std::path::PathBuf;

mod defer;
mod error;

use defer::DeferenceClassifier;

pub struct Classifier {
    pub current_page: Page,
    end_page: Page,
    inferencer: Inferencer,
    largest_defer_size: usize,
    thread_pool: ThreadPool,
}

pub struct ClassifcationStep {
    pub pages_iterated_over: usize,
    pub context_updates: ContextUpdateHistory,
    pub notes: String,
}

#[derive(thiserror::Error, Debug)]
pub enum ClassificationError {
    #[error(transparent)]
    InferenceError(#[from] InferenceError),

    #[error(transparent)]
    ContextRecordError(#[from] ContextError),
}

impl Classifier {
    pub fn new(
        start_page: Page,
        end_page: Page,
        inferencer: Inferencer,
        largest_defer_size: usize,
        num_threads: usize,
        doc_path: PathBuf,
    ) -> Self {
        if start_page.num > end_page.num {
            panic!(
                "end page ({}) is before start page ({})",
                end_page.num, start_page.num
            );
        }

        Self {
            current_page: start_page,
            inferencer,
            largest_defer_size,
            end_page,
            thread_pool: ThreadPool::new(num_threads, doc_path),
        }
    }

    pub fn start(&mut self) -> Result<Vec<ClassifcationStep>, ClassificationError> {
        let mut steps = vec![];
        let mut ctx = Context::new(self.current_page, self.end_page);
        log::trace!(
            "init classifier with page-range [{},{}]",
            self.current_page,
            self.end_page
        );

        for _page in self.current_page.num..self.end_page.num {
            log::trace!("begin page {}", _page);

            let step = self.step(&mut ctx)?;
            steps.push(step);

            self.poll();

            log::trace!("end page {}", _page);
        }

        while let Some(_) = self.poll() {}

        Ok(steps)
    }

    fn step(&mut self, ctx: &mut Context) -> Result<ClassifcationStep, ClassificationError> {
        const STEP_COUNT: usize = 1;

        if self.current_page.num >= self.end_page.num {
            return Ok(ClassifcationStep {
                pages_iterated_over: 0,
                context_updates: vec![],
                notes: format!(
                    "No pages left to complete! (pg{}/pg{})",
                    self.current_page, self.end_page
                ),
            });
        }

        let mut history = ContextUpdateHistory::new();

        let winners = self.inferencer.infer(ctx, vec![self.current_page])?;

        debug_assert_if!(
            STEP_COUNT == 1,
            winners.len() == 1,
            "Should've only received one winner from Inferencer while stepping sequentially."
        );

        // may be a better way to do this, but works as a band-aid for now.
        if winners[0] != KnownObject::UNKNOWN {
            self.thread_pool.schedule(winners[0], self.current_page);
        }

        ctx.decide(self.current_page, winners[0], &mut history)?;
        self.current_page.num += STEP_COUNT as u32;

        Ok(ClassifcationStep {
            context_updates: history,
            notes: "".to_string(),
            pages_iterated_over: STEP_COUNT,
        })
    }

    fn poll(&mut self) -> Option<()> {
        let results = self.thread_pool.poll();

        if results.is_none() {
            log::warn!("attempted to poll despite threadpool being exhausted.");

            return None;
        }

        for result in results.unwrap() {
            match result {
                JobResult::Classification {
                    page,
                    res,
                    as_class,
                } => self.handle_classification_result(page, res, as_class),
                JobResult::Extraction {
                    page,
                    res,
                    as_class,
                } => self.handle_extraction_result(page, res, as_class),
            }
        }

        Some(())
    }

    fn handle_classification_result(
        &self,
        page: Page,
        res: Result<(), String>,
        class: KnownObject,
    ) -> () {
        if let Err(e) = res {
            log::warn!(
                "classification as class {}, failed upon page {}.\n{}",
                class.to_string(),
                page,
                e
            );
            // todo: defer here
        }
    }

    fn handle_extraction_result(
        &mut self,
        page: Page,
        res: UserResult<()>,
        class: KnownObject,
    ) -> () {
        if let UserResult::Fail(e) = res {
            log::error!(
                "failed to extract page {}, as class {}\n {}",
                page,
                class.to_string(),
                e.extract_fail_rsn()
            );
        }

        // todo: handle user data here, would likely just write to a json file
    }

    fn defer(self) -> DeferenceClassifier {
        DeferenceClassifier::new(self.current_page, self.inferencer, self.largest_defer_size)
    }
}
