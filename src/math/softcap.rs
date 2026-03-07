use std::{f32::consts::E, ops::Neg};

const SOFT_CAP_SCALE: f32 = 1.0;

pub fn soft_cap(coll: Vec<f32>, max: f32) -> f32 {
    let sum: f32 = coll.iter().sum();

    max * (1.0 - E.powf(sum.neg() / SOFT_CAP_SCALE))

}
