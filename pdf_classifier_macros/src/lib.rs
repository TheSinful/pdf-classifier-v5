use proc_macro::TokenStream;
use quote::quote;
use syn::{Ident, Token, Type, parse::Parse, punctuated::Punctuated};

struct Variant {
    name: Ident,
    ty: Type,
}

impl Parse for Variant {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        input.parse::<Token![=]>()?;
        let ty: Type = input.parse()?;

        Ok(Self { name, ty })
    }
}

type VariantsTokenStream = Punctuated<Variant, Token![,]>;
struct ConstraintInput {
    enum_name: Ident,
    eval_return: Type,
    variants: VariantsTokenStream,
}

impl Parse for ConstraintInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let enum_name: Ident = input.parse()?;
        input.parse::<Token![,]>()?;

        let eval_return: Type = input.parse()?;
        input.parse::<Token![,]>()?;

        let variants = Punctuated::<Variant, Token![,]>::parse_terminated(input)?;

        Ok(Self {
            enum_name,
            eval_return,
            variants,
        })
    }
}

#[proc_macro]
pub fn impl_instansiated_constraint_enum(tokens: TokenStream) -> TokenStream {
    let ConstraintInput {
        enum_name,
        eval_return,
        variants,
    } = syn::parse_macro_input!(tokens as ConstraintInput);

    let enum_variants = variants.iter().map(|v| {
        let name = &v.name;
        let ty = &v.ty;

        quote!(
            #name(#ty),
        )
    });

    let eval_arms = variants.iter().map(|v| {
        let name = &v.name;
        let ty = &v.ty;

        quote!(
            Self::#name(s) => #ty::eval(s, ctx, class, page),
        )
    });

    let display_arms = variants.iter().map(|v| {
        let name = &v.name;

        quote!(
            Self::#name(_) => write!(f, "{}", stringify!(#name))?,
        )
    });

    let enum_variant_count = enum_variants.len();

    quote!(
        pub const ENUM_VARIANT_COUNT: usize = #enum_variant_count;
        
        #[allow(non_camel_case_types)]
        #[derive(PartialEq, Clone, Copy, Debug, Hash)]
        pub enum #enum_name {
            #(#enum_variants)*
        }

        impl #enum_name {
            pub fn eval(&self, ctx: &crate::context::Context, class: KnownObject, page: crate::page::Page) -> #eval_return {
                match self {
                    #(#eval_arms)*
                }
            }
        }

       impl std::fmt::Display for #enum_name {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                match self {
                    #(#display_arms)*
                };

                Ok(())
            }
        }
    )
    .into()
}

#[proc_macro]
pub fn impl_constraint_enum(tokens: TokenStream) -> TokenStream {
    let ConstraintInput {
        enum_name,
        eval_return,
        variants,
    } = syn::parse_macro_input!(tokens as ConstraintInput);

    let enum_variants = variants.iter().map(|v| {
        let name = &v.name;

        quote!(
            #name,
        )
    });

    let eval_arms = variants.iter().map(|v| {
        let name = &v.name;
        let ty = &v.ty;

        quote!(
            Self::#name => #ty::eval(ctx, class, page),
        )
    });

    let display_arms = variants.iter().map(|v| {
        let name = &v.name;

        quote!(
            Self::#name => write!(f, "{}", stringify!(#name))?,
        )
    });

    let try_from_arms = variants.iter().enumerate().map(|v| {
        let idx = v.0;
        let name = &v.1.name;

        quote!(
            #idx => Ok(Self::#name),
        )
    });

    let enum_variant_count = enum_variants.len();

    quote!(
        pub const ENUM_VARIANT_COUNT: usize = #enum_variant_count;
        
        #[allow(non_camel_case_types)]
        #[derive(PartialEq, Clone, Copy, Debug, Hash)]
        pub enum #enum_name {
            #(#enum_variants)*
        }

        impl #enum_name {
            pub fn eval(&self, ctx: &crate::context::Context, class: KnownObject, page: crate::page::Page) -> #eval_return {
                match self {
                    #(#eval_arms)*
                }
            }
        }

       impl std::fmt::Display for #enum_name {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                match self {
                    #(#display_arms)*
                };

                Ok(())
            }
        }

        impl TryFrom<usize> for #enum_name {
            type Error = crate::constraints::CastError;

            fn try_from(v: usize) -> Result<Self, Self::Error> {
                match v {
                    #(#try_from_arms)*
                    _ => Err(crate::constraints::CastError::ConstraintCastError(v))
                }
            }
        }
    )
    .into()
}
