use crate::context::Context;
use crate::generated::generated_object_types::KnownObject;
use crate::generated::reflected_objects::is_root;
use crate::page::Page;
use crate::weighting::constraints::Constraint;
use crate::weighting::constraints::definitive::DefinitiveConstraint;

pub struct FirstPageRoot;

impl Constraint for FirstPageRoot {}

impl DefinitiveConstraint for FirstPageRoot {
    fn eval(ctx: &Context, class: KnownObject, page: Page) -> bool {
        ctx.is_first_page(page) && is_root(class)
    }
}
