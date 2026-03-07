use crate::{
    context::ContextUpdateHistory,
    page::Page,
    weighting::ScoreManager,
};

mod defer;
mod error;

use defer::DeferenceClassifier;

type DecisionResult = ();

pub struct Classifier {
    pub current_page: Page,
    end_page: Page,
    score_manager: ScoreManager,
    largest_defer_size: usize,
}

pub struct ClassifcationStep {
    pages_iterated_over: usize,
    context_updates: ContextUpdateHistory,
    notes: String,
}

#[derive(thiserror::Error, Debug)]
pub enum ClassificationError {}

impl Classifier {
    pub fn new(
        start_page: Page,
        end_page: Page,
        score_manager: ScoreManager,
        largest_defer_size: usize,
    ) -> Self {
        Self {
            current_page: start_page,
            score_manager,
            largest_defer_size,
            end_page,
        }
    }

    pub fn step(&mut self) -> Result<ClassifcationStep, ClassificationError> {
        if let Some(value) = self.validate_page_boundary() {
            return Ok(value);
        }

        self.current_page.num += 1;

        Ok(ClassifcationStep {
            pages_iterated_over: todo!(),
            context_updates: todo!(),
            notes: "".to_string(),
        })
    }

    fn validate_page_boundary(&self) -> Option<ClassifcationStep> {
        if self.current_page.num >= self.end_page.num {
            return Some(ClassifcationStep {
                pages_iterated_over: 0,
                context_updates: vec![],
                notes: format!(
                    "No pages left to complete! (pg{}/pg{})",
                    self.current_page, self.end_page
                ),
            });
        }
        None
    }

    pub fn defer(self) -> DeferenceClassifier {
        DeferenceClassifier::new(
            self.current_page,
            self.score_manager,
            self.largest_defer_size,
        )
    }
}
