use super::HardConstraint;
use crate::generated::reflected_objects;

pub struct ValidPairRule;

impl HardConstraint for ValidPairRule {
    fn eval(
        ctx: &crate::classifier::context::ClassifierContext,
        class: crate::generated::generated_object_types::KnownObject,
    ) -> bool {
        reflected_objects::is_pair(ctx.previous_page(), class)
    }
}
