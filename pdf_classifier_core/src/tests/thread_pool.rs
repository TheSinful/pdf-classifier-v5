use crate::constants::test_constants::FIRST_CHAPTER_PAGE;
use crate::generated::generated_object_types::KnownObject;
use crate::page::Page;
use crate::tests::init::get_LARGE_TEST_DOC_test_path;
use crate::threading::pool::*;

fn make_pool() -> ThreadPool {
    ThreadPool::new(4, get_LARGE_TEST_DOC_test_path())
}

fn expect_result(of_classes: &[KnownObject], result: JobResult) -> Option<bool> {
    match result {
        JobResult::Classification {
            page: _,
            res: _,
            as_class,
        } => Some(of_classes.contains(&as_class)),
        JobResult::Extraction {
            page: _,
            res: _,
            as_class: _,
        } => None,
    }
}

const FIRST_PAGE: Page = Page(FIRST_CHAPTER_PAGE);

#[test]
fn test_poll_blocking() {
    let mut pool = make_pool();

    pool.classify(KnownObject::CHAPTER, FIRST_PAGE);
    pool.classify(KnownObject::SUBCHAPTER, FIRST_PAGE);

    let used_classes = &[KnownObject::CHAPTER, KnownObject::SUBCHAPTER];

    loop {
        let Some(result) = pool.poll_blocking() else {
            break;
        };

        let Some(current_poll) = expect_result(used_classes, result) else {
            continue; // early extraction
        };

        assert!(current_poll)
    }

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
