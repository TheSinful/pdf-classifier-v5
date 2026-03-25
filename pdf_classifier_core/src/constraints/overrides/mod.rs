mod blank_after;

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

impl ToString for OverrideAction {
    fn to_string(&self) -> String {
        match self {
            OverrideAction::Skip => "skip".to_string(),
            OverrideAction::InferenceAs(class) => format!("inference as {}", class.to_string()),
            OverrideAction::ClassifyAs(class) => format!("classify as {}", class.to_string()),
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
