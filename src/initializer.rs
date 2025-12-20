use cxx::{UniquePtr, let_cxx_string};
use std::path::PathBuf;

/// 256MiB, standard for PyMuPDF and other bindings 
pub const STANDARD_MEM_LIMIT: usize = 256 << 20;

#[cxx::bridge]
#[allow(unused)]
mod bridge {

    unsafe extern "C++" {
        include!("initializer.h");

        type OpaqueCtx;
        type OpaqueDoc;
        type SharedData;

        fn create_new_ctx(mem_limit: usize) -> UniquePtr<OpaqueCtx>;
        fn create_new_doc(path: &CxxString, o_ctx: &OpaqueCtx) -> UniquePtr<OpaqueDoc>;
        fn call_classify(
            o_ctx: &OpaqueCtx,
            o_doc: &OpaqueDoc,
            obj: &CxxString,
        ) -> UniquePtr<SharedData>;
        fn call_extract(
            o_ctx: &OpaqueCtx,
            o_doc: &OpaqueDoc,
            shared: &SharedData,
            obj: &CxxString,
        ) -> ();

    }
}

pub struct Context(pub UniquePtr<bridge::OpaqueCtx>);

impl Context {
    pub unsafe fn new(mem_limit: usize) -> Self {
        Self {
            0: bridge::create_new_ctx(mem_limit),
        }
    }
}

pub struct Document(pub UniquePtr<bridge::OpaqueDoc>);

impl Document {
    pub unsafe fn new(path: PathBuf, ctx: &Context) -> Self {
        let_cxx_string!(cxx_path = path.to_string_lossy().to_string());

        Self {
            0: bridge::create_new_doc(&cxx_path, &ctx.0),
        }
    }
}

pub struct Shared(pub UniquePtr<bridge::SharedData>);
