use crate::{
    constraints::{Constraint, soft::SoftConstraint},
    context::Context,
    generated::reflected_objects::is_independent,
    page::Page,
    score::Score,
};

/// Ensures that if inference class 'x', has a pair
/// That it is the first in pair if the previous page was not a pair type
///
/// In other words, in a range of inferences [n,x] of size two
/// if n.does_not_have_pair {
///     return x.is_first_in_pair
/// }
pub struct FirstInPairConstraint;

impl Constraint for FirstInPairConstraint {}

impl SoftConstraint for FirstInPairConstraint {
    fn eval(
        ctx: &Context,
        class: crate::generated::generated_object_types::KnownObject,
        page: Page,
    ) -> crate::score::Score {
        if !class.has_pair() {
            return Score::Neutral;
        }

        let prev_classifiation = ctx.previous_page_inference(page);
        let valid_prev_classification =
            prev_classifiation.is_second_in_pair() || is_independent(*prev_classifiation);
        if valid_prev_classification && class.is_first_in_pair() {
            return Score::REWARD_Heavy;
        }

        return Score::Neutral;
    }
}
