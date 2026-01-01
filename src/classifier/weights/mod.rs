pub mod structural;

use std::ops::{Add, Deref};

struct Weight(pub f32);

impl Deref for Weight {
    type Target = f32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Add for Weight {
    type Output = Weight;

    fn add(self, rhs: Self) -> Self::Output {
        Weight(self.0 + rhs.0)
    }
}


