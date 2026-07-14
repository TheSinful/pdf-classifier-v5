use serde::{Deserialize, Serialize};
use std::{
    fmt::Display,
    ops::{Add, AddAssign, Deref, Sub},
};

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Page(pub u32);

impl Deref for Page {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for Page {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{}", self.0))
    }
}

impl From<usize> for Page {
    fn from(value: usize) -> Self {
        Self(value as u32)
    }
}

impl From<u32> for Page {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl Sub for Page {
    type Output = Page;

    fn sub(self, rhs: Self) -> Self::Output {
        Page(self.0 - rhs.0)
    }
}

impl AddAssign for Page {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0
    }
}

impl Add for Page {
    type Output = Page;

    fn add(self, rhs: Self) -> Self::Output {
        Page(self.0 + rhs.0)
    }
}

impl Page {
    pub fn previous(&self) -> Page {
        Page(self.0 - 1)
    }

    pub fn next(&mut self) -> () {
        self.0 += 1
    }
}
