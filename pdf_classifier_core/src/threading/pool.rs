use crate::ffi::{OkUserResult, Shared};
use crate::{
    ffi::{ClassificationResult, UserResult},
    generated::generated_object_types::KnownObject,
    page::Page,
    threading::{ClassifyFuturePayload, ExtractionFuturePayload, JobType, WorkerThread},
};
use cxx::CxxString;
use futures::task::noop_waker_ref;
use futures::{StreamExt, stream::FuturesUnordered};
use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::{
    path::PathBuf,
    task::{Context, Poll},
};
use tracing::{debug_span, instrument};

#[derive(Debug)]
pub struct ThreadPool {
    pub queue: Vec<PendingJob>,
    busy_workers: Vec<WorkerThread>,
    available_workers: Vec<WorkerThread>, // worker threads live aslong as the classification
    classification_futures:
        FuturesUnordered<Box<dyn Future<Output = ClassifyFuturePayload> + 'static + Unpin>>,
    extraction_futures:
        FuturesUnordered<Box<dyn Future<Output = ExtractionFuturePayload> + 'static + Unpin>>,
    pending_extract_shared: HashMap<Page, Box<OkUserResult<Shared>>>,
}

#[derive(Debug)]
pub struct PendingJob {
    class: KnownObject,
    page: Page,
    job_type: JobType,
}

pub enum JobResult {
    Classification {
        page: Page,
        res: Result<(), String>, // erase shared data since threadpool internally manages it
        as_class: KnownObject,
    },
    Extraction {
        page: Page,
        res: UserResult<CxxString>, // classifier decides what to do with extracted data, (currently nothing, but in the future should be some ambigous type that can be passed to py)
        as_class: KnownObject,
    },
}

impl Display for JobResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobResult::Classification {
                page,
                res,
                as_class,
            } => write!(
                f,
                "ClassificationJob {{page: {}, is_ok: {}, class: {}}}",
                page,
                res.is_ok(),
                as_class
            ),
            JobResult::Extraction {
                page,
                res,
                as_class,
            } => write!(
                f,
                "ExtractionJob {{page: {}, is_ok: {}, class: {}}}",
                page,
                res.is_ok(),
                as_class
            ),
        }
    }
}

impl ThreadPool {
    pub fn new(num_threads: usize, doc_path: PathBuf) -> Self {
        let mut available_workers = Vec::new();

        for i in 0..num_threads {
            let worker = WorkerThread::spawn(doc_path.clone(), i as u32);
            available_workers.push(worker);
        }

        tracing::trace!(
            "initialized threadpool with {} workers",
            available_workers.len()
        );

        Self {
            available_workers,
            busy_workers: Vec::with_capacity(num_threads),
            pending_extract_shared: HashMap::new(),
            queue: Vec::with_capacity(num_threads),
            classification_futures: FuturesUnordered::new(),
            extraction_futures: FuturesUnordered::new(),
        }
    }

    pub fn poll(&mut self) -> Option<Vec<JobResult>> {
        let mut cx = Context::from_waker(noop_waker_ref());
        let mut results: Vec<JobResult> = vec![];

        while let Some(worker) = self.available_workers.pop() {
            if self.work_available_worker(worker).is_none() {
                break;
            }
        }

        while let Poll::Ready(Some(fut)) = self.classification_futures.poll_next_unpin(&mut cx) {
            results.push(self.handle_classification_result(fut.class, fut.page, fut.result));

            self.push_worker_to_available(fut.worker_id, &fut.page);
        }

        while let Poll::Ready(Some(fut)) = self.extraction_futures.poll_next_unpin(&mut cx) {
            results.push(self.handle_extraction_result(fut.class, fut.page, fut.result));

            self.push_worker_to_available(fut.worker_id, &fut.page);
        }

        if self.exhausted(&results) {
            None
        } else {
            Some(results)
        }
    }

    pub fn available_workers_count(&self) -> usize {
        self.available_workers.len()
    }

    fn exhausted(&self, results: &Vec<JobResult>) -> bool {
        self.queue.is_empty() && results.is_empty() && self.busy_workers.is_empty()
    }

    fn work_available_worker(&mut self, worker: WorkerThread) -> Option<()> {
        let Some(pending) = self.queue.pop() else {
            self.available_workers.push(worker);
            return None;
        };

        let _span = debug_span!("work_available_worker", worker = worker.id).entered();

        match pending.job_type {
            JobType::Classification => {
                self.push_classification_to_worker(pending.class, pending.page, &worker)
            }
            JobType::Extraction => {
                let shared_data = self.pending_extract_shared.remove(&pending.page).expect(&format!("Expected a viable shared output from extraction of page {}, but none existed for page", pending.page));

                self.push_extraction_to_worker(pending.class, pending.page, shared_data, &worker)
            }
        };

        self.busy_workers.push(worker);
        Some(())
    }

    #[instrument(name = "push_classification_job", skip(self), fields(available_workers = self.available_workers_count()))]
    pub fn classify(&mut self, class: KnownObject, page: Page) -> () {
        self.queue.push(PendingJob {
            class,
            page,
            job_type: JobType::Classification,
        })
    }

    #[instrument(name = "push_extraction_job", skip(self), fields(available_workers = self.available_workers_count()))]
    pub fn extract(&mut self, class: KnownObject, page: Page) -> () {
        tracing::trace!(
            "pushing extraction job: {}, pg{} ({} workers)",
            class,
            page,
            self.available_workers_count()
        );

        self.queue.push(PendingJob {
            class,
            page,
            job_type: JobType::Extraction,
        })
    }

    #[instrument(name = "push_unchecked_classification_job", skip(self), fields(available_workers = self.available_workers_count()))]
    pub fn classify_unchecked(&mut self, class: KnownObject, page: Page) -> () {
        let worker = self
            .available_workers
            .pop()
            .expect("should've had an available worker when spawning a thread unsafely.");

        self.push_classification_to_worker(class, page, &worker);
        self.busy_workers.push(worker);
    }

    fn push_worker_to_available(&mut self, id: u32, page: &Page) -> () {
        let found = self
            .busy_workers
            .iter()
            .position(|x| x.id == id)
            .map(|idx| self.busy_workers.remove(idx))
            .expect(&format!(
                "Expected a worker with id {} from page {}",
                id, page.0
            ));

        self.available_workers.push(found);
    }

    fn handle_classification_result(
        &mut self,
        class: KnownObject,
        page: Page,
        res: ClassificationResult,
    ) -> JobResult {
        match res {
            UserResult::Ok(res) => {
                let res_ptr = Box::new(res);

                self.pending_extract_shared.insert(page, res_ptr);
                self.extract(class, page);

                JobResult::Classification {
                    page,
                    res: Ok(()),
                    as_class: class,
                }
            }
            UserResult::Fail(res) => JobResult::Classification {
                page,
                res: Err(res.extract_fail_rsn().to_string()),
                as_class: class,
            },
        }
    }

    fn handle_extraction_result(
        &self,
        class: KnownObject,
        page: Page,
        res: UserResult<CxxString>,
    ) -> JobResult {
        match res {
            UserResult::Ok(_) => {
                // TODO: export data, currently just goes to user to write to a file
                JobResult::Extraction {
                    page,
                    res: res,
                    as_class: class,
                }
            }
            UserResult::Fail(_) => JobResult::Extraction {
                page,
                res,
                as_class: class,
            },
        }
    }

    fn push_classification_to_worker(
        &mut self,
        class: KnownObject,
        page: Page,
        worker: &WorkerThread,
    ) -> () {
        let call = worker.classify(class, page);
        let fut = Box::new(Box::pin(call));

        self.classification_futures.push(fut);
    }

    fn push_extraction_to_worker(
        &mut self,
        class: KnownObject,
        page: Page,
        res: Box<OkUserResult<Shared>>,
        worker: &WorkerThread,
    ) -> () {
        let call = worker.extract(class, res.extract_payload_as_shared(), page);
        let fut = Box::new(Box::pin(call));

        self.extraction_futures.push(fut);
    }
}
