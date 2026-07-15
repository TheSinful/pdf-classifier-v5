use tracing::instrument;

use super::{ClassificationError, committed::CommittedClassifier};
use crate::{
    generated::{
        generated_object_types::KnownObject,
        reflected_objects::{get_all_dependents, get_global_independents},
    },
    obj_list::KnownObjectList,
    page::Page,
    page_lock::PageLock,
    threading::pool::JobResult,
};

#[derive(Debug)]
pub struct DeferralClassifier {
    pub base: CommittedClassifier,
    page_lock: PageLock,
    start_page: Page,
    end_page: Option<Page>,
    current_independent: KnownObject,
    independents: Vec<KnownObject>,
}

impl DeferralClassifier {
    pub fn new(mut base: CommittedClassifier) -> Self {
        let mut independents = get_global_independents();
        base.page_lock = base.page_lock.lock();

        Self {
            start_page: base.current_page(),
            page_lock: base.page_lock.clone().unlock(),
            base,
            current_independent: independents
                .pop()
                .expect("should've had independents defined!"),
            independents,
            end_page: None,
        }
    }

    pub fn current_page(&self) -> Page {
        *self.page_lock.get()
    }

    #[instrument(name = "enter_deferral", skip_all, fields(page = %self.current_page(), available_workers = self.base.thread_pool.available_workers_count()))]
    pub fn find_next_independent(mut self) -> Result<CommittedClassifier, ClassificationError> {
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

    #[instrument(name = "exit_deferral", skip(self), fields(from_page = %self.start_page))]
    fn finalize(
        mut self,
        on_page: Page,
        as_class: KnownObject,
    ) -> Result<CommittedClassifier, ClassificationError> {
        self.base.page_lock = PageLock::Unlocked(on_page);
        self.base.try_decide_as(as_class, on_page).unwrap(); // no error possible

        self.end_page = Some(on_page);

        self.fill_in_dependents(as_class)?;

        while let Some(_) = self.base.poll() {}

        self.base.should_defer = false;
        self.base.page_lock.increment();
        Ok(self.base)
    }

    #[instrument(skip_all, fields(ended_on_class = %finalized_as))]
    fn fill_in_dependents(&mut self, finalized_as: KnownObject) -> Result<(), ClassificationError> {
        tracing::trace!("filling in dependents for class {}", finalized_as);

        let dependents = get_all_dependents(finalized_as);

        if dependents.len() == 2
            && dependents[0].is_first_in_pair()
            && dependents[1].is_second_in_pair()
        {
            return Ok(self.fill_in_with_only_pair((dependents[0], dependents[1])));
        } // i.e example document type, of dependents: diagram-datatable

        if dependents.len() == 1 {
            return Ok(self.fill_in_with_sole_class(dependents[0]));
        } // i.e document type with a sole datatable class

        return self.fill_in_by_standard_classification(finalized_as); // runs standard inferencing 
    }

    #[instrument(skip_all)]
    fn fill_in_by_standard_classification(
        &mut self,
        finalized_as: KnownObject,
    ) -> Result<(), ClassificationError> {
        tracing::trace!(
            "filling in deferred pages by standard classification for class {}",
            finalized_as
        );

        let dependents = KnownObjectList::from_vec(get_all_dependents(finalized_as));
        let fill_page_range = self.fill_range();

        for (page_offset, page) in fill_page_range.iter().enumerate() {
            let inferenced = self
                .base
                .inferencer
                .infer(&mut self.base.ctx, *page, &dependents)?;

            self.base
                .decide_and_classify_as(inferenced, self.start_page + Page::from(page_offset))?;
        }

        Ok(())
    }

    #[instrument(skip(self))]
    fn fill_in_with_only_pair(&mut self, pair: (KnownObject, KnownObject)) -> () {
        self.fill_range()
            .into_iter()
            .enumerate()
            .for_each(|(i, page)| match i % 2 {
                0 => {
                    tracing::trace!("filling in page {} as pair object {}", page, pair.0);
                    self.base.decide_and_classify_as(pair.0, page).unwrap()
                }
                _ => {
                    tracing::trace!("filling in page {} as pair object {}", page, pair.1);
                    self.base.decide_and_classify_as(pair.1, page).unwrap()
                }
            });
    }

    #[instrument(skip(self))]
    fn fill_in_with_sole_class(&mut self, class: KnownObject) -> () {
        self.fill_range()
            .into_iter()
            .for_each(|page| self.base.decide_and_classify_as(class, page).unwrap());
    }

    fn get_end_page(&self) -> Page {
        match self.end_page {
            Some(s) => s,
            None => panic!(
                "Attempted to finalize deferral without calling DeferralClassifier::finalize."
            ),
        }
    }

    #[instrument(name = "deferral_fill_range", skip_all, fields(from_page = %self.start_page, to_page = ?self.end_page, range = self.current_page().0 - self.get_end_page().0))]
    fn fill_range(&self) -> Vec<Page> {
        (self.start_page.0..self.get_end_page().0)
            .map(|num| Page(num))
            .collect()
    }

    #[instrument(name = "eliminate_class", skip_all, fields(page = %on_page, class = %as_class))]
    fn handle_failed(&mut self, on_page: Page, as_class: KnownObject) -> () {
        self.base.ctx.guarantee_failure_of(as_class, on_page);
    }

    #[instrument(name = "check_independent", skip(self), fields(page = %self.current_page(), independent = %self.current_independent))]
    fn spawn_worker(&mut self) -> () {
        self.base
            .thread_pool
            .classify_unchecked(self.current_independent, self.current_page());

        self.page_lock.increment();

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
        let mut classifier =
            init_test_committed_classifier(FIRST_SUB_CHAPTER_PAGE, LARGE_TEST_DOC_END_PAGE);

        classifier
            .decide_and_classify_as(KnownObject::SUBCHAPTER, Page(FIRST_SUB_CHAPTER_PAGE))
            .unwrap();

        classifier.defer();

        let classifier = DeferralClassifier::new(classifier)
            .find_next_independent()
            .expect("deferral should've succeeded");

        for page_num in FIRST_SUB_CHAPTER_PAGE + 1..SECOND_SUB_CHAPTER_PAGE - 1 {
            let page = Page(page_num);

            let filled_in = classifier
                .ctx
                .pages
                .get(&page)
                .expect(&format!("should've filled in page {}", page));

            if page.0 % 2 == 0 {
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
