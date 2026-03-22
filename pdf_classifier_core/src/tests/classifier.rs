use super::init::get_LARGE_TEST_DOC_test_path;
use crate::tests::init::get_TEST_OUTPUT_DIR_test_path;
use crate::{
    classifier::Classifier, context::ContextUpdate, generated::generated_object_types::KnownObject,
    inferencer::Inferencer,
};
use serde_json::to_string;
use std::io::Write;
use std::ops::Index;

const LARGE_TEST_DOC_END_PAGE: u32 = 1280;
const LARGE_TEST_DOC_START_PAGE: u32 = 44;

fn init_classifier(page_start: u32, page_end: u32) -> Classifier {
    Classifier::new(
        page_start.into(),
        page_end.into(),
        Inferencer::new(),
        0,
        4,
        get_LARGE_TEST_DOC_test_path(),
    )
}

#[derive(serde::Serialize)]
struct Decision {
    pub page: u32,
    pub class: String,
}

#[test]
fn test_classifier_start() {
    let mut classifier = init_classifier(LARGE_TEST_DOC_START_PAGE, LARGE_TEST_DOC_START_PAGE + 4);

    let results = classifier.start().unwrap();

    let mut decisions: Vec<String> = vec![];
    for result in &results {
        let update = &result.context_updates[0];

        match update {
            ContextUpdate::Decision(page, known_object) => {
                let decision = Decision {
                    page: page.num,
                    class: known_object.to_string(),
                };

                decisions.push(to_string(&decision).unwrap());
            }
            ContextUpdate::NewParent(_) => continue,
        }
    }

    let out_dir = get_TEST_OUTPUT_DIR_test_path();
    let mut file = std::fs::File::create(out_dir.join("test_classify_full_run.json")).unwrap();
    for decision in decisions {
        file.write(decision.as_bytes()).unwrap();
    }
}

#[test]
fn test_first_page_classification() {
    let mut classifier = init_classifier(0, 1);

    let res = classifier.start().unwrap();

    assert!(res.len() == 1);
    assert!(res.index(0).context_updates.len() == 1);

    let update = res.index(0).context_updates.index(0);
    match update {
        ContextUpdate::Decision(page, known_object) => {
            assert!(page.num == 0);
            assert!(known_object == &KnownObject::CHAPTER)
        }
        ContextUpdate::NewParent(_) => panic!("unexpected update"),
    }
}
