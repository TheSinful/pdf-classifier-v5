use crate::{
    ffi::{self, UserResult},
    generated::generated_object_types::KnownObject,
};
use cxx::Exception;
use std::{
    path::PathBuf,
    thread::{self, Builder, JoinHandle},
};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::oneshot as tokio_oneshot;

/// The number of items tokio pre-allocates for within Sender<WorkerJob>
/// sizeof(CHANNEL_BUFFER_SIZE) = CHANNEL_BUFFER_SIZE * sizeof(WorkerJob)
pub const CHANNEL_BUFFER_SIZE: usize = 50;

/// Currently just returns a Result<Shared, ()> because we don't return any information of failure
/// Later, this should be like: Result<Shared, String> where String is the failure reason
type ClassificationJobResponder = tokio_oneshot::Sender<ClassificationResult>;
type ExtractionJobResponder = tokio_oneshot::Sender<ExtractionResult>;
type ClassificationResult = UserResult<ffi::Shared>;
type ExtractionResult = Result<UserResult<()>, Exception>; // payload is irrelevent

enum WorkerJob {
    Classify {
        ident: KnownObject,
        responder: ClassificationJobResponder,
    },
    Extract {
        ident: KnownObject,
        shared: ffi::Shared,
        responder: ExtractionJobResponder,
    },
}

struct WorkerThread {
    sender: Sender<WorkerJob>,
}

struct WorkerState<'a> {
    pub ctx: &'a ffi::fz_context,
    pub doc: ffi::fz_document<'a>,
}

impl WorkerThread {
    pub fn spawn(&self, doc_path: PathBuf) -> Self {
        let (sender, mut receiver) = tokio::sync::mpsc::channel::<WorkerJob>(CHANNEL_BUFFER_SIZE);

        thread::spawn(move || {
            /// SAFETY: nothing is inherently unsafe about context or document creation as both
            /// have the lifetiem of the thread that utilizes them.
            /// this should be later reflected in the actual definition of fz_context and fz_document.
            /// And since context is declared prior to document, document should always Drop first,
            /// as fz_drop_document() requires a ctx pointer.
            let ctx = unsafe { ffi::fz_context::new(ffi::STANDARD_CTX_MEM_LIMIT) };
            let doc = unsafe { ffi::fz_document::new(doc_path, &ctx) };
            let state = WorkerState { ctx: &ctx, doc };

            Self::_loop(state, receiver);
        });

        Self { sender }
    }

    pub async fn classify(&self, ident: KnownObject) -> ClassificationResult {
        let (sender, receiver) = tokio_oneshot::channel::<ClassificationResult>();

        self.sender.send(WorkerJob::Classify {
            ident,
            responder: sender,
        });

        match receiver.await {
            Ok(res) => res,
            Err(_) => panic!("Thread sender was prematurely dropped!"),
        }
    }

    pub async fn extract(&self, ident: KnownObject, shared: ffi::Shared) -> ExtractionResult {
        let (sender, receiver) = tokio_oneshot::channel::<ExtractionResult>();

        self.sender.send(WorkerJob::Extract {
            ident,
            shared,
            responder: sender,
        });

        match receiver.await {
            Ok(res) => res,
            Err(_) => panic!("Thread sender was prematurely dropped!"),
        }
    }

    fn _loop(state: WorkerState, mut receiver: Receiver<WorkerJob>) {
        while let Some(job) = receiver.blocking_recv() {
            match job {
                WorkerJob::Classify { ident, responder } => unsafe {
                    Self::handle_classify(&state, responder, ident);
                },
                WorkerJob::Extract {
                    ident,
                    shared,
                    responder,
                } => Self::handle_extract(&state, responder, shared, ident),
            }
        }
    }

    fn handle_classify(
        state: &WorkerState,
        responder: ClassificationJobResponder, // sender
        object_ident: KnownObject,
    ) -> () {
        let call = unsafe { ffi::classify(&state.ctx, &state.doc, object_ident) };

        responder.send(call);
    }

    fn handle_extract(
        state: &WorkerState,
        responder: ExtractionJobResponder,
        shared: ffi::Shared,
        ident: KnownObject,
    ) -> () {
        let call = unsafe { ffi::extract(&state.ctx, &state.doc, &shared, ident) };
    }
}
