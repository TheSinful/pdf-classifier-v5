pub mod skipped_children;

use crate::classifier::context::ClassifierContext;
use crate::generated::generated_object_types::KnownObject;

/// Marker trait that indicates self penalizes the returned evaluation 
/// (rather is negative)
pub trait Penalty: SoftConstraint {}

/// Marker trait that indicates self rewards the returned evaluation
/// (or rather is +f32)
pub trait Reward: SoftConstraint {}

pub trait SoftConstraint {
    fn eval(ctx: &ClassifierContext, class: KnownObject) -> f32; 
}