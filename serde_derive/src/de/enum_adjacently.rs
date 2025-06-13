//! Generator for externally tagged enums

use crate::de::Parameters;
use crate::internals::ast::Variant;
use crate::internals::attr;
use proc_macro2::{Literal, TokenStream};
use quote::quote;

/// Generates implementation of `Variant` and `VariantVisitor` for `FromFlatten`
pub fn generate_variant_visitor(params: &Parameters, variants: &[Variant]) -> TokenStream {
    let expecting = format!("variant identifier of enum `{}`", params.type_name());
    let this_type = &params.this_type;
    let (de_impl_generics, de_ty_generics, ty_generics, where_clause) = params.generics();
    let delife = params.borrowed.de_lifetime();

    // Skip any variants that has #[serde(skip_deserializing)]
    let variants = variants.iter().filter(|v| !v.attrs.skip_deserializing());
    let count = variants.clone().count();

    let variant_names = variants.clone().flat_map(|variant| variant.attrs.aliases());
    let declare_variant = variants.clone().map(|variant| {
        let ident = &variant.ident;
        quote!(#ident)
    });

    // NOTE: skipped variants are not counted and does not influence to variant number
    let visit_u64 = variants.clone().enumerate().map(|(i, variant)| {
        let i = i as u64;
        let ident = &variant.ident;
        quote!(#i => _serde::__private::Ok(__Variant::#ident))
    });
    let unknown_u64 = format!("variant index 0 <= i < {}", count);

    let visit_str = variants.clone().map(|variant| {
        let ident = &variant.ident;
        let aliases = variant.attrs.aliases();
        // `aliases` also contains a main name
        quote! {
            #(#aliases|)* => _serde::__private::Ok(__Variant::#ident)
        }
    });
    let visit_bytes = variants.map(|variant| {
        let ident = &variant.ident;
        // `aliases` also contains a main name
        let aliases = variant
            .attrs
            .aliases()
            .iter()
            .map(|alias| Literal::byte_string(alias.value.as_bytes()));
        quote! {
            #(#aliases|)* => _serde::__private::Ok(__Variant::#ident)
        }
    });

    quote! {
        #[doc(hidden)]
        const VARIANTS: &'static [&'static str] = &[ #(#variant_names),* ];

        #[allow(non_camel_case_types)]
        #[doc(hidden)]
        enum __Variant {
            #(#declare_variant,)*
        }

        #[doc(hidden)]
        struct __VariantVisitor;
        #[automatically_derived]
        impl<'de> _serde::de::Visitor<'de> for __VariantVisitor {
            type Value = __Variant;

            fn expecting(&self, __formatter: &mut _serde::__private::Formatter) -> _serde::__private::fmt::Result {
                _serde::__private::Formatter::write_str(__formatter, #expecting)
            }

            fn visit_u64<__E>(self, __value: u64) -> _serde::__private::Result<Self::Value, __E>
            where
                __E: _serde::de::Error,
            {
                match __value {
                    #(#visit_u64,)*
                    _ => _serde::__private::Err(_serde::de::Error::invalid_value(
                        _serde::de::Unexpected::Unsigned(__value),
                        &#unknown_u64,
                    ))
                }
            }

            fn visit_str<__E>(self, value: &str) -> _serde::__private::Result<Self::Value, __E>
            where
                __E: _serde::de::Error,
            {
                match __value {
                    #(#visit_str,)*
                    _ => _serde::__private::Err(_serde::de::Error::unknown_variant(__value, VARIANTS)),
                }
            }

            fn visit_bytes<__E>(self, __value: &[u8]) -> _serde::__private::Result<Self::Value, __E>
            where
                __E: _serde::de::Error,
            {
                match __value {
                    #(#visit_bytes,)*
                    _ => {
                        let __value = &_serde::__private::from_utf8_lossy(__value);
                        _serde::__private::Err(_serde::de::Error::unknown_variant(__value, VARIANTS))
                    }
                }
            }
        }

        #[doc(hidden)]
        struct __Seed<'de, T> {
            variant: __Variant,
            __ignore: _serde::__private::PhantomData<(&'de (), Enum<T>)>,
        }
        #[automatically_derived]
        impl #de_impl_generics _serde::de::Deserialize<#delife> for __Seed #de_ty_generics #where_clause {
            fn deserialize<__D>(__deserializer: __D) -> _serde::__private::Result<Self, __D::Error>
            where
                __D: _serde::Deserializer<#delife>,
            {
                match _serde::Deserializer::deserialize_identifier(deserializer, __VariantVisitor) {
                    _serde::__private::Ok(__variant) => _serde::__private::Ok(__Seed {
                        variant: __variant,
                        __ignore: _serde::__private::PhantomData,
                    }),
                    _serde::__private::Err(__err) => _serde::__private::Err(__err),
                }
            }
        }
        #[automatically_derived]
        impl #de_impl_generics _serde::de::DeserializeSeed<#delife> for __Seed #de_ty_generics #where_clause {
            type Value = #this_type #ty_generics;

            fn deserialize<__D>(self, __deserializer: __D) -> _serde::__private::Result<Self::Value, __D::Error>
            where
                __D: _serde::Deserializer<#delife>,
            {
                match self.variant {
                }
            }
        }
    }
}

/// Generates implementation of `FromFlatten`
pub fn generate_from_flatten(
    params: &Parameters,
    cattrs: &attr::Container,
    tag: &str,
    content: &str,
) -> TokenStream {
    let ident = &params.local;
    let (de_impl_generics, de_ty_generics, ty_generics, where_clause) = params.generics();
    let delife = params.borrowed.de_lifetime();

    let rust_name = params.type_name();
    let expecting = format!("adjacently tagged enum {}", rust_name);
    let expecting = cattrs.expecting().unwrap_or(&expecting);

    quote! {
        #[automatically_derived]
        impl #de_impl_generics _serde::de::FromFlatten<#delife> for #ident #ty_generics #where_clause {
            type ValueVisitor = _serde::de::AdjacentlyTaggedEnumValueVisitor<#delife, __Seed #de_ty_generics, Self>;
            type FieldVisitor = _serde::de::AdjacentlyTaggedEnumFieldVisitor;

            #[inline]
            fn value_visitor() -> Self::ValueVisitor {
                _serde::de::AdjacentlyTaggedEnumValueVisitor::new(#tag, #content, #expecting)
            }

            #[inline]
            fn field_visitor() -> Self::FieldVisitor {
                _serde::de::AdjacentlyTaggedEnumFieldVisitor {
                    tag: #tag,
                    content: #content,
                }
            }

            #[inline]
            fn construct<__E>(__storage: Self::ValueVisitor) -> _serde::__private::Result<Self, __E>
            where
                __E: _serde::de::Error,
            {
                _serde::de::AdjacentlyTaggedEnumValueVisitor::construct(__storage)
            }
        }
    }
}
