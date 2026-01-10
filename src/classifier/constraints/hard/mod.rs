pub mod invalid_pair;
pub mod natural_child;

use super::impl_constraint_enum;
use crate::classifier::constraints::hard::invalid_pair::ValidPairRule;
use crate::classifier::constraints::hard::natural_child::NaturalChild;
use crate::classifier::context::ClassifierContext;
use crate::generated::generated_object_types::KnownObject;
use crate::page::Page;

trait HardConstraint {
    fn eval(ctx: &ClassifierContext, class: KnownObject, page: Page) -> bool;
}

impl_constraint_enum!(
    HardConstraints,
    bool,
    IsNaturalChild = NaturalChild,
    InvalidPair = ValidPairRule
);
