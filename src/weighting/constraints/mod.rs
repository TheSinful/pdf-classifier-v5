mod hard;
mod soft;
mod definitive;

pub use hard::ENUM_VARIANT_COUNT as HARD_ENUM_VARIANT_COUNT;
pub use hard::HardConstraints;
pub use soft::ENUM_VARIANT_COUNT as SOFT_ENUM_VARIANT_COUNT;
pub use soft::SoftConstraints;
pub use definitive::ENUM_VARIANT_COUNT as DEFINITIVE_ENUM_VARIANT_COUNT; 
pub use definitive::DefinitiveConstraints;

pub trait Constraint {}

#[derive(thiserror::Error, Debug)]
pub enum CastError {
    #[error("Failed to cast discriminant {0} into a constraint type!")]
    ConstraintCastError(u8),
}

macro_rules! impl_constraint_enum {
    ($enum_name: ident, $eval_return: ty, $( $variant:ident = $variant_ty:ty ),* $(,)? ) => {
        #[allow(non_camel_case_types)]
        #[repr(u8)]
        #[derive(PartialEq, Clone, Copy, Debug, Hash)]
        pub enum $enum_name {
            $($variant),*
        }

        impl $enum_name {
            pub fn eval(&self, ctx: &crate::context::Context, class: KnownObject, page: crate::page::Page) -> $eval_return {
                match self {
                    $(
                        $enum_name::$variant => <$variant_ty>::eval(ctx, class, page),
                    )*
                }
            }
        }

        pub const ENUM_VARIANT_COUNT: u8 = <[()]>::len(&[
            $( { let _ = stringify!($variant); () } ),*
        ]) as u8;


        impl TryFrom<u8> for $enum_name {
            type Error = crate::weighting::constraints::CastError;

            fn try_from(v: u8) -> Result<Self, Self::Error> {
                if v < ENUM_VARIANT_COUNT {
                    // SAFETY:
                    // - repr(u8)
                    // - discriminants are contiguous from 0
                    // - bounds checked above
                    Ok(unsafe { core::mem::transmute(v) })
                } else {
                    Err(crate::weighting::constraints::CastError::ConstraintCastError(v))
                }
            }
        }

        impl std::fmt::Display for $enum_name {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                match self {
                    $(
                        $enum_name::$variant => write!(f, "{}", stringify!($variant))?, 
                    )*
                };

                Ok(())
            }
        }


    };
}

pub(crate) use impl_constraint_enum;
