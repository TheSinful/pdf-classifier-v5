pub mod invalid_pair;
pub mod natural_child;

use super::impl_constraint_enum;
use crate::context::Context;
use crate::generated::generated_object_types::KnownObject;
use crate::page::Page;
use crate::weighting::constraints::Constraint;
use invalid_pair::ValidPairRule;
use natural_child::NaturalChild;

trait HardConstraint: Constraint {
    fn eval(ctx: &Context, class: KnownObject, page: Page) -> bool;
}


impl_constraint_enum!(
    HardConstraints,
    bool,
    IsNaturalChild = NaturalChild,
    InvalidPair = ValidPairRule
);
