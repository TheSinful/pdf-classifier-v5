mod blank_after;

use pdf_classifier_macros::impl_instansiated_constraint_enum;

use crate::constraints::Constraint;
use crate::context::Context;
use crate::generated::generated_object_types::KnownObject;
use crate::page::Page;

pub enum OverrideAction {
    Skip,
    InferenceAs(KnownObject),
    ClassifyAs(KnownObject),
}

pub trait Override<T>: Constraint {
    fn eval(
        &self,
        ctx: &Context,
        class: KnownObject,
        page: Page,
    ) -> Option<OverrideAction>;
}

impl_instansiated_constraint_enum!(OverrideConstraints, Option<OverrideAction>, BlankAfterClass = blank_after::BlankAfterClass);