// ! To run this, run main.py within examples to generate generated modules.

use crate::ffi::*;
use crate::generated::generated_object_types::KnownObject;
use std::path::PathBuf;


#[test]
fn create_ctx() {
    let ctx = unsafe { Context::new(STANDARD_CTX_MEM_LIMIT) };
    assert!(!ctx.0.is_null())
}

#[test]
fn test_classify_call() {
    let ctx = unsafe { Context::new(STANDARD_CTX_MEM_LIMIT) };
    assert!(!ctx.0.is_null());

    let test_doc_path = std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/data/small_test_doc.pdf";
    let doc = unsafe { Document::new(PathBuf::from(test_doc_path), &ctx) };
    assert!(!doc.is_null());

    let call = unsafe { classify(&ctx, &doc, KnownObject::CHAPTER) };
    match &call {
        UserResult::Ok(ok) => assert!(!ok.is_null()),
        UserResult::Fail(_) => panic!("Shouldn't have failed!"),
    }
}

#[test]
fn test_extract_call() {
    let ctx = unsafe { Context::new(STANDARD_CTX_MEM_LIMIT) };
    assert!(!ctx.is_null());

    let test_doc_path = std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/data/small_test_doc.pdf";
    let doc = unsafe { Document::new(PathBuf::from(test_doc_path), &ctx) };
    assert!(!doc.is_null());

    let classify = unsafe { classify(&ctx, &doc, KnownObject::CHAPTER) };
    match &classify {
        UserResult::Ok(ok) => {
            assert!(!ok.is_null());

            unsafe {
                let shared = ok.extract_payload_as_shared();
                extract(&ctx, &doc, &shared, KnownObject::CHAPTER)
            };
        }
        UserResult::Fail(_) => panic!("Shouldn't have failed!"),
    }
}
