mod hard;
mod soft;

pub use soft::ENUM_VARIANT_COUNT as SOFT_ENUM_VARIANT_COUNT;
pub use soft::SoftConstraints;

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
    };
}

pub(crate) use impl_constraint_enum;
