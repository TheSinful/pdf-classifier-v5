use super::Page;
use super::unknown::Unknown;

#[derive(Clone, Copy)]
pub enum DeferBlock {
    Complete { inner: CompleteDeferBlock },
    Incomplete { inner: IncompleteDeferBlock },
}

impl DeferBlock {
    pub fn new(start_page: Page) -> Self {
        Self::Incomplete {
            inner: IncompleteDeferBlock {
                start_page: start_page,
                end_page: Unknown::pending(),
            },
        }
    }

    pub fn get_start_page(&self) -> Page {
        match self {
            DeferBlock::Complete { inner } => inner.start_page,
            DeferBlock::Incomplete { inner } => inner.start_page,
        }
    }

    pub fn complete(self, end_page: Page) -> Self {
        match self {
            DeferBlock::Complete { inner: _ } => {
                unreachable!("Cannot complete an already complete defer")
            }
            DeferBlock::Incomplete {
                inner: incomplete_inner,
            } => Self::Complete {
                inner: incomplete_inner.complete(end_page),
            },
        }
    }
}

#[derive(Clone, Copy)]
struct CompleteDeferBlock {
    start_page: Page,
    end_page: Page,
}

impl CompleteDeferBlock {
    fn new(start_page: Page, end_page: Page) -> Self {
        Self {
            start_page,
            end_page,
        }
    }
}

#[derive(Clone, Copy)]
struct IncompleteDeferBlock {
    start_page: Page,
    end_page: Unknown<Page>,
}

impl IncompleteDeferBlock {
    fn new(start_page: Page) -> Self {
        Self {
            start_page,
            end_page: Unknown::pending(),
        }
    }

    fn complete(mut self, end_page: Page) -> CompleteDeferBlock {
        self.end_page.define(end_page);
        // .define ensures self.end_page is Some
        CompleteDeferBlock::new(self.start_page, self.end_page.unwrap())
    }
}
