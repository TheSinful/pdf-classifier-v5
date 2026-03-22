mod definitive;
mod hard;
mod overrides;
mod soft;

pub use definitive::DefinitiveConstraints;
pub use definitive::ENUM_VARIANT_COUNT as DEFINITIVE_ENUM_VARIANT_COUNT;
pub use hard::ENUM_VARIANT_COUNT as HARD_ENUM_VARIANT_COUNT;
pub use hard::HardConstraints;
pub use soft::ENUM_VARIANT_COUNT as SOFT_ENUM_VARIANT_COUNT;
pub use soft::SoftConstraints;

pub trait Constraint {}

#[derive(thiserror::Error, Debug)]
pub enum CastError {
    #[error("Failed to cast discriminant {0} into a constraint type!")]
    ConstraintCastError(usize),
}

pub use pdf_classifier_macros::impl_constraint_enum;
