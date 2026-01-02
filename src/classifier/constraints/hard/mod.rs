mod invalid_pair;
mod natural_child;

use crate::classifier::context::ClassifierContext;
use crate::generated::generated_object_types::KnownObject;

pub trait HardConstraint {
    fn eval(ctx: &ClassifierContext, class: KnownObject) -> bool;
}
