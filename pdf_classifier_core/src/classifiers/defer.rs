use super::{ClassificationError, committed::CommittedClassifier};
use crate::{
    context::ContextUpdateHistory,
    generated::{
        generated_object_types::KnownObject,
        reflected_objects::{get_all_dependents, get_global_independents},
    },
    obj_list::KnownObjectList,
    page::Page,
    threading::pool::JobResult,
};

pub struct DeferralClassifier {
    pub base: CommittedClassifier,
    pub current_page: Page,
    start_page: Page,
    current_independent: KnownObject,
    independents: Vec<KnownObject>,
}

impl DeferralClassifier {
    pub fn new(base: CommittedClassifier) -> Self {
        let mut independents = get_global_independents();

        Self {
            start_page: base.current_page,
            current_page: base.current_page,
            base,
            current_independent: independents
                .pop()
                .expect("should've had independents defined!"),
            independents,
        }
    }

    pub fn find_next_independent(mut self) -> Result<CommittedClassifier, ClassificationError> {
        log::trace!(
            "searching for next independent starting from page {} current available workers: {}",
            self.current_page,
            self.base.thread_pool.available_workers_count()
        );

        loop {
            if self.base.thread_pool.available_workers_count() > 0 {
                self.spawn_worker();
            }

            let Some(results) = self.base.thread_pool.poll() else {
                continue;
            };

            for result in results {
                if let JobResult::Classification {
                    page,
                    res,
                    as_class,
                } = result
                {
                    match res {
                        Ok(_) => return self.finalize(page, as_class),
                        Err(_) => self.handle_failed(page, as_class),
                    }
                }
            }
        }
    }

    fn finalize(
        mut self,
        on_page: Page,
        as_class: KnownObject,
    ) -> Result<CommittedClassifier, ClassificationError> {
        log::trace!(
            "finalizing deferral on page {} as class {}",
            on_page,
            as_class
        );

        self.base.current_page = on_page;
        self.base
            .decide_as(as_class, ContextUpdateHistory::new())
            .unwrap(); // no error possible, nor do we care about the ClassificationStep

        self.fill_in_dependents(as_class)?;

        while let Some(_) = self.base.poll() {}

        Ok(self.base)
    }

    fn fill_in_dependents(&mut self, finalized_as: KnownObject) -> Result<(), ClassificationError> {
        log::trace!("filling in dependents for class {}", finalized_as);

        let dependents = get_all_dependents(finalized_as);

        if dependents.len() == 2
            && dependents[0].is_first_in_pair()
            && dependents[1].is_second_in_pair()
        {
            return Ok(self.fill_in_when_only_pair((dependents[0], dependents[1])));
        } // i.e example document type, of dependents: diagram-datatable

        if dependents.len() == 1 {
            return Ok(self.fill_in_with_sole_class(dependents[0]));
        } // i.e document type with a sole datatable class

        return self.fill_in_by_standard_classification(finalized_as); // runs standard inferencing 
    }

    fn fill_in_by_standard_classification(
        &mut self,
        finalized_as: KnownObject,
    ) -> Result<(), ClassificationError> {
        log::trace!(
            "filling in deferred pages by standard classification for class {}",
            finalized_as
        );

        let dependents = KnownObjectList::from_vec(get_all_dependents(finalized_as));
        let pages = self.fill_range();

        self.base
            .inferencer
            .infer(&mut self.base.ctx, pages, &dependents)?
            .into_iter()
            .enumerate()
            .for_each(|(page_offset, inferenced)| {
                self.base
                    .decide_and_classify_as(inferenced, self.start_page + Page::from(page_offset));
            });

        Ok(())
    }

    fn fill_in_when_only_pair(&mut self, pair: (KnownObject, KnownObject)) -> () {
        log::trace!("filling in deferred pages as pair ({}, {})", pair.0, pair.1);

        self.fill_range()
            .into_iter()
            .enumerate()
            .for_each(|(i, page)| match i % 2 {
                0 => self.base.decide_and_classify_as(pair.0, page),
                _ => self.base.decide_and_classify_as(pair.1, page),
            });
    }

    fn fill_in_with_sole_class(&mut self, class: KnownObject) -> () {
        log::trace!("filling in deferred pages as sole class {}", class);

        self.fill_range()
            .into_iter()
            .for_each(|page| self.base.decide_and_classify_as(class, page));
    }

    fn fill_range(&self) -> Vec<Page> {
        (self.start_page.num..self.current_page.num)
            .map(Into::into)
            .collect()
    }

    fn handle_failed(&mut self, on_page: Page, as_class: KnownObject) -> () {
        log::trace!(
            "guaranteed class {} to not be on page {}",
            as_class,
            on_page
        );

        self.base.ctx.guarantee_failure_of(as_class, on_page);
    }

    fn spawn_worker(&mut self) -> () {
        self.base
            .thread_pool
            .classify_unchecked(self.current_independent, self.current_page);

        self.current_page.next();

        if self.independents.is_empty() {
            self.independents = get_global_independents();
        }

        self.current_independent = self
            .independents
            .pop()
            .expect("should've refreshed independents when it emptied!")
    }
}

#[cfg(test)]
mod tests {
    use crate::constants::test_constants::*;
    use crate::{
        classifiers::{committed::init_test_committed_classifier, defer::DeferralClassifier},
        page::Page,
    };

    use crate::generated::generated_object_types::KnownObject;

    #[test]
    pub fn test_deferral_run() {
        let start_page = LARGE_TEST_DOC_START_PAGE;
        let mut classifier = init_test_committed_classifier(start_page, LARGE_TEST_DOC_END_PAGE);

        classifier
            .decide_and_classify_as(KnownObject::SUBCHAPTER, Page::from(FIRST_SUB_CHAPTER_PAGE));

        classifier.defer();

        let classifier = DeferralClassifier::new(classifier)
            .find_next_independent()
            .expect("deferral should've succeeded");

        for page in FIRST_SUB_CHAPTER_PAGE..SECOND_SUB_CHAPTER_PAGE {
            let filled_in = classifier.ctx.pages.get(&Page::from(page)).unwrap();

            if page % 2 == 0 {
                assert!(
                    *filled_in == KnownObject::DATATABLE,
                    "page {} should've been filled in as a diagram",
                    page
                )
            } else {
                assert!(
                    *filled_in == KnownObject::DIAGRAM,
                    "page {} should've been filled in as a diagram",
                    page
                )
            }
        }
    }
}
