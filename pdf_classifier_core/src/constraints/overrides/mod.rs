mod blank_after;
mod mp_hierarchy_break;

pub use blank_after::BlankAfter;
pub use mp_hierarchy_break::MultiPageHierarchyBreak;

use crate::constraints::Constraint;
use crate::context::Context;
use crate::generated::generated_object_types::KnownObject;
use crate::page::Page;
use pdf_classifier_macros::impl_instansiated_constraint_enum;
use std::fmt::Display;

#[derive(Debug)]
pub enum OverrideAction {
    Skip,
    InferAs(KnownObject),
    ClassifyAs(KnownObject),
}

impl Display for OverrideAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OverrideAction::Skip => write!(f, "skip"),
            OverrideAction::InferAs(class) => write!(f, "inference as {}", class),
            OverrideAction::ClassifyAs(class) => write!(f, "classify as {}", class),
        }
    }
}

pub enum OverrideStreamExitCase {
    IfClassifiedAs(KnownObject),
    Exit,
}

pub trait OverrideStream: Constraint + Display + Sync + Send {
    fn step(&mut self, ctx: &Context, page: Page) -> OverrideAction;
    fn should_enter(&self, ctx: &Context, page: Page) -> bool;
    fn should_exit(&self, ctx: &Context, page: Page) -> OverrideStreamExitCase;
}

pub trait Override: Constraint + Display {
    fn eval(&self, ctx: &Context, class: KnownObject, page: Page) -> Option<OverrideAction>;
}

impl_instansiated_constraint_enum!(
    OverrideConstraints,
    Option<OverrideAction>,
    BlankAfter = BlankAfter
);
