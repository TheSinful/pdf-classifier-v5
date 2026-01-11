use crate::classifier::constraints::soft::{Reward, SoftConstraint};
use crate::classifier::context::ClassifierContext;
use crate::classifier::score::Score;
use crate::generated::generated_object_types::KnownObject;
use crate::page::Page;

const END_PAIR_REWARD: f32 = 0.5;
// const NEW_PAIR_REWARD: f32 = 0.0; // ! see PairRewards::eval

pub struct PairRewards;

impl Reward for PairRewards {}

impl SoftConstraint for PairRewards {
    fn eval(ctx: &ClassifierContext, class: KnownObject, page: Page) -> Score {
        // if class.has_pair() && class.is_first_in_pair() {
        //     return NEW_PAIR_REWARD;
        // }
        // ! I don't think it's a good idea to reward a new pair in general
        // ! It ends up just being extra noise because every pair is automatically
        // ! More important than any other object. So rather reward pair ending.

        let prev_page = ctx.previous_page_inference(page.into());
        let previous_page_first_in_pair = prev_page.has_pair() && prev_page.is_first_in_pair();

        if class.has_pair() && class.is_second_in_pair() && previous_page_first_in_pair {
            return END_PAIR_REWARD.into();
        }

        Score::NO_EFFECT()
    }
}
