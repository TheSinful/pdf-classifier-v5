use super::{Penalty, SoftConstraint};
use crate::context::Context;
use crate::generated::generated_object_types::KnownObject;
use crate::generated::reflected_objects::is_child;
use crate::page::Page;
use crate::score::Score;
use crate::weighting::constraints::Constraint;

pub struct SkippedChildrenConstraint;

impl Constraint for SkippedChildrenConstraint {}

impl Penalty for SkippedChildrenConstraint {}

impl SoftConstraint for SkippedChildrenConstraint {
    fn eval(ctx: &Context, class: KnownObject, page: Page) -> Score {
        let prev_inference = ctx.previous_page_inference(page.into());

        if !is_child(*prev_inference, class) {
            Score::PUNISHMENT_Heavy
        } else {
            Score::Neutral
        }
    }
}
