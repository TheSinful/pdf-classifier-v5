use crate::classifiers::committed::CommittedClassifier;
use crate::classifiers::defer::DeferralClassifier;
use crate::classifiers::ovstr::OverrideStreamClassifier;
use crate::context::Context;
use crate::context::ContextError;
use crate::context::ContextUpdateHistory;
use crate::inferencer::InferenceError;
use crate::inferencer::Inferencer;
use crate::page::Page;
use crate::threading::pool::ThreadPool;
use std::path::PathBuf;

mod committed;
mod defer;
mod ovstr;

const STEP_COUNT: usize = 1;

pub struct ClassificationStep {
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

enum ClassifierState {
    Committed(CommittedClassifier),
    Deferral(DeferralClassifier),
    OverrideStream(OverrideStreamClassifier),
    Transition,
}

impl ClassifierState {
    pub fn committed<'a>(&'a mut self) -> &'a mut CommittedClassifier {
        match self {
            ClassifierState::Committed(c) => c,
            ClassifierState::Deferral(d) => &mut d.base,
            ClassifierState::OverrideStream(o) => &mut o.base,
            ClassifierState::Transition => unreachable!(
                "shouldn't attempt to access inner committed classifier while transitioning"
            ),
        }
    }

    pub fn current_page(&self) -> Page {
        match self {
            ClassifierState::Committed(c) => c.current_page,
            ClassifierState::Deferral(c) => c.current_page,
            ClassifierState::OverrideStream(c) => c.base.current_page,
            ClassifierState::Transition => unreachable!(
                "shouldn't attempt to access inner committed classifier while transitioning"
            ),
        }
    }
}

type StepList = Vec<ClassificationStep>;

pub struct Classifier {
    state: ClassifierState,
    end_page: Page,
}

impl Classifier {
    pub fn new(
        start_page: Page,
        end_page: Page,
        allocated_threads: usize,
        doc_path: PathBuf,
    ) -> Self {
        Self {
            state: ClassifierState::Committed(CommittedClassifier::new(
                start_page,
                Inferencer::new(),
                ThreadPool::new(allocated_threads, doc_path),
                Context::new(start_page, end_page),
            )),
            end_page,
        }
    }

    pub fn run(mut self) -> Result<StepList, ClassificationError> {
        loop {
            if self.state.current_page().num == self.end_page.num {
                let mut state = std::mem::replace(&mut self.state, ClassifierState::Transition);
                let steps = std::mem::replace(&mut state.committed().steps, vec![]);
                break Ok(steps);
            }

            let state = std::mem::replace(&mut self.state, ClassifierState::Transition);
            match state {
                ClassifierState::Committed(classifier) => {
                    if classifier.should_defer {
                        self.state = ClassifierState::Deferral(DeferralClassifier::new(classifier));
                    } else if let Some(stream) = classifier.should_enter_override_stream() {
                        self.state = ClassifierState::OverrideStream(OverrideStreamClassifier::new(
                            stream, classifier,
                        ))
                    }
                }
                ClassifierState::Deferral(classifier) => {
                    self.state = ClassifierState::Committed(classifier.find_next_independent()?);
                }
                ClassifierState::OverrideStream(classifier) => {
                    self.state = ClassifierState::Committed(classifier.till_stream_end()?);
                }
                ClassifierState::Transition => unreachable!(
                    "shouldn't be in transitonal stage outside of specific transition states."
                ),
            };
        }
    }
}
