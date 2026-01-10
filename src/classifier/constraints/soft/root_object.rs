use crate::classifier::constraints::soft::{NO_AFFECT, Reward, SoftConstraint};
use crate::classifier::context::ClassifierContext;
use crate::generated::generated_object_types::KnownObject;
use crate::generated::reflected_objects::is_root;
use crate::page::Page;

const IS_ROOT_REWARD: f32 = 1.2; /* Should take up total budget, guarantee that first page is root */

pub struct RootObjectReward;

impl Reward for RootObjectReward {}

impl SoftConstraint for RootObjectReward {
    fn eval(ctx: &ClassifierContext, class: KnownObject, page: Page ) -> f32 {
        if ctx.is_first_page(page) && is_root(class) {
            IS_ROOT_REWARD
        } else {
            NO_AFFECT
        }
    }
}
