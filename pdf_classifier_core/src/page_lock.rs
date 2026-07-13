use tracing::{debug_span, span::EnteredSpan};

use crate::page::Page;

type CurrentPage = Page;

#[derive(Debug, Clone)]
pub enum PageLock {
    Unlocked(CurrentPage),
    Locked(CurrentPage),
}

impl PageLock {
    pub fn lock(self) -> Self {
        match self {
            PageLock::Unlocked(pg) => Self::Locked(pg),
            PageLock::Locked(pg) => Self::Locked(pg),
        }
    }

    pub fn unlock(self) -> Self {
        match self {
            PageLock::Unlocked(pg) => Self::Unlocked(pg),
            PageLock::Locked(pg) => Self::Unlocked(pg),
        }
    }

    pub fn increment(&mut self) -> Option<()> {
        match self {
            PageLock::Unlocked(pg) => {
                pg.next();
                let _ = self.enter_increment_span(Page(1));
                Some(())
            }
            PageLock::Locked(_) => None,
        }
    }

    fn enter_increment_span(&self, incrementing_by: Page) -> EnteredSpan {
        debug_span!(
            "increment_current_page",
            from_page = %self.get(),
            to_page = %(*self.get() + incrementing_by)
        )
        .entered()
    }

    pub fn increment_by(&mut self, by: Page) -> Option<()> {
        match self {
            PageLock::Unlocked(keeper) => {
                *keeper = *keeper + by;
                let _ = self.enter_increment_span(by);
                Some(())
            }
            PageLock::Locked(_) => None,
        }
    }

    pub fn get(&self) -> &Page {
        match self {
            PageLock::Unlocked(page) => page,
            PageLock::Locked(page) => page,
        }
    }
}
