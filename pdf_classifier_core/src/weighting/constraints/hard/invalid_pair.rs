use super::HardConstraint;
use crate::{context::Context, generated::reflected_objects, page::Page, weighting::constraints::Constraint};

pub struct ValidPairRule;

impl Constraint for ValidPairRule {}

impl HardConstraint for ValidPairRule {
    fn eval(
        ctx: &Context,
        class: crate::generated::generated_object_types::KnownObject,
        page: Page,
    ) -> bool {
        reflected_objects::is_pair(*ctx.previous_page_inference(page), class)
    }
}
