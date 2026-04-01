mod blank_after;

use std::fmt::Display;

pub use blank_after::BlankAfter;

use crate::constraints::Constraint;
use crate::context::Context;
use crate::generated::generated_object_types::KnownObject;
use crate::page::Page;
use pdf_classifier_macros::impl_instansiated_constraint_enum;

pub enum OverrideAction {
    Skip,
    InferenceAs(KnownObject),
    ClassifyAs(KnownObject),
}

impl Display for OverrideAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OverrideAction::Skip => write!(f, "skip"),
            OverrideAction::InferenceAs(class) => write!(f, "inference as {}", class),
            OverrideAction::ClassifyAs(class) => write!(f, "classify as {}", class),
        }
    }
}

pub trait Override: Constraint + ToString {
    fn eval(&self, ctx: &Context, class: KnownObject, page: Page) -> Option<OverrideAction>;
}

impl_instansiated_constraint_enum!(
    OverrideConstraints,
    Option<OverrideAction>,
    BlankAfter = BlankAfter
);
