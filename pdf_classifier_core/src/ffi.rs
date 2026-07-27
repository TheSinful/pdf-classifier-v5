use crate::generated::generated_object_types::KnownObject;
use cxx::{CxxString, UniquePtr, let_cxx_string};
use std::fmt::Debug;
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
pub type ExtractionResult = UserResult<CxxString>;
pub type ClassificationResult = UserResult<Shared>;

#[cxx::bridge]
mod bridge {

    unsafe extern "C++" {
        include!("ffi.hpp");

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
            page: u32,
        ) -> Result<UniquePtr<OpaqueResult>>;

        fn call_extract(
            o_ctx: &UniquePtr<OpaqueCtx>,
            o_doc: &UniquePtr<OpaqueDoc>,
            shared: &UniquePtr<SharedData>,
            obj: &CxxString,
            page: u32,
        ) -> Result<UniquePtr<OpaqueResult>>;

        fn extract_shared_payload(r: &UniquePtr<OpaqueResult>) -> Result<UniquePtr<SharedData>>;
        fn extract_error_result(r: &UniquePtr<OpaqueResult>) -> Result<&CxxString>;
        fn get_result_status(r: &UniquePtr<OpaqueResult>) -> i32;
        fn extract_string_payload(r: &UniquePtr<OpaqueResult>) -> Result<&CxxString>;

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

impl<T> Debug for OkUserResult<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OkUserResult")
            .field("inner", &format_args!("{:p}", &self.inner as *const _))
            .field("_data", &self._data)
            .finish()
    }
}

pub struct FailUserResult {
    inner: UniquePtr<bridge::OpaqueResult>,
}

impl Debug for FailUserResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FailUserResult")
            .field("err", &self.extract_fail_rsn())
            .finish()
    }
}

#[derive(Debug)]
pub enum UserResult<T> {
    Ok(OkUserResult<T>),
    Fail(FailUserResult),
}

impl<T> UserResult<T> {
    pub fn is_ok(&self) -> bool {
        match self {
            UserResult::Ok(_) => true,
            UserResult::Fail(_) => false,
        }
    }

    #[expect(dead_code, reason = "used within unit tests")]
    pub fn err(self) -> Option<FailUserResult> {
        match self {
            UserResult::Ok(_) => None,
            UserResult::Fail(fail_user_result) => Some(fail_user_result),
        }
    }

    #[expect(dead_code, reason = "used to propogate error reason to the front-end")]
    pub fn fail_rsn(&self) -> Option<String> {
        match self {
            UserResult::Ok(_) => None,
            UserResult::Fail(fail_user_result) => {
                Some(fail_user_result.extract_fail_rsn().to_string())
            }
        }
    }
}

impl UserResult<CxxString> {
    pub fn extract_inner_as_string(&self) -> Option<String> {
        match self {
            UserResult::Ok(ok) => Some(ok.extract_payload_as_string()),
            UserResult::Fail(_) => None,
        }
    }
}

impl<T> From<OkUserResult<T>> for UserResult<T> {
    fn from(value: OkUserResult<T>) -> Self {
        Self::Ok(value)
    }
}

impl<T> From<FailUserResult> for UserResult<T> {
    fn from(value: FailUserResult) -> Self {
        Self::Fail(value)
    }
}

/// SAFETY:
/// UserResult is meant to be sent from one thread to another, BUT not shared between threads
/// For example:
///     Worker runs classify
///     Worker **sends** UserResult back to mainthread for evaluation (from classify call)
///     Main thread now has ownership
///
/// UserResult is intentionally not Sync as explained by the example above.
unsafe impl<T> Send for UserResult<T> {}

impl<T> Drop for OkUserResult<T> {
    fn drop(&mut self) {
        bridge::drop_result(&self.inner);
    }
}

impl Drop for FailUserResult {
    fn drop(&mut self) {
        bridge::drop_result(&self.inner);
    }
}

impl<T> Deref for OkUserResult<T> {
    type Target = UniquePtr<bridge::OpaqueResult>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl OkUserResult<Shared> {
    pub fn extract_payload_as_shared(&self) -> Shared {
        let raw_shared = bridge::extract_shared_payload(&self)
            .expect("Attempted to access payload on a FAIL result.");
        Shared(raw_shared)
    }
}

impl OkUserResult<CxxString> {
    pub fn extract_payload_as_string(&self) -> String {
        match bridge::extract_string_payload(&self.inner) {
            Ok(_str) => return _str.to_string_lossy().to_string(),
            Err(e) => panic!("Failed to extract string payload {}", e.what()),
        }
    }
}

impl FailUserResult {
    pub fn extract_fail_rsn(&self) -> &str {
        match bridge::extract_error_result(&self.inner) {
            Ok(_str) => _str
                .to_str()
                .expect("Error result was not in UTF-8 format!"),
            Err(e) => panic!("Failed to extract failure reason: {}", e.what()),
        }
    }
}

pub struct FzContext(pub UniquePtr<bridge::OpaqueCtx>);

impl FzContext {
    pub unsafe fn new(mem_limit: usize) -> Self {
        Self {
            0: bridge::create_new_ctx(mem_limit).unwrap_or_else(|e| {
                panic!("Failed to create context (from binding: {})", e.what())
            }),
        }
    }
}

impl Deref for FzContext {
    type Target = UniquePtr<bridge::OpaqueCtx>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Drop for FzContext {
    fn drop(&mut self) {
        bridge::drop_ctx(&self.0);
    }
}
pub struct Document<'ctx> {
    inner: UniquePtr<bridge::OpaqueDoc>,
    _ctx: &'ctx FzContext,
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
    pub unsafe fn new(path: PathBuf, ctx: &'ctx FzContext) -> Self {
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

pub unsafe fn classify(
    ctx: &FzContext,
    doc: &Document,
    ident: KnownObject,
    page: u32,
) -> ClassificationResult {
    let_cxx_string!(ident_to_cxx_str = ident.to_string());

    let call = bridge::call_classify(ctx, doc, &ident_to_cxx_str, page)
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
    ctx: &FzContext,
    doc: &Document,
    shared: &Shared,
    ident: KnownObject,
    page: u32,
) -> ExtractionResult {
    let_cxx_string!(ident_to_cxx_str = ident.to_string());

    let call = bridge::call_extract(ctx, doc, shared, &ident_to_cxx_str, page)
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
