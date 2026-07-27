use crate::constants::test_constants::FIRST_CHAPTER_PAGE;
use crate::generated::generated_object_types::KnownObject;
use crate::page::Page;
use crate::tests::init::get_LARGE_TEST_DOC_test_path;
use crate::threading::pool::*;
use std::unreachable;

fn make_pool() -> ThreadPool {
    ThreadPool::new(4, get_LARGE_TEST_DOC_test_path())
}

fn expect_result(of_class: KnownObject, result: Option<JobResult>) {
    assert!(result.is_some());

    match result.unwrap() {
        JobResult::Classification {
            page: _,
            res: _,
            as_class,
        } => assert_eq!(as_class, of_class),
        JobResult::Extraction {
            page: _,
            res: _,
            as_class: _,
        } => unreachable!("shouldn't get an extraction case when only scheduling classifications."),
    }
}

const FIRST_PAGE: Page = Page(FIRST_CHAPTER_PAGE);

#[test]
fn test_poll_blocking() {
    let mut pool = make_pool();

    pool.classify(KnownObject::CHAPTER, FIRST_PAGE);
    pool.classify(KnownObject::SUBCHAPTER, FIRST_PAGE);

    expect_result(KnownObject::CHAPTER, pool.poll_blocking()); // should be the first available
    expect_result(KnownObject::SUBCHAPTER, pool.poll_blocking()); // should be available after blocking again
    assert!(pool.poll_blocking().is_none()); // check for exhaustion
}

#[test]
fn test_poll_draining() {
    let mut pool = make_pool();

    for i in 0..100 {
        pool.classify(KnownObject::CHAPTER, Page(i));
    }

    let poll = pool.poll_draining();

    let mut total_classify_jobs_polled = 0;
    for res in poll {
        if let JobResult::Classification {
            page: _,
            res: _,
            as_class: _,
        } = res
        {
            total_classify_jobs_polled += 1;
        }
    }
    assert_eq!(total_classify_jobs_polled, 100);
}
