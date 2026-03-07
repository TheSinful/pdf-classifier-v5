use super::*;
use crate::generated::reflected_objects;

pub struct NaturalChild;

impl Constraint for NaturalChild {}

impl HardConstraint for NaturalChild {
    fn eval(ctx: &Context, class: KnownObject, _: Page) -> bool {
        reflected_objects::is_child(ctx.current_parent, class)
    }
}
