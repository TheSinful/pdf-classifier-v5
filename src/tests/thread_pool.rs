use crate::threading::pool::*;
use crate::{generated::generated_object_types::KnownObject};
use super::util::get_test_doc_path;

#[test]
fn test_poll() {
    let mut pool = ThreadPool::new(4, get_test_doc_path());
    pool.schedule(KnownObject::CHAPTER, 0u32.into());

    let mut _prev = 0;
    let mut _classified = false;
    loop {
        _prev += 3;

        let mut results = pool.poll();

        if results.is_empty() {
            continue; // awaiting  
        }

        let res = results.pop().unwrap();

        match res {
            JobResult::Classification {
                page,
                res,
                as_class,
            } => {
                assert!(
                    res.is_ok(),
                    "Classification failed {}",
                    res.as_ref().err().unwrap()
                );

                dbg!(
                    "Classified page {} as class {}",
                    page.num,
                    as_class.to_string()
                );

                _classified = true;
            }
            JobResult::Extraction {
                page,
                res,
                as_class,
            } => {
                assert!(
                    res.is_ok(),
                    "Extraction failed {}",
                    res.err().unwrap().extract_fail_rsn()
                );

                dbg!(
                    "Extracted page {} as class {}",
                    page.num,
                    as_class.to_string()
                );

                break;
            }
        }

        assert!(_classified)
    }
}
