#![allow(dead_code)]

use crate::generated::generated_object_types::KnownObject;
use cxx::{UniquePtr, let_cxx_string};
use std::marker::PhantomData;
use std::ops::Deref;
use std::path::PathBuf;

/// 256MiB, standard for PyMuPDF and other bindings
pub const STANDARD_CTX_MEM_LIMIT: usize = 256 << 20;

/// Currently, there is no implementation to pass extracted data from the C++ side (via [extract]).
/// Into anywhere else, my current thought process behind it is to instead pass it to Python
/// into some form of a key-table structure (i.e JSON), but I may also just simply write that data into
/// A file and deal with it there.
/// Although, most likely this will incorporate some dynamic form of data that can be configured
/// To appeal to the dynamic aspect of this project.  
pub type ExtractResult = UserResult<()>;

#[cxx::bridge]
#[allow(unused)]
mod bridge {

    unsafe extern "C++" {
        include!("initializer.h");

        type OpaqueCtx;
        type OpaqueDoc;
        type OpaqueResult;
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
        ) -> Result<UniquePtr<OpaqueResult>>;

        fn call_extract(
            o_ctx: &UniquePtr<OpaqueCtx>,
            o_doc: &UniquePtr<OpaqueDoc>,
            shared: &UniquePtr<SharedData>,
            obj: &CxxString,
        ) -> Result<UniquePtr<OpaqueResult>>;

        fn extract_shared_payload(r: &UniquePtr<OpaqueResult>) -> Result<UniquePtr<SharedData>>;
        fn extract_error_result(r: &UniquePtr<OpaqueResult>) -> Result<&CxxString>;
        fn get_result_status(r: &UniquePtr<OpaqueResult>) -> i32;

        fn drop_ctx(o_ctx: &UniquePtr<OpaqueCtx>) -> ();
        fn drop_doc(o_ctx: &UniquePtr<OpaqueCtx>, o_doc: &UniquePtr<OpaqueDoc>) -> ();
        fn drop_result(f: &UniquePtr<OpaqueResult>) -> ();
    }
}

/// Where T is just a marker to indicate what payload is on Ok
pub struct OkUserResult<T> {
    inner: UniquePtr<bridge::OpaqueResult>,
    _data: PhantomData<T>,
}

pub struct FailUserResult {
    inner: UniquePtr<bridge::OpaqueResult>,
}

pub enum UserResult<T> {
    Ok(OkUserResult<T>),
    Fail(FailUserResult),
}

/// SAFETY:
/// UserResult is meant to be sent from one thread to another, BUT not shared between threads
/// For example:
///     Worker runs classify
///     Worker **sends** UserResult back to mainthread for evaluation (from classify call)
///     Main thread now has ownership
///
/// UserResult is meant to not be synced between threads while its purpose is that example
unsafe impl<T> Send for UserResult<T> {}

impl<T> Drop for UserResult<T> {
    fn drop(&mut self) {
        // SAFETY: Rust always holds ownership over any UserResult
        // For example, if created in classify the Rust layer is returned said result
        // Therefore, Rust holds the ownership of UserResult aslong as users don't hold
        // unexpected pointers/references to UserResult
        match self {
            Self::Ok(inner) => bridge::drop_result(&inner.inner),
            Self::Fail(inner) => bridge::drop_result(&inner.inner),
        }
    }
}

impl<T> Deref for OkUserResult<T> {
    type Target = UniquePtr<bridge::OpaqueResult>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> OkUserResult<T> {
    pub fn extract_payload_as_shared(&self) -> Shared {
        let raw_shared = bridge::extract_shared_payload(&self)
            .expect("Attempted to access payload on a FAIL result.");
        Shared(raw_shared)
    }
}

impl FailUserResult {
    pub fn extract_fail_rsn(&self) -> &str {
        bridge::extract_error_result(&self.inner)
            .expect("Attempted to access failure reason on a OK result.")
            .to_str()
            .expect("Failure reason wasn't valid UTF-8 **this should have never happened, FFI returns a std::string!**")
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

impl Drop for Context {
    fn drop(&mut self) {
        bridge::drop_ctx(&self.0);
    }
}
pub struct Document<'ctx> {
    inner: UniquePtr<bridge::OpaqueDoc>,
    _ctx: &'ctx Context,
}

impl<'ctx> Drop for Document<'ctx> {
    fn drop(&mut self) {
        bridge::drop_doc(self._ctx, &self.inner);
    }
}

impl<'ctx> Deref for Document<'ctx> {
    type Target = UniquePtr<bridge::OpaqueDoc>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'ctx> Document<'ctx> {
    pub unsafe fn new(path: PathBuf, ctx: &'ctx Context) -> Self {
        let_cxx_string!(cxx_path = path.to_string_lossy().to_string());

        Self {
            inner: bridge::create_new_doc(ctx, &cxx_path).unwrap_or_else(|e| {
                panic!("Failed to create document (from binding: {})", e.what())
            }),
            _ctx: ctx,
        }
    }
}

pub struct Shared(pub UniquePtr<bridge::SharedData>);

/// SAFETY:
///     Shared is never mutated within the actual classifier layer
///     Shared is simply passed between threads to its final state
///     being the call to [extract] for said object
///     where then it would then be dropped
unsafe impl Send for Shared {}

impl Deref for Shared {
    type Target = UniquePtr<bridge::SharedData>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub unsafe fn classify(ctx: &Context, doc: &Document, ident: KnownObject) -> UserResult<Shared> {
    let_cxx_string!(ident_to_cxx_str = ident.to_string());

    let call = bridge::call_classify(ctx, doc, &ident_to_cxx_str)
        .unwrap_or_else(|e| panic!("Failed to call classify! (from intermediary: {})", e.what()));

    assert!(
        !call.is_null(),
        "Failed to call classify! returned null data!"
    );

    let status = bridge::get_result_status(&call);

    if status == 0 {
        UserResult::Ok(OkUserResult {
            inner: call,
            _data: PhantomData::default(),
        })
    } else {
        UserResult::Fail(FailUserResult { inner: call })
    }
}

pub unsafe fn extract(
    ctx: &Context,
    doc: &Document,
    shared: &Shared,
    ident: KnownObject,
) -> ExtractResult /* placeholder */ {
    let_cxx_string!(ident_to_cxx_str = ident.to_string());

    let call = bridge::call_extract(ctx, doc, shared, &ident_to_cxx_str)
        .unwrap_or_else(|e| panic!("Failed to call extract! (from intermediary: {})", e.what()));

    assert!(
        !call.is_null(),
        "Failed to call extract! returned null data!"
    );

    let status = bridge::get_result_status(&call);

    if status == 0 {
        UserResult::Ok(OkUserResult {
            inner: call,
            _data: PhantomData::default(),
        })
    } else {
        UserResult::Fail(FailUserResult { inner: call })
    }
}
