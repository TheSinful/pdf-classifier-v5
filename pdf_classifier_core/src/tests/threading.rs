use crate::constants::test_constants::FIRST_CHAPTER_PAGE;
use crate::ffi::UserResult;
use crate::generated::generated_object_types::KnownObject;
use crate::tests::init::get_LARGE_TEST_DOC_test_path;
use crate::threading::*;

#[tokio::test]
async fn test_bulk_worker_spawn() {
    WorkerThread::spawn(get_LARGE_TEST_DOC_test_path(), 1);
    WorkerThread::spawn(get_LARGE_TEST_DOC_test_path(), 2);
    WorkerThread::spawn(get_LARGE_TEST_DOC_test_path(), 3);
    WorkerThread::spawn(get_LARGE_TEST_DOC_test_path(), 4);
}

#[tokio::test]
async fn test_worker_classify_call() {
    let worker = WorkerThread::spawn(get_LARGE_TEST_DOC_test_path(), 1);
    let fut = worker
        .classify(KnownObject::CHAPTER, FIRST_CHAPTER_PAGE.into())
        .await;

    match &fut.result {
        UserResult::Ok(ok_user_result) => {
            assert!(
                !ok_user_result.extract_payload_as_shared().is_null(),
                "Extracted payload was null!"
            )
        }
        UserResult::Fail(f) => {
            panic!(
                "Worker classification failed with error{}",
                f.extract_fail_rsn()
            )
        }
    }
}

#[tokio::test]
async fn test_worker_extract_call() {
    let worker = WorkerThread::spawn(get_LARGE_TEST_DOC_test_path(), 1);
    let classify_result = worker
        .classify(KnownObject::CHAPTER, FIRST_CHAPTER_PAGE.into())
        .await; // from examples

    let classify_unwrap = classify_result.result;
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
            0u32.into(),
        )
        .await
        .result;

    match extract_result {
        UserResult::Ok(_) => {}
        UserResult::Fail(f) => {
            panic!(
                "Failed to run extraction on thread! {}",
                f.extract_fail_rsn()
            )
        }
    }
}
