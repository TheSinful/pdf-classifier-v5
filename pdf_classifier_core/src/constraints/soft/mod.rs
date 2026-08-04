// Currently, each soft constraint is meant to tally into a budget of 1.5 (total)
// Which will be its soft-max, although to mitigate maxing the budget everytime,
// Generally for now they will try to add up to around ~1.2
// This is also to leave some space for future soft-constraints especially
// ones which have more importance.
pub mod first_in_pair;
pub mod natural_child;

use super::impl_constraint_enum;
use crate::constraints::Constraint;

use crate::context::Context;
use crate::generated::generated_object_types::KnownObject;
use crate::page::Page;
use crate::score::Score;
use natural_child::NaturalChild;

pub trait SoftConstraint: Constraint {
    fn eval(ctx: &Context, class: KnownObject, page: Page) -> Score;
}

impl_constraint_enum!(
    SoftConstraints,
    Score,
    REWARD_IsNaturalChild = NaturalChild,
    REWARD_FirstInPair = first_in_pair::FirstInPairConstraint
);
