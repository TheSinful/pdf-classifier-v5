use std::{fmt::Display, ops::Index};

use crate::{
    constraints::{
        Constraint,
        overrides::{OverrideAction, OverrideStream, OverrideStreamExitCase},
    },
    context::Context,
    generated::generated_object_types::KnownObject,
    page::Page,
};

// todo: make fill_with pattern more robust. currently no type exists to explain structural patterns.

pub struct MultiPageHierarchyBreak {
    pub after: KnownObject,
    pub impl_blank_after: bool,
    pub end_on: KnownObject,
    pub fill_with: &'static [KnownObject], 
    current_fillin_idx: usize,
}

impl MultiPageHierarchyBreak {
    pub const fn new(
        after: KnownObject,
        impl_blank_after: bool,
        end_on: KnownObject,
        fill_with: &'static [KnownObject],
    ) -> Self {
        Self {
            after,
            impl_blank_after,
            end_on,
            fill_with,
            current_fillin_idx: 0,
        }
    }
}

impl Constraint for MultiPageHierarchyBreak {}

impl Display for MultiPageHierarchyBreak {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MultiPageHierarchyBreak {{after: {}, end_on: {}, expected_blank_after_impl: {}, fill_with: {:?}}}",
            self.after, self.end_on, self.impl_blank_after, self.fill_with
        )
    }
}

unsafe impl Sync for MultiPageHierarchyBreak {}

impl OverrideStream for MultiPageHierarchyBreak {
    fn step(&mut self, _: &Context, _: Page) -> OverrideAction {
        let class = self.fill_with.index(self.current_fillin_idx);

        if self.current_fillin_idx + 1 > self.fill_with.len() {
            self.current_fillin_idx = 0;
        } else {
            self.current_fillin_idx += 1;
        }

        OverrideAction::ClassifyAs(*class)
    }

    fn should_enter(&self, ctx: &Context, page: Page) -> bool {
        if self.impl_blank_after {
            let is_prev_page_unknown = ctx.previous_page_inference(page) == &KnownObject::UNKNOWN;
            let is_prev_page_target = ctx.previous_page_inference(page.previous()) == &self.after;
            return is_prev_page_unknown && is_prev_page_target;
        } else {
            let is_prev_page_target = ctx.previous_page_inference(page) == &self.after;
            return is_prev_page_target;
        }
    }

    fn should_exit(&self, _: &Context,  _: Page) -> OverrideStreamExitCase {
        return OverrideStreamExitCase::IfClassifiedAs(self.end_on);
    }
}
