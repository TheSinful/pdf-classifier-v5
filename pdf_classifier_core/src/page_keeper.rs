use crate::page::Page;

type CurrentPage = Page;

pub enum PageKeeper {
    Unlocked(CurrentPage),
    Locked(CurrentPage),
}

impl PageKeeper {
    pub fn lock(self) -> Self {
        match self {
            PageKeeper::Unlocked(pg) => Self::Locked(pg),
            PageKeeper::Locked(pg) => Self::Locked(pg),
        }
    }

    pub fn unlock(self) -> Self {
        match self {
            PageKeeper::Unlocked(pg) => Self::Unlocked(pg),
            PageKeeper::Locked(pg) => Self::Unlocked(pg),
        }
    }

    pub fn increment(&mut self) -> Option<()> {
        match self {
            PageKeeper::Unlocked(pg) => {
                pg.next();
                Some(())
            }
            PageKeeper::Locked(_) => None,
        }
    }

    pub fn increment_by(&mut self, by: impl Into<u32>) -> Option<()> {
        match self {
            PageKeeper::Unlocked(keeper) => {
                *keeper = Page(keeper.0 + by.into());
                Some(())
            }
            PageKeeper::Locked(_) => None,
        }
    }
}
