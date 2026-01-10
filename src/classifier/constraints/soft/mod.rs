// Currently, each soft constraint is meant to tally into a budget of 1.5 (total)
// Which will be its soft-max, although to mitigate maxing the budget everytime,
// Generally for now they will try to add up to around ~1.2
// This is also to leave some space for future soft-constraints especially
// ones which have more importance.

pub mod pair_rewards;
pub mod root_object;
pub mod skipped_children;

use super::impl_constraint_enum;
use crate::classifier::constraints::soft::pair_rewards::PairRewards;
use crate::classifier::constraints::soft::root_object::RootObjectReward;
use crate::classifier::constraints::soft::skipped_children::SkippedChildrenConstraint;
use crate::classifier::context::ClassifierContext;
use crate::generated::generated_object_types::KnownObject;
use crate::page::Page;

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

/// When [SoftConstraint]::eval -> 0.0
/// (doesn't affect any weights)
const NO_AFFECT: f32 = 0.0;

trait SoftConstraint {
    fn eval(ctx: &ClassifierContext, class: KnownObject, page: Page) -> f32;
}

impl_constraint_enum!(
    SoftConstraints,
    f32,
    REWARD_PairOrder = PairRewards,
    REWARD_RootObject = RootObjectReward,
    PENALTY_SkippedChildren = SkippedChildrenConstraint
);
