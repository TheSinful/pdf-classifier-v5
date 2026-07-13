use crate::classifiers::committed::CommittedClassifier;
use crate::classifiers::defer::DeferralClassifier;
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
use tracing::field::Empty;
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
            let _ = $classifier.step()?;
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
        while let Some(_) = classifier.poll() {}

        self.state = ClassifierState::Deferral(DeferralClassifier::new(classifier));
    }

    #[instrument(skip_all, fields(page = %classifier.current_page()))]
    fn schedule_override_stream(
        &mut self,
        mut classifier: CommittedClassifier,
        stream: StreamPtr,
    ) -> () {
        while let Some(_) = classifier.poll() {}

        self.state =
            ClassifierState::OverrideStream(OverrideStreamClassifier::new(stream, classifier))
    }

    fn exit_deferral(&mut self, classifier: DeferralClassifier) -> Result<(), ClassificationError> {
        self.state = ClassifierState::Committed(classifier.find_next_independent()?);

        Ok(())
    }

    #[instrument(name = "exit_override_stream", skip_all, fields(ended_on_page = Empty, ended_on_class = Empty))]
    fn exit_ovstr(
        &mut self,
        classifier: OverrideStreamClassifier,
    ) -> Result<(), ClassificationError> {
        self.state = ClassifierState::Committed(classifier.till_stream_end()?);

        let end_page = self.state.current_page();
        let committed = self.state.committed();
        if let Some(end_class) = committed.ctx.get_decision(end_page) {
            committed.decide_and_classify_as(*end_class, end_page)?;
        };

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

            if self.state.current_page().0 >= self.end_page.0 {
                break Ok(self.state.resulted_structure());
            }

            let state = std::mem::replace(&mut self.state, ClassifierState::Transition);
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
                        let _ = classifier.step()?;
                        self.state = ClassifierState::Committed(classifier);
                    }
                }
                ClassifierState::Deferral(classifier) => self.exit_deferral(classifier)?,
                ClassifierState::OverrideStream(classifier) => self.exit_ovstr(classifier)?,
                ClassifierState::Transition => unreachable!(
                    "shouldn't be in transitonal stage outside of specific transition states."
                ),
            }
        }
    }
}
