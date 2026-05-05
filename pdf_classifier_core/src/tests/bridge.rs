// ! To run this, run main.py within examples to generate generated modules.

use crate::constants::test_constants::FIRST_CHAPTER_PAGE;
use crate::ffi::*;
use crate::generated::generated_object_types::KnownObject;
use crate::tests::init::get_LARGE_TEST_DOC_test_path;

#[test]
fn create_ctx() {
    let ctx = unsafe { FzContext::new(STANDARD_CTX_MEM_LIMIT) };
    assert!(!ctx.0.is_null())
}

#[test]
fn test_classify_call() {
    let ctx = unsafe { FzContext::new(STANDARD_CTX_MEM_LIMIT) };
    assert!(!ctx.0.is_null());

    let doc = unsafe { Document::new(get_LARGE_TEST_DOC_test_path(), &ctx) };
    assert!(!doc.is_null());

    let call = unsafe { classify(&ctx, &doc, KnownObject::CHAPTER, FIRST_CHAPTER_PAGE) };
    match &call {
        UserResult::Ok(ok) => assert!(!ok.is_null()),
        UserResult::Fail(f) => panic!("Classify failed with err {}!", f.extract_fail_rsn()),
    }
}

#[test]
fn test_extract_call() {
    let ctx = unsafe { FzContext::new(STANDARD_CTX_MEM_LIMIT) };
    assert!(!ctx.is_null());

    let doc = unsafe { Document::new(get_LARGE_TEST_DOC_test_path(), &ctx) };
    assert!(!doc.is_null());

    let classify = unsafe { classify(&ctx, &doc, KnownObject::CHAPTER, FIRST_CHAPTER_PAGE) };
    match &classify {
        UserResult::Ok(ok) => {
            assert!(!ok.is_null());

            unsafe {
                let shared = ok.extract_payload_as_shared();
                extract(
                    &ctx,
                    &doc,
                    &shared,
                    KnownObject::CHAPTER,
                    FIRST_CHAPTER_PAGE,
                )
            };
        }
        UserResult::Fail(f) => panic!("Extraction failed with err {}!", f.extract_fail_rsn()),
    }
}
