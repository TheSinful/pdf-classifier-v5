use super::ClassificationError;
use super::STEP_COUNT;
use crate::classifiers::ovstr::StreamPtr;
use crate::constraints::overrides::OverrideAction;
use crate::context::Context;
use crate::context::ContextUpdateHistory;
use crate::ffi::UserResult;
use crate::generated::generated_object_types::KnownObject;
use crate::generated::generated_object_types::KnownObject::UNKNOWN;
use crate::generated::overrides::OVERRIDE_STREAMS;
use crate::inferencer::Inferencer;
use crate::obj_list::KnownObjectList;
use crate::page::Page;
use crate::page_lock::PageLock;
use crate::stream::Streamer;
use crate::threading::pool::ThreadPool;
use crate::{generated::overrides::OVERRIDES, threading::pool::JobResult};
use cxx::CxxString;
use tracing::debug_span;
use tracing::info;
use tracing::instrument;

// intentionally a "transparent" struct so other classifier modes can depend upon it
#[derive(Debug)]
pub struct CommittedClassifier {
    pub inferencer: Inferencer,
    pub thread_pool: ThreadPool,
    pub should_defer: bool,
    pub ctx: Context,
    pub streamer: Option<Streamer>,
    pub page_lock: PageLock,
}

impl CommittedClassifier {
    pub fn new(
        start_page: Page,
        inferencer: Inferencer,
        thread_pool: ThreadPool,
        context: Context,
    ) -> Self {
        Self {
            page_lock: PageLock::Unlocked(start_page),
            inferencer,
            thread_pool,
            ctx: context,
            should_defer: false,
            streamer: Some(
                Streamer::new()
                    .expect("cannot initialize classification without linking to a frontend!"),
            ),
        }
    }

    #[cfg(test)]
    pub fn without_streamer(
        start_page: Page,
        inferencer: Inferencer,
        thread_pool: ThreadPool,
        context: Context,
    ) -> Self {
        Self {
            page_lock: PageLock::Unlocked(start_page),
            inferencer,
            thread_pool,
            ctx: context,
            should_defer: false,
            // no_increment: false,
            streamer: None,
        }
    }

    #[instrument(skip(self), fields(current_page = %self.current_page(), err))]
    pub fn step(&mut self) -> Result<(), ClassificationError> {
        self.step_inner()?;
        self.poll();

        Ok(())
    }

    pub fn current_page(&self) -> Page {
        *self.page_lock.get()
    }

    pub fn step_inner(&mut self) -> Result<(), ClassificationError> {
        let history = ContextUpdateHistory::new();
        let current_page = self.current_page();
        let winner = self
            .inferencer
            .infer(&mut self.ctx, current_page, &KnownObjectList::new())?;

        if winner == KnownObject::UNKNOWN {
            return self.decide_as(KnownObject::UNKNOWN, history, self.current_page());
        }

        if let Some(action) = self._override(winner) {
            return self.handle_override(action, history);
        }

        self.thread_pool.classify(winner, self.current_page());
        self.decide_as(winner, history, self.current_page())
    }

    #[instrument(skip(self, history))]
    pub fn handle_override(
        &mut self,
        override_result: OverrideAction,
        history: ContextUpdateHistory,
    ) -> Result<(), ClassificationError> {
        match override_result {
            OverrideAction::Skip => {
                self.decide_as(KnownObject::UNKNOWN, history, self.current_page())
            }
            OverrideAction::InferAs(class) => {
                self.thread_pool.classify(class, self.current_page());
                self.decide_as(class, history, self.current_page())
            }
            OverrideAction::ClassifyAs(class) => {
                self.thread_pool.extract(class, self.current_page());
                self.decide_as(class, history, self.current_page())
            }
        }
    }

    #[instrument(skip(self, history))]
    pub fn decide_as(
        &mut self,
        class: KnownObject,
        mut history: ContextUpdateHistory,
        page: Page,
    ) -> Result<(), ClassificationError> {
        self.ctx.decide(page, class, &mut history)?;
        self.increment_current_page(STEP_COUNT.into());

        Ok(())
    }

    fn increment_current_page(&mut self, by: Page) -> () {
        // intentionally a no-op while deferral holds the lock: backfill
        // decisions must not advance the committed cursor
        let _ = self.page_lock.try_increment_by(by);
    }

    #[instrument(skip(self))]
    pub fn decide_and_classify_as(
        &mut self,
        class: KnownObject,
        page: Page,
    ) -> Result<(), ClassificationError> {
        self.decide_as(class, ContextUpdateHistory::new(), page)?;

        self.thread_pool.classify(class, page);

        Ok(())
    }

    #[instrument(skip(self), fields(page = %self.current_page(), winner), ret)]
    pub fn _override(&self, winner: KnownObject) -> Option<OverrideAction> {
        for over in OVERRIDES {
            let action = over.eval(&self.ctx, winner, self.current_page());
            if action.is_none() {
                continue;
            }

            let action = action.unwrap();
            tracing::trace!(
                page = %self.current_page(),
                r#override = %over,
            );

            return Some(action);
        }

        None
    }

    pub fn poll(&mut self) -> Option<()> {
        if let Some(results) = self.thread_pool.poll() {
            if !results.is_empty() {
                let _span = debug_span!("polled_results", result_count = results.len()).entered();
                results.into_iter().for_each(|result| match result {
                    JobResult::Classification {
                        page,
                        res,
                        as_class,
                    } => self.handle_classification_result(page, as_class, res),
                    JobResult::Extraction {
                        page,
                        res,
                        as_class,
                    } => self.handle_extraction_result(page, as_class, res),
                });
            }
            return Some(());
        }
        None
    }

    #[instrument(skip(self))]
    pub fn handle_classification_result(
        &mut self,
        page: Page,
        class: KnownObject,
        res: Result<(), String>,
    ) -> () {
        if let Err(_) = res {
            if !self.should_defer {
                self.defer();
            }
        }
    }

    #[instrument(skip(self))]
    pub fn handle_extraction_result(
        &mut self,
        page: Page,
        class: KnownObject,
        res: UserResult<CxxString>,
    ) -> () {
        if let UserResult::Fail(e) = res {
            if !self.should_defer {
                tracing::error!(
                    "failed to extract page {}, as class {}\n {}",
                    page,
                    class.to_string(),
                    e.extract_fail_rsn()
                );
            }

            if let Some(streamer) = &mut self.streamer {
                streamer
                    .send_data(
                        page,
                        UNKNOWN,
                        &format!("failed extraction as {}", class).as_bytes(),
                    )
                    .unwrap();
            }

            return;
        }

        let payload = res.extract_inner_as_string().unwrap();

        if let Some(streamer) = &mut self.streamer {
            streamer.send_data(page, class, payload.as_bytes()).unwrap();
        }
    }

    pub fn defer(&mut self) -> () {
        debug_assert!(
            !self.should_defer,
            "Shouldn't attempt to defer when already in deferral."
        );
        self.should_defer = true;

        let _span = debug_span!(
            "signal_deferral",
            page = %self.current_page(),
            tasks_in_queue = self.thread_pool.queue.len()
        )
        .entered();

        info!("polling before entering deferral");
        while let Some(_) = self.poll() {}
    }

    pub fn should_enter_override_stream(&self) -> Option<StreamPtr> {
        for stream in OVERRIDE_STREAMS.iter() {
            if stream
                .lock()
                .unwrap()
                .should_enter(&self.ctx, self.current_page())
            {
                return Some(StreamPtr(stream));
            }
        }

        None
    }
}

#[cfg(test)]
pub fn init_test_classifier(page_start: u32, page_end: u32) -> crate::classifiers::Classifier {
    use crate::classifiers::Classifier;
    use crate::constants::test_constants::NUM_TEST_THREADS;
    use crate::tests::init::get_LARGE_TEST_DOC_test_path;

    let start_page = Page(page_start);
    let end_page = Page(page_end);

    Classifier::new(
        start_page,
        end_page,
        NUM_TEST_THREADS,
        get_LARGE_TEST_DOC_test_path(),
    )
}

#[cfg(test)]
pub fn init_test_committed_classifier(start_page: u32, end_page: u32) -> CommittedClassifier {
    use crate::constants::test_constants::NUM_TEST_THREADS;
    use crate::tests::init::get_LARGE_TEST_DOC_test_path;
    use std::thread::sleep;
    use std::time::Duration;

    sleep(Duration::from_millis(200));
    CommittedClassifier::without_streamer(
        Page(start_page),
        Inferencer::new(),
        ThreadPool::new(NUM_TEST_THREADS, get_LARGE_TEST_DOC_test_path()),
        Context::new(Page(start_page), Page(end_page)),
    )
}

#[cfg(test)]
mod tests {
    use crate::classifiers::committed::init_test_committed_classifier;
    use crate::constants::test_constants::*;
    use crate::generated::generated_object_types::KnownObject;
    use crate::page::Page;
    use crate::tests::init::*;
    use serde_json::to_string as json_to_string;
    use std::io::Write;
    use tracing::info;

    #[derive(serde::Serialize)]
    struct Decision {
        pub page: Page,
        pub class: String,
    }

    #[test]
    fn test_committed_classifier_start() {
        let end_page = LARGE_TEST_DOC_START_PAGE + 6;
        let mut classifier = init_test_committed_classifier(LARGE_TEST_DOC_START_PAGE, end_page);

        tracing::trace!(
            "init classifier with page-range [{},{}]",
            LARGE_TEST_DOC_START_PAGE,
            end_page
        );

        for _ in LARGE_TEST_DOC_START_PAGE..end_page {
            classifier.step().unwrap();
            classifier.poll();
        }

        info!("[TEST] finalized stepping, polling");
        while let Some(_) = classifier.poll() {}

        let decisions: Vec<String> = classifier
            .ctx
            .pages
            .into_iter()
            .map(|(page, class)| {
                let decision = Decision {
                    page,
                    class: class.to_string(),
                };

                json_to_string(&decision).unwrap()
            })
            .collect();

        let out_dir = get_TEST_OUTPUT_DIR_test_path();
        let mut file = std::fs::File::create(out_dir.join("test_classify_full_run.json")).unwrap();
        for decision in decisions {
            file.write(decision.as_bytes()).unwrap();
        }
    }

    #[test]
    fn test_first_page_classification() {
        let mut classifier = init_test_committed_classifier(0, 1);
        classifier.step().unwrap();
        if let Some(known_object) = classifier.ctx.pages.get(&Page(0)) {
            assert!(*known_object == KnownObject::CHAPTER)
        } else {
            panic!("no page was found to be classified")
        }
    }
}
