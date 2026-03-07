// Currently, each soft constraint is meant to tally into a budget of 1.5 (total)
// Which will be its soft-max, although to mitigate maxing the budget everytime,
// Generally for now they will try to add up to around ~1.2
// This is also to leave some space for future soft-constraints especially
// ones which have more importance.

pub mod pair_rewards;
pub mod skipped_children;

use super::impl_constraint_enum;
use crate::context::Context;
use crate::generated::generated_object_types::KnownObject;
use crate::page::Page;
use crate::score::Score;
use crate::weighting::constraints::Constraint;
use pair_rewards::PairRewards;
use skipped_children::SkippedChildrenConstraint;

/// Marker trait that indicates self penalizes the returned evaluation
/// (or rather is -f32)
/// Doesn't do anything substantial, just makes it explicit that
/// self is going to return a negative float.
trait Penalty: SoftConstraint {}

/// Marker trait that indicates self rewards the returned evaluation
/// (or rather is +f32)
/// Doesn't do anything substantial, just makes it explicit that
/// self is going to return a positive float.
trait Reward: SoftConstraint {}

pub trait SoftConstraint: Constraint {
    fn eval(ctx: &Context, class: KnownObject, page: Page) -> Score;
}

impl_constraint_enum!(
    SoftConstraints,
    Score,
    REWARD_PairOrder = PairRewards,
    PENALTY_SkippedChildren = SkippedChildrenConstraint
);
