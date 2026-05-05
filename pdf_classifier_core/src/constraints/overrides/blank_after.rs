use std::fmt::Display;

use crate::{
    constraints::{
        Constraint,
        overrides::{Override, OverrideAction},
    },
    generated::generated_object_types::KnownObject,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlankAfter {
    pub config: KnownObject,
}

impl Constraint for BlankAfter {}

impl Override for BlankAfter {
    fn eval(
        &self,
        ctx: &crate::context::Context,
        _class: KnownObject,
        page: crate::page::Page,
    ) -> Option<OverrideAction> {
        if page == ctx.start_page {
            return None;
        }

        if *ctx.previous_page_inference(page) == self.config {
            return Some(OverrideAction::Skip);
        }

        None
    }
}

impl Display for BlankAfter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BlankAfter<{}>", self.config.to_string())
    }
}
