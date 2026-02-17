use std::{fmt::Display, ops::Sub};

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub struct Page {
    pub num: u32,
}

impl Display for Page {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{}", self.num))
    }
}

impl From<usize> for Page {
    fn from(value: usize) -> Self {
        Self { num: value as u32 }
    }
}

impl From<u32> for Page {
    fn from(value: u32) -> Self {
        Self { num: value }
    }
}

impl Into<u32> for Page {
    fn into(self) -> u32 {
        self.num
    }
}

impl Into<usize> for Page {
    fn into(self) -> usize {
        self.num as usize
    }
}

impl Sub for Page {
    type Output = Page;

    fn sub(self, rhs: Self) -> Self::Output {
        Page::new(self.num - rhs.num)
    }
}

impl Page {
    pub fn new(num: u32) -> Self {
        Page { num }
    }
}
