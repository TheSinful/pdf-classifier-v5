use super::{Reward, SoftConstraint};
use crate::context::Context;
use crate::generated::generated_object_types::KnownObject;
use crate::generated::reflected_objects::is_root;
use crate::page::Page;
use crate::score::Score;

pub struct RootObjectReward;

impl Reward for RootObjectReward {}

impl SoftConstraint for RootObjectReward {
    const REWARD: f32 = 1.2;

    fn eval(ctx: &Context, class: KnownObject, page: Page) -> Score {
        if ctx.is_first_page(page) && is_root(class) {
            Self::REWARD.into()
        } else {
            Score::NO_AFFECT()
        }
    }
}
