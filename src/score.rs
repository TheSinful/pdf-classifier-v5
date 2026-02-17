use std::{
    f32,
    ops::{Add, AddAssign, Deref, Sub},
};

#[derive(PartialEq, PartialOrd, Default, Clone, Copy)]
pub struct Score(f32);

impl Score {
    #[allow(non_snake_case)]
    pub fn NEG_INFINITY() -> Self {
        Self {
            0: f32::NEG_INFINITY,
        }
    }

    #[allow(non_snake_case)]
    pub fn NO_AFFECT() -> Self {
        Self { 0: 0.0f32 }
    }
}

impl Eq for Score {}

impl Ord for Score {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0
            .partial_cmp(&other.0)
            .unwrap_or(std::cmp::Ordering::Less)
    }
}

impl From<f32> for Score {
    fn from(value: f32) -> Self {
        Self { 0: value }
    }
}

impl Into<f32> for Score {
    fn into(self) -> f32 {
        self.0
    }
}

impl Deref for Score {
    type Target = f32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Add for Score {
    type Output = Score;
    fn add(self, rhs: Self) -> Self::Output {
        (self.0 + rhs.0).into()
    }
}

impl Sub for Score {
    type Output = Score;
    fn sub(self, rhs: Self) -> Self::Output {
        (self.0 - rhs.0).into()
    }
}

impl AddAssign for Score {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0
    }
}
