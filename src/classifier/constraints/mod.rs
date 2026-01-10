mod hard;
mod soft;

pub use soft::SoftConstraints; 
pub use hard::HardConstraints;

macro_rules! impl_constraint_enum {
    ($enum_name: ident, $eval_return: ty, $( $variant:ident = $variant_ty:ty ),* $(,)? ) => {
        #[allow(non_camel_case_types)]
        pub enum $enum_name {
            $($variant),*
        }

        impl $enum_name {
            pub fn eval(&self, ctx: &ClassifierContext, class: KnownObject, page: crate::page::Page) -> $eval_return {
                match self {
                    $(
                        $enum_name::$variant => <$variant_ty>::eval(ctx, class, page),
                    )*
                }
            }
        }
    };
}

pub(crate) use impl_constraint_enum;