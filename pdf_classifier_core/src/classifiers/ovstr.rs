use crate::constraints::overrides::OverrideStream;
use crate::context::Context;
use crate::page::Page;
use crate::threading::pool::{JobResult, ThreadPool};
use crate::{
    classifiers::{ClassificationError, committed::CommittedClassifier},
    constraints::overrides::OverrideStreamExitCase,
    context::ContextUpdateHistory,
};
use std::fmt::Debug;
use std::ops::Deref;
use std::sync::Mutex;
use std::sync::MutexGuard;

pub struct StreamPtr(pub &'static Mutex<Box<dyn OverrideStream>>);

impl Deref for StreamPtr {
    type Target = Mutex<Box<dyn OverrideStream>>;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl Debug for StreamPtr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple(std::any::type_name::<StreamPtr>())
            .field(&format_args!("{:p}", &self.0 as *const _))
            .finish()
    }
}

type OverrideStreamBorrow<'a> = MutexGuard<'a, Box<dyn OverrideStream + 'static>>;

#[derive(Debug)]
pub struct OverrideStreamClassifier {
    pub for_stream: StreamPtr,
    pub base: CommittedClassifier,
}

impl OverrideStreamClassifier {
    pub fn new(for_stream: StreamPtr, base: CommittedClassifier) -> Self {
        Self { base, for_stream }
    }

    pub fn till_stream_end(mut self) -> Result<CommittedClassifier, ClassificationError> {
        tracing::info!(
            "beginning override for: {}",
            self.for_stream.lock().unwrap()
        );
        loop {
            let mut stream = self.for_stream.lock().unwrap();
            let history = ContextUpdateHistory::new();

            tracing::trace!(
                "initialized override step for page: {}",
                self.base.current_page
            );
            let step = stream.step(&self.base.ctx, self.base.current_page);
            let _ = self.base.handle_override(step, history)?;

            let exit_case = Self::should_break_from_exit_case(
                &self.base.ctx,
                self.base.current_page,
                &mut self.base.thread_pool,
                stream,
            );

            if exit_case {
                tracing::trace!(
                    "hit exit case for stream on page {}",
                    self.base.current_page
                );
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
                tracing::trace!("evaluating for exit case on page {}", current_page);

                thread_pool.classify_unchecked(class, current_page);

                let results = loop {
                    if let Some(results) = thread_pool.poll()
                        && results.len() > 0
                    {
                        break results;
                    }
                };

                debug_assert!(
                    results.len() == 1,
                    "should only have one classification result while iterating in override stream context!",
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
                    } => unreachable!(
                        "shouldn't have any other job results when polling in override stream context."
                    ),
                }
            }
            OverrideStreamExitCase::Exit => true,
        }
    }
}
