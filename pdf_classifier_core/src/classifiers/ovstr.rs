use crate::constraints::overrides::OverrideStream;
use crate::threading::pool::JobResult;
use crate::{
    classifiers::{ClassificationError, committed::CommittedClassifier},
    constraints::overrides::OverrideStreamExitCase,
};
use std::fmt::Debug;
use std::ops::Deref;
use std::sync::Mutex;
use std::sync::MutexGuard;
use tracing::{instrument, trace};

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

    #[instrument(skip(self), fields(stream = %self.for_stream.lock().unwrap()))]
    pub fn till_stream_end(mut self) -> Result<CommittedClassifier, ClassificationError> {
        let mut first_iter = true;

        loop {
            let mut stream = self.for_stream.lock().unwrap();

            if first_iter {
                if Self::should_break_from_exit_case(&mut self.base, &stream)? {
                    return Ok(self.base);
                }

                first_iter = false;
            }

            let step = stream.step(&self.base.ctx, self.base.current_page());
            self.base.handle_override(step)?;

            let results = self.base.thread_pool.poll_draining();

            self.base.handle_polled_results(results);

            let exit_case = Self::should_break_from_exit_case(&mut self.base, &stream)?;

            if exit_case {
                tracing::trace!(
                    "hit exit case for stream on page {}",
                    self.base.current_page()
                );

                return Ok(self.base);
            }
        }
    }

    fn should_break_from_exit_case(
        base: &mut CommittedClassifier,
        stream: &OverrideStreamBorrow<'_>,
    ) -> Result<bool, ClassificationError> {
        let current_page = base.current_page();
        let ctx = &base.ctx;
        let thread_pool = &mut base.thread_pool;

        match stream.should_exit(ctx, current_page) {
            OverrideStreamExitCase::IfClassifiedAs(class) => {
                tracing::trace!("evaluating for exit case on page {}", current_page);

                thread_pool.classify_unchecked(class, current_page);

                let results = thread_pool
                    .poll_blocking()
                    .expect("lost result for exit stream case!");

                match results {
                    JobResult::Classification {
                        page,
                        res,
                        as_class,
                    } => {
                        debug_assert!(
                            page == current_page,
                            "expected to remain in a sequential iteration while in override stream,
                            but received a result for page {} while current page is {}",
                            page,
                            current_page
                        );
                        debug_assert!(
                            as_class == class,
                            "expected singular result resulted class to match override stream's class."
                        );

                        trace!(?page, ?class, "found exit case, calling extraction");

                        if res.is_ok() {
                            // since the classification was Ok, extraction will automatically begin on it
                            base.decide_as(class, current_page);

                            Ok(true)
                        } else {
                            Ok(false)
                        }
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
            OverrideStreamExitCase::Exit => Ok(true),
        }
    }
}
