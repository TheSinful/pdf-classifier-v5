#![allow(dead_code)]

use crate::{
    ffi::{self, ClassificationResult, ExtractionResult},
    generated::generated_object_types::KnownObject,
    page::Page,
};
use std::{path::PathBuf, thread};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::oneshot as tokio_oneshot;

/// The number of items tokio pre-allocates for within Sender<WorkerJob>
/// sizeof(CHANNEL_BUFFER_SIZE) = CHANNEL_BUFFER_SIZE * sizeof(WorkerJob)
pub const CHANNEL_BUFFER_SIZE: usize = 50;

/// Currently just returns a Result<Shared, ()> because we don't return any information of failure
/// Later, this should be like: Result<Shared, String> where String is the failure reason
type ClassificationJobResponder = tokio_oneshot::Sender<ClassificationResult>;
type ExtractionJobResponder = tokio_oneshot::Sender<ExtractionResult>;

pub enum WorkerJob {
    Classify {
        ident: KnownObject,
        page: Page,
        responder: ClassificationJobResponder,
    },
    Extract {
        ident: KnownObject,
        page: Page,
        shared: ffi::Shared,
        responder: ExtractionJobResponder,
    },
}

pub struct WorkerThread {
    sender: Sender<WorkerJob>,
}

/// Stored within the actual spawned thread,
struct WorkerState<'a> {
    pub ctx: &'a ffi::Context,
    pub doc: ffi::Document<'a>,
}

#[derive(thiserror::Error, Debug)]
pub enum ThreadError {
    #[error("Failed to send job to thread!")]
    SendError(#[from] tokio::sync::mpsc::error::SendError<WorkerJob>),
}

impl WorkerThread {
    pub fn spawn(doc_path: PathBuf) -> Self {
        let (sender, receiver) = tokio::sync::mpsc::channel::<WorkerJob>(CHANNEL_BUFFER_SIZE);

        thread::spawn(move || {
            // SAFETY: nothing is inherently unsafe about context or document creation as both
            // have the lifetiem of the thread that utilizes them.
            // this should be later reflected in the actual definition of fz_context and fz_document.
            // And since context is declared prior to document, document should always Drop first,
            // as fz_drop_document() requires a ctx pointer.
            let ctx = unsafe { ffi::Context::new(ffi::STANDARD_CTX_MEM_LIMIT) };
            let doc = unsafe { ffi::Document::new(doc_path, &ctx) };
            let state = WorkerState { ctx: &ctx, doc };

            Self::_loop(state, receiver);
        });

        Self { sender }
    }

    pub async fn classify(
        &self,
        ident: KnownObject,
        page: Page,
    ) -> Result<ClassificationResult, ThreadError> {
        let (sender, receiver) = tokio_oneshot::channel::<ClassificationResult>();

        self.sender
            .send(WorkerJob::Classify {
                ident,
                page,
                responder: sender,
            })
            .await?;

        match receiver.await {
            Ok(res) => Ok(res),
            Err(_) => panic!("Thread sender was prematurely dropped!"),
        }
    }

    pub async fn extract(
        &self,
        ident: KnownObject,
        shared: ffi::Shared,
        page: Page,
    ) -> Result<ExtractionResult, ThreadError> {
        let (sender, receiver) = tokio_oneshot::channel::<ExtractionResult>();

        self.sender
            .send(WorkerJob::Extract {
                ident,
                shared,
                responder: sender,
                page,
            })
            .await?;

        match receiver.await {
            Ok(res) => Ok(res),
            Err(_) => panic!("Thread sender was prematurely dropped!"), // tokio::sync::oneshot::error::RecvError ensures that this is the only error case.
        }
    }

    fn _loop(state: WorkerState, mut receiver: Receiver<WorkerJob>) {
        while let Some(job) = receiver.blocking_recv() {
            match job {
                WorkerJob::Classify {
                    ident,
                    responder,
                    page,
                } => Self::handle_classify(&state, responder, ident, page),
                WorkerJob::Extract {
                    ident,
                    shared,
                    responder,
                    page,
                } => Self::handle_extract(&state, responder, shared, ident, page),
            }
        }
    }

    fn handle_classify(
        state: &WorkerState,
        responder: ClassificationJobResponder, // sender
        object_ident: KnownObject,
        page: Page,
    ) -> () {
        let call = unsafe { ffi::classify(&state.ctx, &state.doc, object_ident, page.into()) };

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
        ident: KnownObject,
        page: Page,
    ) -> () {
        let call = unsafe { ffi::extract(&state.ctx, &state.doc, &shared, ident, page.into()) };

        let packet = responder.send(call);

        match packet {
            Ok(_) => (),
            Err(_) => panic!("Thread reciever prematurely dropped!"),
        }
    }
}
