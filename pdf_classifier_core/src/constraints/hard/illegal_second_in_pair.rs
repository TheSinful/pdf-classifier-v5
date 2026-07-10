use super::HardConstraint;
use crate::{constraints::Constraint, context::Context, page::Page};

/// Removes any secondary pair classes when the previous page wasn't a primary pair class.  
pub struct IllegalSecondInPair;

impl Constraint for IllegalSecondInPair {}

impl HardConstraint for IllegalSecondInPair {
    fn eval(
        ctx: &Context,
        class: crate::generated::generated_object_types::KnownObject,
        page: Page,
    ) -> bool {
        let prev_classifiation = ctx.previous_page_inference(page);
        if !prev_classifiation.is_first_in_pair() && class.is_second_in_pair() {
            tracing::trace!(
                "FAIL (IllegalSecondInPair): previous page {} wasn't a primary pair, while class {} is a secondary pair",
                prev_classifiation,
                class
            );
            return Self::FAIL();
        }

        return Self::PASS();
    }
}
