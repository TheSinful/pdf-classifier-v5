// ! To run this, run main.py within examples to generate generated modules.

use super::init::get_SMALL_TEST_DOC_test_path;
use crate::ffi::*;
use crate::generated::generated_object_types::KnownObject;

#[test]
fn create_ctx() {
    let ctx = unsafe { FzContext::new(STANDARD_CTX_MEM_LIMIT) };
    assert!(!ctx.0.is_null())
}

#[test]
fn test_classify_call() {
    let ctx = unsafe { FzContext::new(STANDARD_CTX_MEM_LIMIT) };
    assert!(!ctx.0.is_null());

    let doc = unsafe { Document::new(get_SMALL_TEST_DOC_test_path(), &ctx) };
    assert!(!doc.is_null());

    let call = unsafe { classify(&ctx, &doc, KnownObject::CHAPTER, 0) };
    match &call {
        UserResult::Ok(ok) => assert!(!ok.is_null()),
        UserResult::Fail(_) => panic!("Shouldn't have failed!"),
    }
}

#[test]
fn test_extract_call() {
    let ctx = unsafe { FzContext::new(STANDARD_CTX_MEM_LIMIT) };
    assert!(!ctx.is_null());

    let doc = unsafe { Document::new(get_SMALL_TEST_DOC_test_path(), &ctx) };
    assert!(!doc.is_null());

    let classify = unsafe { classify(&ctx, &doc, KnownObject::CHAPTER, 0) };
    match &classify {
        UserResult::Ok(ok) => {
            assert!(!ok.is_null());

            unsafe {
                let shared = ok.extract_payload_as_shared();
                extract(&ctx, &doc, &shared, KnownObject::CHAPTER, 0)
            };
        }
        UserResult::Fail(_) => panic!("Shouldn't have failed!"),
    }
}
