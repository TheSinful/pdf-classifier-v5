use super::*;
use crate::generated::reflected_objects;

pub struct NaturalChild;

impl Constraint for NaturalChild {}

impl SoftConstraint for NaturalChild {
    fn eval(ctx: &Context, class: KnownObject, _: Page) -> Score {
        if reflected_objects::is_child(ctx.current_parent, class) {
            Score::REWARD_Light
        } else {
            Score::Neutral
        }
    }
}
