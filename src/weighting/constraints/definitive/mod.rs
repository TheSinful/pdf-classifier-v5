mod first_page;

use super::impl_constraint_enum;
use crate::context::Context;
use crate::generated::generated_object_types::KnownObject;
use crate::page::Page;
use crate::weighting::constraints::Constraint;
use first_page::FirstPageRoot;

trait DefinitiveConstraint: Constraint {
    fn eval(ctx: &Context, class: KnownObject, page: Page) -> bool;
}

impl_constraint_enum!(DefinitiveConstraints, bool, FirstPageRoot = FirstPageRoot);
