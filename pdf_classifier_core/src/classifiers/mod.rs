use crate::classifiers::committed::CommittedClassifier;
use crate::classifiers::defer::DeferralClassifier;
use crate::classifiers::defer::DeferralExitCase;
use crate::classifiers::ovstr::OverrideStreamClassifier;
use crate::classifiers::ovstr::StreamPtr;
use crate::context::Context;
use crate::context::ContextError;
use crate::generated::generated_object_types::KnownObject;
use crate::inferencer::InferenceError;
use crate::inferencer::Inferencer;
use crate::page::Page;
use crate::threading::pool::ThreadPool;
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::Level;
use tracing::error;
use tracing::field::Empty;
use tracing::info_span;
use tracing::instrument;
use tracing::span;

mod committed;
mod defer;
mod ovstr;

const STEP_COUNT: usize = 1;

#[derive(thiserror::Error, Debug)]
pub enum ClassificationError {
    #[error(transparent)]
    InferenceError(#[from] InferenceError),

    #[error(transparent)]
    ContextRecordError(#[from] ContextError),

    #[error(transparent)]
    LoggerInitializationError(#[from] tracing_subscriber::util::TryInitError),

    #[error("Attempted to increment page count while page lock was locked.")]
    PageLockLocked,

    #[error("Expected record within context does not exist")]
    RecordNotFound
    
}

#[derive(Debug)]
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
            ClassifierState::Committed(c) => c.current_page(),
            ClassifierState::Deferral(c) => c.current_page(),
            ClassifierState::OverrideStream(c) => c.base.current_page(),
            ClassifierState::Transition => unreachable!(
                "shouldn't attempt to access inner committed classifier while transitioning"
            ),
        }
    }

    pub fn resulted_structure(self) -> HashMap<Page, KnownObject> {
        match self {
            ClassifierState::Committed(committed_classifier) => committed_classifier.ctx.pages,
            ClassifierState::Deferral(deferral_classifier) => deferral_classifier.base.ctx.pages,
            ClassifierState::OverrideStream(override_stream_classifier) => {
                override_stream_classifier.base.ctx.pages
            }
            ClassifierState::Transition => {
                panic!("Attempted to get result when classifier was transitioning.")
            }
        }
    }
}

#[derive(Debug)]
pub struct Classifier {
    state: ClassifierState,
    end_page: Page,
}

macro_rules! ignore_state_change_if {
    ($condition: expr, $msg: expr, $classifier: expr, $state: expr) => {
        if $condition {
            tracing::info!($msg);
            $classifier.step()?;
            $state = ClassifierState::Committed($classifier);
            continue;
        }
    };
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

    #[instrument(skip_all, fields(page = %classifier.current_page()))]
    fn schedule_deferral(&mut self, mut classifier: CommittedClassifier) -> () {
        self.drain_pool(&mut classifier);

        self.state = ClassifierState::Deferral(DeferralClassifier::new(classifier));
    }

    #[instrument(skip_all, fields(page = %classifier.current_page()))]
    fn schedule_override_stream(
        &mut self,
        mut classifier: CommittedClassifier,
        stream: StreamPtr,
    ) -> () {
        self.drain_pool(&mut classifier);

        self.state =
            ClassifierState::OverrideStream(OverrideStreamClassifier::new(stream, classifier))
    }

    fn drain_pool(&mut self, classifier: &mut CommittedClassifier) -> () {
        let poll = classifier.thread_pool.poll_draining();
        classifier.handle_polled_results(poll);
    }

    #[instrument(name = "exit_override_stream", skip_all, fields(ended_on_page = Empty, ended_on_class = Empty))]
    fn exit_ovstr(
        &mut self,
        classifier: OverrideStreamClassifier,
    ) -> Result<(), ClassificationError> {
        self.state = ClassifierState::Committed(classifier.till_stream_end()?);

        Ok(())
    }

    #[instrument(name = "start_classifiation_loop", skip_all, fields(from_page = %self.state.current_page(), to_page = %self.end_page))]
    pub fn run(mut self) -> Result<HashMap<Page, KnownObject>, ClassificationError> {
        loop {
            let span = span!(
                Level::INFO,
                "classification",
                page = %self.state.current_page()
            );
            let _guard = span.enter();

            let mut state = std::mem::replace(&mut self.state, ClassifierState::Transition);
            if state.current_page().0 >= self.end_page.0 {
                let classifier = state.committed();
                self.drain_pool(classifier);
                return Ok(state.resulted_structure());
            }

            match state {
                ClassifierState::Committed(mut classifier) => {
                    ignore_state_change_if!(
                        classifier.current_page() == classifier.ctx.start_page,
                        "ignoring state change since first page",
                        classifier,
                        self.state
                    );

                    if classifier.should_defer {
                        self.schedule_deferral(classifier);
                    } else if let Some(stream) = classifier.should_enter_override_stream() {
                        self.schedule_override_stream(classifier, stream);
                    } else {
                        tracing::debug!("state remained committed.");
                        classifier.step()?;
                        self.state = ClassifierState::Committed(classifier);
                    }
                }
                ClassifierState::Deferral(classifier) => {
                    let _ =
                        info_span!("enter_deferral", page = %classifier.current_page()).entered();

                    match classifier.find_next_independent()? {
                        DeferralExitCase::NoAnchorFound(mut classifier) => {
                            let poll = classifier.thread_pool.poll_draining();
                            classifier.handle_polled_results(poll);
                            return Ok(classifier.ctx.pages);
                        }
                        DeferralExitCase::Successful(classifier) => {
                            self.state = ClassifierState::Committed(classifier);
                        }
                    }
                }
                ClassifierState::OverrideStream(classifier) => self.exit_ovstr(classifier)?,
                ClassifierState::Transition => unreachable!(
                    "shouldn't be in transitonal stage outside of specific transition states."
                ),
            }
        }
    }
}
