use crate::generated::generated_object_types::KnownObject;
use cxx::{UniquePtr, let_cxx_string};
use std::ops::Deref;
use std::path::PathBuf;

/// 256MiB, standard for PyMuPDF and other bindings
pub const STANDARD_CTX_MEM_LIMIT: usize = 256 << 20;

#[cxx::bridge]
#[allow(unused)]
mod bridge {

    unsafe extern "C++" {
        include!("initializer.h");

        type OpaqueCtx;
        type OpaqueDoc;
        type SharedData;

        fn create_new_ctx(mem_limit: usize) -> Result<UniquePtr<OpaqueCtx>>;
        fn create_new_doc(
            o_ctx: &UniquePtr<OpaqueCtx>,
            path: &CxxString,
        ) -> Result<UniquePtr<OpaqueDoc>>;
        fn call_classify(
            o_ctx: &UniquePtr<OpaqueCtx>,
            o_doc: &UniquePtr<OpaqueDoc>,
            obj: &CxxString,
        ) -> Result<UniquePtr<SharedData>>;
        fn call_extract(
            o_ctx: &UniquePtr<OpaqueCtx>,
            o_doc: &UniquePtr<OpaqueDoc>,
            shared: &UniquePtr<SharedData>,
            obj: &CxxString,
        ) -> Result<()>;

    }
}

pub struct Context(pub UniquePtr<bridge::OpaqueCtx>);

impl Context {
    pub unsafe fn new(mem_limit: usize) -> Self {
        Self {
            0: bridge::create_new_ctx(mem_limit).unwrap_or_else(|e| {
                panic!("Failed to create context (from binding: {})", e.what())
            }),
        }
    }
}

impl Deref for Context {
    type Target = UniquePtr<bridge::OpaqueCtx>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct Document(pub UniquePtr<bridge::OpaqueDoc>);

impl Deref for Document {
    type Target = UniquePtr<bridge::OpaqueDoc>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Document {
    pub unsafe fn new(path: PathBuf, ctx: &Context) -> Self {
        let_cxx_string!(cxx_path = path.to_string_lossy().to_string());

        Self {
            0: bridge::create_new_doc(ctx, &cxx_path).unwrap_or_else(|e| {
                panic!("Failed to create document (from binding: {})", e.what())
            }),
        }
    }
}

pub struct Shared(pub UniquePtr<bridge::SharedData>);

impl Deref for Shared {
    type Target = UniquePtr<bridge::SharedData>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub unsafe fn classify(ctx: &Context, doc: &Document, ident: KnownObject) -> Shared {
    let_cxx_string!(ident_to_cxx_str = ident.to_string());

    let call = bridge::call_classify(ctx, doc, &ident_to_cxx_str)
        .unwrap_or_else(|e| panic!("Failed to call classify! (from intermediary: {})", e.what()));

    assert!(
        !call.is_null(),
        "Failed to call classify! returned null data!"
    );

    Shared(call)
}

pub unsafe fn extract(ctx: &Context, doc: &Document, shared: &Shared, ident: KnownObject) -> () {
    let_cxx_string!(ident_to_cxx_str = ident.to_string());

    bridge::call_extract(ctx, doc, shared, &ident_to_cxx_str)
        .unwrap_or_else(|e| panic!("Failed to call extract! (from intermediary: {})", e.what()));
}
