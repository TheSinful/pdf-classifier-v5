use super::Classifier;
use super::error::ClassifierError;
use crate::{
    generated::generated_object_types::KnownObject, page::Page, result_map::ClassifierResultMap,
    score::Score, weighting::ScoreManager,
};

pub struct DeferenceClassifier {
    hypotheses: ClassifierResultMap<KnownObject>,
    start_page: Page,
    current_page: Page,
    score_manager: ScoreManager,
    largest_defer_size: usize,
}

impl DeferenceClassifier {
    /// avg_hypotheses_size will refer to the average number of pages in a deference block.
    pub fn new(current_page: Page, score_manager: ScoreManager, largest_defer_size: usize) -> Self {
        Self {
            hypotheses: ClassifierResultMap::with_capacity(largest_defer_size), // will be dropped on end_defer
            start_page: current_page,
            current_page,
            score_manager,
            largest_defer_size,
        }
    }

    pub fn finalize(mut self, end_page: Page) -> Result<Classifier, ClassifierError> {
        let largest_defer = if self.largest_defer_size > self.current_page.into() {
            self.largest_defer_size
        } else {
            self.current_page.into()
        };

        Ok(Classifier::new(
            self.current_page,
            end_page,
            self.score_manager,
            self.largest_defer_size,
        ))
    }
}
