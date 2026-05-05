use crate::constraints::overrides::OverrideStream;
use crate::context::Context;
use crate::page::Page;
use crate::threading::pool::{JobResult, ThreadPool};
use crate::{
    classifiers::{ClassificationError, committed::CommittedClassifier},
    constraints::overrides::OverrideStreamExitCase,
    context::ContextUpdateHistory,
};
use std::sync::Mutex;
use std::sync::MutexGuard;

pub type StreamList = &'static Mutex<Box<dyn OverrideStream>>;
type OverrideStreamBorrow<'a> = MutexGuard<'a, Box<dyn OverrideStream + 'static>>;

pub struct OverrideStreamClassifier {
    pub for_stream: StreamList,
    pub base: CommittedClassifier,
}

impl OverrideStreamClassifier {
    pub fn new(for_stream: StreamList, base: CommittedClassifier) -> Self {
        Self { base, for_stream }
    }

    pub fn till_stream_end(mut self) -> Result<CommittedClassifier, ClassificationError> {
        loop {
            let mut stream = self.for_stream.lock().unwrap();
            let history = ContextUpdateHistory::new();

            let step = stream.step(&self.base.ctx, self.base.current_page);
            let _ = self.base.handle_override(step, history)?;

            let exit_case = Self::should_break_from_exit_case(
                &self.base.ctx,
                self.base.current_page,
                &mut self.base.thread_pool,
                stream,
            );

            if exit_case {
                return Ok(self.base);
            }
        }
    }

    fn should_break_from_exit_case(
        ctx: &Context,
        current_page: Page,
        thread_pool: &mut ThreadPool,
        stream: OverrideStreamBorrow<'_>,
    ) -> bool {
        match stream.should_exit(ctx, current_page) {
            OverrideStreamExitCase::IfClassifiedAs(class) => {
                thread_pool.classify_unchecked(class, current_page);

                let results = loop {
                    if let Some(results) = thread_pool.poll() {
                        break results;
                    }
                };

                debug_assert!(
                    results.len() == 1,
                    "should only have one classification result while iterating in override stream context!"
                );

                match &results[0] {
                    JobResult::Classification {
                        page,
                        res,
                        as_class,
                    } => {
                        debug_assert!(
                            page == &current_page,
                            "expected singular result page to match current page while in override stream context."
                        );
                        debug_assert!(
                            as_class == &class,
                            "expected singular result resulted class to match override stream's class."
                        );

                        return res.is_ok();
                    }
                    JobResult::Extraction {
                        page: _,
                        res: _,
                        as_class: _,
                    } => panic!(
                        "shouldn't have any other job results when polling in override stream context."
                    ),
                }
            }
            OverrideStreamExitCase::Exit => true,
        }
    }
}
