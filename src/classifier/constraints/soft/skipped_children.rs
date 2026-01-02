use crate::classifier::constraints::soft::{Penalty, SoftConstraint};
use crate::classifier::context::ClassifierContext;
use crate::generated::generated_object_types::KnownObject;
use crate::generated::reflected_objects::is_child;

const SKIPPED_CHILDREN_CONSTRAINT_PENALTY: f32 = -2.0;

pub struct SkippedChildrenConstraint;

impl Penalty for SkippedChildrenConstraint {}

impl SoftConstraint for SkippedChildrenConstraint {
    fn eval(ctx: &ClassifierContext, class: KnownObject) -> f32 {
        let prev_inference = ctx.previous_page();

        if prev_inference.has_children() && !is_child(prev_inference, class) {
            SKIPPED_CHILDREN_CONSTRAINT_PENALTY
        } else {
            0.0
        }
    }
}
