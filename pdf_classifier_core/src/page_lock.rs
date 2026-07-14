use crate::page::Page;
use tracing::instrument;

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

    #[instrument]
    pub fn increment(&mut self) -> () {
        self.increment_by(Page(1))
    }

    #[instrument(skip(self))]
    pub fn increment_by(&mut self, by: Page) -> () {
        match self {
            PageLock::Unlocked(pg) => {
                *pg = *pg + by;
            }
            PageLock::Locked(pg) => panic!(
                "Attempted to advance a locked page cursor past page {}! (use try_increment_by when standing still is intended)",
                pg
            ),
        }
    }

    pub fn try_increment_by(&mut self, by: Page) -> Option<()> {
        match self {
            PageLock::Unlocked(_) => {
                self.increment_by(by);
                Some(())
            }
            PageLock::Locked(pg) => {
                tracing::trace!(page = %pg, "skipped increment of locked page cursor");
                None
            }
        }
    }

    pub fn get(&self) -> &Page {
        match self {
            PageLock::Unlocked(page) => page,
            PageLock::Locked(page) => page,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PageLock;
    use crate::page::Page;

    #[test]
    fn unlocked_increments() {
        let mut lock = PageLock::Unlocked(Page(4));
        lock.increment();
        assert_eq!(*lock.get(), Page(5));

        lock.increment_by(Page(3));
        assert_eq!(*lock.get(), Page(8));
    }

    #[test]
    #[should_panic(expected = "locked page cursor")]
    fn locked_increment_panics() {
        let mut lock = PageLock::Unlocked(Page(4)).lock();
        lock.increment();
    }

    #[test]
    fn try_increment_noops_when_locked() {
        let mut lock = PageLock::Unlocked(Page(4)).lock();
        assert!(lock.try_increment_by(Page(1)).is_none());
        assert_eq!(*lock.get(), Page(4));

        let mut lock = lock.unlock();
        assert!(lock.try_increment_by(Page(1)).is_some());
        assert_eq!(*lock.get(), Page(5));
    }

    #[test]
    fn lock_round_trip_preserves_page() {
        let lock = PageLock::Unlocked(Page(7)).lock();
        assert_eq!(*lock.get(), Page(7));
        let lock = lock.unlock();
        assert_eq!(*lock.get(), Page(7));
    }
}
