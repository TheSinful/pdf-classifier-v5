mod first_page;
mod second_in_pair;

use super::impl_constraint_enum;
use crate::constraints::Constraint;
use crate::constraints::definitive::second_in_pair::SecondInPair;
use crate::context::Context;
use crate::generated::generated_object_types::KnownObject;
use crate::page::Page;
use first_page::FirstPageRoot;

pub trait DefinitiveConstraint: Constraint {
    fn eval(ctx: &Context, class: KnownObject, page: Page) -> bool;
}

impl_constraint_enum!(
    DefinitiveConstraints,
    bool,
    FirstPageRoot = FirstPageRoot,
    SecondInPair = SecondInPair
);
