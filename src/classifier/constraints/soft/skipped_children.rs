use crate::classifier::constraints::soft::{Penalty, SoftConstraint};
use crate::classifier::context::ClassifierContext;
use crate::classifier::score::Score;
use crate::generated::generated_object_types::KnownObject;
use crate::generated::reflected_objects::is_child;
use crate::page::Page;

/// In a state where we skip children
/// but class is ending a pair, we end up
/// prioritizing the ended pair over children inbetween
/// therefore, this has to be higher (in the budget) than new pair reward
const SKIPPED_CHILDREN_CONSTRAINT_PENALTY: f32 = -0.6;

pub struct SkippedChildrenConstraint;

impl Penalty for SkippedChildrenConstraint {}

impl SoftConstraint for SkippedChildrenConstraint {
    fn eval(ctx: &ClassifierContext, class: KnownObject, page: Page) -> Score {
        let prev_inference = ctx.previous_page_inference(page.into());

        if prev_inference.has_children() && !is_child(*prev_inference, class) {
            SKIPPED_CHILDREN_CONSTRAINT_PENALTY.into()
        } else {
            Score::NO_EFFECT()
        }
    }
}
