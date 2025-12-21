use crate::generated::generated_object_types::KnownObject;
use crate::initializer::*;
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
    assert!(!doc.0.is_null());

    let call = unsafe { classify(&ctx, &doc, KnownObject::CHAPTER) };
    assert!(!call.0.is_null());
}

#[test]
fn test_extract_cal() {
    let ctx = unsafe { Context::new(STANDARD_CTX_MEM_LIMIT) };
    assert!(!ctx.0.is_null());

    let test_doc_path = std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/data/small_test_doc.pdf";
    let doc = unsafe { Document::new(PathBuf::from(test_doc_path), &ctx) };
    assert!(!doc.0.is_null());

    let classify = unsafe { classify(&ctx, &doc, KnownObject::CHAPTER) };
    assert!(!classify.0.is_null());

    unsafe { extract(&ctx, &doc, &classify, KnownObject::CHAPTER) };
}
