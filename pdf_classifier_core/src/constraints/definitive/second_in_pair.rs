use crate::constraints::Constraint;
use crate::constraints::definitive::DefinitiveConstraint;
use crate::context::Context;
use crate::generated::generated_object_types::KnownObject;
use crate::page::Page;

#[derive(PartialEq, Clone, Copy, Debug, Hash)]
pub struct SecondInPair;

impl Constraint for SecondInPair {}

impl DefinitiveConstraint for SecondInPair {
    fn eval(ctx: &Context, class: KnownObject, page: Page) -> bool {
        ctx.previous_page_inference(page).is_first_in_pair() && class.is_second_in_pair()
    }
}
