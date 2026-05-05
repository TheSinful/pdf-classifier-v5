#[cfg(test)]
use crate::classifiers::Classifier;
#[cfg(test)]
use crate::constants::test_constants::NUM_TEST_THREADS;
#[cfg(test)]
use crate::tests::init::get_LARGE_TEST_DOC_test_path;

use super::ClassificationError;
use super::ClassificationStep;
use super::STEP_COUNT;
use crate::classifiers::ovstr::StreamList;
use crate::constraints::overrides::OverrideAction;
use crate::context::Context;
use crate::context::ContextUpdateHistory;
use crate::debug_assert_if;
use crate::ffi::UserResult;
use crate::generated::generated_object_types::KnownObject;
use crate::generated::overrides::OVERRIDE_STREAMS;
use crate::inferencer::Inferencer;
use crate::obj_list::KnownObjectList;
use crate::page::Page;
use crate::threading::pool::ThreadPool;
use crate::{generated::overrides::OVERRIDES, threading::pool::JobResult};

// intentionally a "transparent" struct so other classifier modes can depend upon it
pub struct CommittedClassifier {
    pub current_page: Page,
    pub inferencer: Inferencer,
    pub thread_pool: ThreadPool,
    pub should_defer: bool,
    pub ctx: Context,
    pub steps: Vec<ClassificationStep>,
    no_increment: bool,
}

impl CommittedClassifier {
    pub fn new(
        start_page: Page,
        inferencer: Inferencer,
        thread_pool: ThreadPool,
        context: Context,
    ) -> Self {
        Self {
            current_page: start_page,
            inferencer,
            thread_pool,
            ctx: context,
            should_defer: false,
            no_increment: false,
            steps: vec![],
        }
    }

    pub fn step(&mut self) -> Result<&ClassificationStep, ClassificationError> {
        log::trace!("stepping with classifier over page {}", self.current_page,);

        log::trace!("begin page {}", self.current_page);

        let step = self.step_inner()?;
        self.steps.push(step);

        self.poll();

        log::trace!("end page {}", self.current_page);

        while let Some(_) = self.poll() {}

        Ok(self
            .steps
            .last()
            .expect("steps should contain a member as one was pushed just now"))
    }

    pub fn step_inner(&mut self) -> Result<ClassificationStep, ClassificationError> {
        let history = ContextUpdateHistory::new();
        let winners = self.inferencer.infer(
            &mut self.ctx,
            vec![self.current_page],
            &KnownObjectList::new(),
        )?;

        debug_assert_if!(
            STEP_COUNT == 1,
            winners.len() == 1,
            "Should've only received one winner from Inferencer while stepping sequentially."
        );

        let winner = winners[0];
        if winner == KnownObject::UNKNOWN {
            return self.decide_as(KnownObject::UNKNOWN, history);
        }

        if let Some(action) = self._override(winner) {
            return self.handle_override(action, history);
        }

        self.thread_pool.classify(winner, self.current_page);
        self.decide_as(winner, history)
    }

    pub fn handle_override(
        &mut self,
        override_result: OverrideAction,
        history: ContextUpdateHistory,
    ) -> Result<ClassificationStep, ClassificationError> {
        match override_result {
            OverrideAction::Skip => self.decide_as(KnownObject::UNKNOWN, history),
            OverrideAction::InferAs(class) => {
                self.thread_pool.classify(class, self.current_page);
                self.decide_as(class, history)
            }
            OverrideAction::ClassifyAs(class) => {
                self.thread_pool.extract(class, self.current_page);
                self.decide_as(class, history)
            }
        }
    }

    pub fn decide_as(
        &mut self,
        class: KnownObject,
        mut history: ContextUpdateHistory,
    ) -> Result<ClassificationStep, ClassificationError> {
        self.ctx.decide(self.current_page, class, &mut history)?;
        self.increment_current_page(STEP_COUNT as u32);

        return Ok(ClassificationStep {
            pages_iterated_over: STEP_COUNT,
            context_updates: history,
            notes: "".to_string(),
        });
    }

    fn increment_current_page(&mut self, by: u32) -> () {
        if self.no_increment {
            log::trace!(
                "skipping incrementation of current_page by {} since no_increment is enabled.",
                by
            );
            return;
        }

        self.current_page.num += by;
        log::trace!("incremented current_page to page {}", self.current_page)
    }

    pub fn decide_and_classify_as(&mut self, class: KnownObject, page: Page) -> () {
        let step = self.decide_as(class, ContextUpdateHistory::new()).unwrap();
        self.steps.push(step);
        self.thread_pool.classify(class, page);
    }

    pub fn _override(&self, winner: KnownObject) -> Option<OverrideAction> {
        for over in OVERRIDES {
            let action = over.eval(&self.ctx, winner, self.current_page);
            if action.is_none() {
                continue;
            }

            let action = action.unwrap();
            log::trace!(
                "Override {} forced condition {} for page {}!",
                over.to_string(),
                action.to_string(),
                self.current_page
            );

            return Some(action);
        }

        None
    }

    pub fn poll(&mut self) -> Option<()> {
        let results = self.thread_pool.poll();

        if results.is_none() {
            log::warn!("attempted to poll despite threadpool being exhausted.");

            return None;
        }

        results
            .into_iter()
            .flatten()
            .for_each(|result| match result {
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
            });

        Some(())
    }

    pub fn handle_classification_result(
        &mut self,
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

            if !self.should_defer {
                self.defer();
            }
        }
    }

    pub fn handle_extraction_result(
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

    pub fn defer(&mut self) -> () {
        debug_assert!(
            !self.should_defer,
            "Shouldn't attempt to defer when already in deferral."
        );

        log::trace!(
            "attempting to defer with {} tasks in queue",
            self.thread_pool.queue.len()
        );

        while let Some(_) = self.poll() {}

        self.should_defer = true
    }

    pub fn should_enter_override_stream(&self) -> Option<StreamList> {
        for stream in OVERRIDE_STREAMS.iter() {
            if stream
                .lock()
                .unwrap()
                .should_enter(&self.ctx, self.current_page)
            {
                return Some(stream);
            }
        }

        None
    }
}

#[cfg(test)]
pub fn init_test_classifier(page_start: u32, page_end: u32) -> Classifier {
    let start_page = Page::new(page_start);
    let end_page = Page::new(page_end);

    Classifier::new(
        start_page,
        end_page,
        NUM_TEST_THREADS,
        get_LARGE_TEST_DOC_test_path(),
    )
}

#[cfg(test)]
pub fn init_test_committed_classifier(start_page: u32, end_page: u32) -> CommittedClassifier {
    CommittedClassifier::new(
        start_page.into(),
        Inferencer::new(),
        ThreadPool::new(NUM_TEST_THREADS, get_LARGE_TEST_DOC_test_path()),
        Context::new(start_page.into(), end_page.into()),
    )
}

#[cfg(test)]
mod tests {
    use crate::constants::test_constants::*;
    use crate::classifiers::committed::init_test_committed_classifier;
    use crate::context::ContextUpdate;
    use crate::generated::generated_object_types::KnownObject;
    use crate::tests::init::*;
    use serde_json::to_string;
    use std::io::Write;

    #[derive(serde::Serialize)]
    struct Decision {
        pub page: u32,
        pub class: String,
    }

    #[test]
    fn test_committed_classifier_start() {
        let end_page = LARGE_TEST_DOC_START_PAGE + 6;
        let mut classifier = init_test_committed_classifier(LARGE_TEST_DOC_START_PAGE, end_page);

        log::trace!(
            "init classifier with page-range [{},{}]",
            LARGE_TEST_DOC_START_PAGE,
            end_page
        );

        for _page in LARGE_TEST_DOC_START_PAGE..end_page {
            log::trace!("begin page {}", _page);

            let _ = classifier.step().unwrap();

            classifier.poll();

            log::trace!("end page {}", _page);
        }

        while let Some(_) = classifier.poll() {}

        let mut decisions: Vec<String> = vec![];
        for step in &classifier.steps {
            let update = &step.context_updates[0];

            match update {
                ContextUpdate::Decision(page, known_object) => {
                    let decision = Decision {
                        page: page.num,
                        class: known_object.to_string(),
                    };

                    decisions.push(to_string(&decision).unwrap());
                }
                ContextUpdate::NewParent(_) => {}
            }
        }

        let out_dir = get_TEST_OUTPUT_DIR_test_path();
        let mut file = std::fs::File::create(out_dir.join("test_classify_full_run.json")).unwrap();
        for decision in decisions {
            file.write(decision.as_bytes()).unwrap();
        }
    }

    #[test]
    fn test_first_page_classification() {
        let mut classifier = init_test_committed_classifier(0, 1);

        match classifier.step().unwrap().context_updates[0] {
            ContextUpdate::Decision(page, known_object) => {
                assert!(page.num == 0);
                assert!(known_object == KnownObject::CHAPTER)
            }
            ContextUpdate::NewParent(_) => panic!("unexpected update"),
        }
    }
}
