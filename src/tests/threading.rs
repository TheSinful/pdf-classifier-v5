use crate::threading::*;
use std::path::PathBuf;

fn get_test_doc_path() -> PathBuf {
    let root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    PathBuf::from(root).join("data/small_test_doc.pdf")
}

#[tokio::test]
async fn test_bulk_worker_spawn() {
    WorkerThread::spawn(get_test_doc_path());
    WorkerThread::spawn(get_test_doc_path());
    WorkerThread::spawn(get_test_doc_path());
    WorkerThread::spawn(get_test_doc_path());
}

#[tokio::test]
async fn test_worker_classify_call() {
    let worker = WorkerThread::spawn(get_test_doc_path());
    let result = worker
        .classify(crate::generated::generated_object_types::KnownObject::CHAPTER)
        .await; // from examples
    match result {
        Ok(_) => {}
        Err(e) => panic!("Failed to run classify on thread! {}", e.to_string()),
    }

    let unwrap = result.unwrap();
    match &unwrap {
        crate::ffi::UserResult::Ok(ok_user_result) => {
            assert!(
                !ok_user_result.extract_payload_as_shared().is_null(),
                "Extracted payload was null!"
            )
        }
        crate::ffi::UserResult::Fail(_) => {
            panic!("Returned unexpected UserResult, example should've returned an Ok")
        }
    }
}

#[tokio::test]
async fn test_worker_extract_call() {
    let worker = WorkerThread::spawn(get_test_doc_path());
    let classify_result = worker
        .classify(crate::generated::generated_object_types::KnownObject::CHAPTER)
        .await; // from examples
    match classify_result {
        Ok(_) => {}
        Err(e) => panic!("Failed to run classify on thread! {}", e.to_string()),
    }

    let classify_unwrap = classify_result.unwrap();
    let shared = match &classify_unwrap {
        crate::ffi::UserResult::Ok(ok_user_result) => {
            let unwrap = ok_user_result.extract_payload_as_shared();

            assert!(!unwrap.is_null(), "Extracted payload was null!");
            unwrap
        }
        crate::ffi::UserResult::Fail(_) => {
            panic!("Returned unexpected UserResult, example should've returned an Ok")
        }
    };

    let extract_result = worker
        .extract(
            crate::generated::generated_object_types::KnownObject::CHAPTER,
            shared,
        )
        .await;

    match extract_result {
        Ok(_) => {}
        Err(e) => panic!("Failed to run classify on thread! {}", e.to_string()),
    }

    extract_result.unwrap();
    // ! extract returns NULL because it has no outputted data currently
    // ! so there is no point to validating its outputted data.
}
