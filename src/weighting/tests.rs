use crate::weighting::constraints::{SOFT_ENUM_VARIANT_COUNT, SoftConstraints};

#[test]
fn test_transmute_into_constraint_safety_assurance() {
    let out_of_bounds_idx_upper = SOFT_ENUM_VARIANT_COUNT + 1;
    let result = super::transmute_into_constraint(out_of_bounds_idx_upper);

    match result {
        Ok(_) => {
            panic!("Shouldn't be able to get a valid SoftConstraint with an out of bounds index!")
        }
        Err(_) => { /* expected */ }
    }
}

#[test]
fn test_transmute_accuracy() {
    let result = super::transmute_into_constraint(0);

    let constraint = result.unwrap();
    assert_eq!(constraint, SoftConstraints::REWARD_PairOrder);
}
