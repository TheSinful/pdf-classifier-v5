/// Weightings for structural changes
/// Where N represents the current prediction
#[allow(non_snake_case)]
pub struct StructuralWeights {
    // /// N is not child to the current parent we are within
    // /// This should be a penalty instead of a reward since many objects
    // /// could be children to a parent, but we just need to filter out
    // /// what cannot be structurally sound.
    // // pub PENALTY_not_natural_child: f32,
    // /// N breaks the parent-child relation too quickly
    // /// I.e, subchapter -> subchapter (we missed children of subchapter)
    // // pub PENALTY_skipped_children: f32,
    // /// N completes **A PAIR** but not the current pair
    // /// initiated by a new_pair   
    // pub PENALTY_invalid_pair: f32,
    /// N is of an object that starts a new pair
    pub REWARD_new_pair: f32,
    /// N ends an expected pair
    /// (previous page was a new_pair)
    pub REWARD_end_pair: f32,
    /// N is the root object and we are on the first page.
    pub REWARD_root_object: f32,
    /// N is the only valid child to the current parent
    pub REWARD_only_valid_child: f32,
    /// N is the first child
    /// when previous page started a new current parent
    pub REWARD_first_child: f32,
}

impl StructuralWeights {
    pub fn conservative() -> Self {
        Self {
            PENALTY_not_natural_child: (),
            PENALTY_skipped_children: (),
            PENALTY_invalid_pair: (),
            REWARD_new_pair: (),
            REWARD_end_pair: (),
            REWARD_root_object: (),
            REWARD_only_valid_child: (),
            REWARD_first_child: (),
        }
    }
}
