use std::{
    f32,
    ops::{Add, Sub},
};

// I chose to prefix with P and N for positive/negative since
// the only other visual difference is in the "un" prefix later in the name
#[allow(non_camel_case_types)]
#[derive(PartialEq, PartialOrd, Default, Clone, Copy)]
pub enum Score {
    REWARD_Heavy, // 1.0
    REWARD_Light, // 0.5
    #[default]
    Neutral, // 0.0
    PUNISHMENT_Light, // -0.5
    PUNISHMENT_Heavy, // -0.0
    Custom(f32),  // [-1.0, 1.0]
}

impl From<f32> for Score {
    fn from(value: f32) -> Self {
        debug_assert!(value <= 1.0 && value >= -1.0);

        match value {
            1.0 => Score::REWARD_Heavy,
            0.5 => Score::REWARD_Light,
            0.0 => Score::Neutral,
            -0.5 => Score::PUNISHMENT_Light,
            -1.0 => Score::PUNISHMENT_Heavy,
            _ => Score::Custom(value),
        }
    }
}

impl Into<f32> for Score {
    fn into(self) -> f32 {
        match self {
            Score::REWARD_Heavy => 1.0,
            Score::REWARD_Light => 0.5,
            Score::Neutral => 0.0,
            Score::PUNISHMENT_Light => -0.5,
            Score::PUNISHMENT_Heavy => -1.0,
            Score::Custom(f) => f,
        }
    }
}

impl Score {
    fn into_f32(self) -> f32 {
        self.into()
    }
}

impl Eq for Score {}

impl Ord for Score {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let _self: f32 = self.into_f32();
        _self
            .partial_cmp(&other.into_f32())
            .unwrap_or(std::cmp::Ordering::Less)
    }
}

impl Add for Score {
    type Output = Score;
    fn add(self, rhs: Self) -> Self::Output {
        (self.into_f32() + rhs.into_f32()).into()
    }
}

impl Sub for Score {
    type Output = Score;
    fn sub(self, rhs: Self) -> Self::Output {
        (self.into_f32() - rhs.into_f32()).into()
    }
}
