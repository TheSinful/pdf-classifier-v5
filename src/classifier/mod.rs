use crate::{
    context::{Context, ContextError, ContextUpdateHistory},
    page::Page,
    inferencer::{InferenceError, Inferencer},
};

mod defer;
mod error;

use defer::DeferenceClassifier;

type DecisionResult = ();

pub struct Classifier {
    pub current_page: Page,
    end_page: Page,
    inferencer: Inferencer,
    largest_defer_size: usize,
}

pub struct ClassifcationStep {
    pages_iterated_over: usize,
    context_updates: ContextUpdateHistory,
    notes: String,
}

#[derive(thiserror::Error, Debug)]
pub enum ClassificationError {
    #[error(transparent)]
    ScoreManagerError(#[from] InferenceError),

    #[error(transparent)]
    ContextRecordError(#[from] ContextError),
}

impl Classifier {
    pub fn new(
        start_page: Page,
        end_page: Page,
        inferencer: Inferencer,
        largest_defer_size: usize,
    ) -> Self {
        Self {
            current_page: start_page,
            inferencer,
            largest_defer_size,
            end_page,
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

            log::trace!("end page {}", _page);
        }

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

        #[cfg(debug_assertions)]
        if STEP_COUNT == 1 {
            // slightly over-engineered/future proofed, but ensures this won't be an issue if step_count is changed.
            debug_assert_eq!(
                winners.len(),
                1,
                "Should've only received one winner from ScoreManager while stepping sequentially."
            );
        }

        ctx.decide(self.current_page, winners[0], &mut history)?;
        self.current_page.num += STEP_COUNT as u32;

        Ok(ClassifcationStep {
            context_updates: history,
            notes: "".to_string(),
            pages_iterated_over: STEP_COUNT,
        })
    }

    pub fn defer(self) -> DeferenceClassifier {
        DeferenceClassifier::new(
            self.current_page,
            self.inferencer,
            self.largest_defer_size,
        )
    }
}
