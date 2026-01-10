use super::*;
use crate::generated::reflected_objects;

pub struct NaturalChild;

impl HardConstraint for NaturalChild {
    fn eval(ctx: &ClassifierContext, class: KnownObject, _: Page) -> bool {
        reflected_objects::is_child(ctx.current_parent, class)
    }
}
