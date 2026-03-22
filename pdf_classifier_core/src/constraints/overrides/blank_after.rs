use crate::{
    constraints::{
        Constraint,
        overrides::{Override, OverrideAction},
    },
    generated::generated_object_types::KnownObject,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlankAfterClass {
    config: KnownObject,
}

impl Constraint for BlankAfterClass {}

impl Override<KnownObject> for BlankAfterClass {
    fn eval(
        &self,
        ctx: &crate::context::Context,
        _class: crate::generated::generated_object_types::KnownObject,
        page: crate::page::Page,
    ) -> Option<OverrideAction> {
        if *ctx.previous_page_inference(page) == self.config {
            return Some(OverrideAction::Skip);
        }

        None
    }
}
