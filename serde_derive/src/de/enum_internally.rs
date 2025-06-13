//! Generator for internally tagged enums

use crate::de::Parameters;
use crate::internals::ast::Variant;
use crate::internals::attr;
use proc_macro2::TokenStream;
use quote::quote;

/// Generates implementation of `Variant` and `VariantVisitor` for `FromFlatten`
pub fn generate_variant_visitor(params: &Parameters, variants: &[Variant]) -> TokenStream {
    // TODO: другой Visitor для Unit вариантов
    super::enum_adjacently::generate_variant_visitor(params, variants)
}

/// Generates implementation of `FromFlatten`
pub fn generate_from_flatten(
    params: &Parameters,
    cattrs: &attr::Container,
    tag: &str,
) -> TokenStream {
    let ident = &params.local;
    let (de_impl_generics, de_ty_generics, ty_generics, where_clause) = params.generics();
    let delife = params.borrowed.de_lifetime();

    let rust_name = params.type_name();
    let expecting = format!("internally tagged enum `{}`", rust_name);
    let expecting = cattrs.expecting().unwrap_or(&expecting);

    quote! {
        #[automatically_derived]
        impl #de_impl_generics _serde::de::FromFlatten<#delife> for #ident #ty_generics #where_clause {
            type ValueVisitor = _serde::de::InternallyTaggedEnumValueVisitor<#delife, __Seed #de_ty_generics, Self>;
            type FieldVisitor = __FieldVisitor #de_ty_generics;

            #[inline]
            fn value_visitor() -> Self::ValueVisitor {
                _serde::de::InternallyTaggedEnumValueVisitor::new(#tag, #expecting)
            }

            #[inline]
            fn field_visitor() -> Self::FieldVisitor {
                __FieldVisitor {
                    tag: #tag,
                    __ignore: _serde::__private::PhantomData,
                }
            }

            #[inline]
            fn construct<__E>(__storage: Self::ValueVisitor) -> _serde::__private::Result<Self, __E>
            where
                __E: _serde::de::Error,
            {
                todo!()
            }
        }
    }
}
