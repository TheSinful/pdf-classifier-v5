pub mod pool;

use crate::{
    ffi::{self, ClassificationResult, ExtractionResult},
    generated::generated_object_types::KnownObject,
    page::Page,
};
use std::sync::atomic::Ordering;
use std::{
    path::PathBuf,
    sync::{Arc, atomic::AtomicBool},
    thread,
};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::oneshot as tokio_oneshot;

/// The number of items tokio pre-allocates for within Sender<WorkerJob>
/// sizeof(CHANNEL_BUFFER_SIZE) = CHANNEL_BUFFER_SIZE * sizeof(WorkerJob)
pub const CHANNEL_BUFFER_SIZE: usize = 50;

/// Currently just returns a Result<Shared, ()> because we don't return any information of failure
/// Later, this should be like: Result<Shared, String> where String is the failure reason
pub type ClassificationJobResponder = tokio_oneshot::Sender<ClassificationResult>;
pub type ExtractionJobResponder = tokio_oneshot::Sender<ExtractionResult>;

pub type ClassifyFuturePayload = FFIFuture<ClassificationResult>;
pub type ExtractionFuturePayload = FFIFuture<ExtractionResult>;

pub struct FFIFuture<T> {
    pub class: KnownObject,
    pub page: Page,
    pub result: T,
    pub worker_id: u32,
}

pub enum JobType {
    Classification,
    Extraction,
}

pub enum WorkerJob {
    Classify {
        class: KnownObject,
        page: Page,
        responder: ClassificationJobResponder,
    },
    Extract {
        class: KnownObject,
        page: Page,
        shared: ffi::Shared,
        responder: ExtractionJobResponder,
    },
}

pub struct WorkerThread {
    pub id: u32,
    is_working: Arc<AtomicBool>,
    sender: Sender<WorkerJob>,
}

/// Stored within the actual spawned thread,
struct WorkerState<'a> {
    pub ctx: &'a ffi::FzContext,
    pub doc: ffi::Document<'a>,
}

#[derive(thiserror::Error, Debug)]
pub enum ThreadError {
    #[error("Failed to send job to thread!")]
    SendError(#[from] tokio::sync::mpsc::error::SendError<WorkerJob>),
}

impl WorkerThread {
    pub fn spawn(doc_path: PathBuf, id: u32) -> Self {
        let (sender, receiver) = tokio::sync::mpsc::channel::<WorkerJob>(CHANNEL_BUFFER_SIZE);
        let is_working = Arc::new(AtomicBool::new(false));
        let is_working_clone = is_working.clone();

        thread::spawn(move || {
            // SAFETY: nothing is inherently unsafe about context or document creation as both
            // have the lifetiem of the thread that utilizes them.
            // this should be later reflected in the actual definition of fz_context and fz_document.
            // And since context is declared prior to document, document should always Drop first,
            // as fz_drop_document() requires a ctx pointer.
            let ctx = unsafe { ffi::FzContext::new(ffi::STANDARD_CTX_MEM_LIMIT) };
            let doc = unsafe { ffi::Document::new(doc_path, &ctx) };
            let state = WorkerState { ctx: &ctx, doc };

            Self::_loop(state, receiver, is_working_clone);
        });

        Self {
            sender,
            is_working,
            id,
        }
    }

    pub fn is_busy(&self) -> bool {
        self.is_working.load(Ordering::Acquire)
    }

    pub fn classify(
        &self,
        class: KnownObject,
        page: Page,
    ) -> impl Future<Output = ClassifyFuturePayload> + 'static {
        let (sender, receiver) = tokio_oneshot::channel::<ClassificationResult>();
        let worker_sender = self.sender.clone();
        let id = self.id.clone();

        async move {
            worker_sender
                .send(WorkerJob::Classify {
                    class,
                    page,
                    responder: sender,
                })
                .await
                .expect(&format!(
                    "Thread error while attempting to resolve classify future for page {} with class {}",
                    class.to_string(),
                    page.num
                ));

            let rec_result = match receiver.await {
                Ok(classification_result) => classification_result,
                Err(_) => panic!("Thread sender was prematurely dropped!"),
            };

            FFIFuture {
                class,
                page,
                result: rec_result,
                worker_id: id,
            }
        }
    }

    pub fn extract(
        &self,
        class: KnownObject,
        shared: ffi::Shared,
        page: Page,
    ) -> impl Future<Output = ExtractionFuturePayload> + 'static {
        let (sender, receiver) = tokio_oneshot::channel::<ExtractionResult>();
        let worker_sender = self.sender.clone();
        let id = self.id.clone();

        async move {
            worker_sender
                .send(WorkerJob::Extract {
                    class,
                    shared,
                    responder: sender,
                    page,
                })
                .await.expect(&format!(
                    "Thread error while attempting to resolve extraction future for page {} with class {}",
                    class.to_string(),
                    page.num
                ));

            let result = match receiver.await {
                Ok(res) => res,
                Err(_) => panic!("Thread sender was prematurely dropped!"), // tokio::sync::oneshot::error::RecvError ensures that this is the only error case.
            };

            FFIFuture {
                class,
                page,
                result,
                worker_id: id,
            }
        }
    }

    fn _loop(state: WorkerState, mut receiver: Receiver<WorkerJob>, working_flag: Arc<AtomicBool>) {
        while let Some(job) = receiver.blocking_recv() {
            working_flag.store(true, Ordering::Release);

            match job {
                WorkerJob::Classify {
                    class,
                    responder,
                    page,
                } => Self::handle_classify(&state, responder, class, page),
                WorkerJob::Extract {
                    class,
                    shared,
                    responder,
                    page,
                } => Self::handle_extract(&state, responder, shared, class, page),
            }
        }
    }

    fn handle_classify(
        state: &WorkerState,
        responder: ClassificationJobResponder, // sender
        class: KnownObject,
        page: Page,
    ) -> () {
        let call = unsafe { ffi::classify(&state.ctx, &state.doc, class, page.into()) };

        let packet = responder.send(call);

        match packet {
            Ok(_) => (),
            Err(_) => panic!("Thread reciever prematurely dropped!"),
        }
    }

    fn handle_extract(
        state: &WorkerState,
        responder: ExtractionJobResponder,
        shared: ffi::Shared,
        class: KnownObject,
        page: Page,
    ) -> () {
        let call = unsafe { ffi::extract(&state.ctx, &state.doc, &shared, class, page.into()) };

        let packet = responder.send(call);

        match packet {
            Ok(_) => (),
            Err(_) => panic!("Thread reciever prematurely dropped!"),
        }
    }
}
