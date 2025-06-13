//! Generator for externally tagged enums

use crate::de::{deserialize_externally_tagged_variant, Parameters};
use crate::fragment::Match;
use crate::internals::ast::Variant;
use crate::internals::attr;
use proc_macro2::{Literal, TokenStream};
use quote::quote;

/// Generates implementation of `Variant` and `VariantVisitor` for `FromFlatten`
pub fn generate_variant_visitor(params: &Parameters, variants: &[Variant]) -> TokenStream {
    let expecting = format!("variant identifier of enum `{}`", params.type_name());

    // Skip any variants that has #[serde(skip_deserializing)]
    let variants = variants.iter().filter(|v| !v.attrs.skip_deserializing());

    let variant_names = variants.clone().flat_map(|variant| variant.attrs.aliases());
    let declare_variant = variants.clone().map(|variant| {
        let ident = &variant.ident;
        quote!(#ident)
    });
    let visit_bytes = variants.filter_map(|variant| {
        let name = &variant.ident;
        // `aliases` also contains a main name
        let aliases = variant
            .attrs
            .aliases()
            .iter()
            .map(|alias| Literal::byte_string(alias.value.as_bytes()));
        Some(quote! {
            #(#aliases|)* => _serde::__private::Ok(_serde::de::Key::Own(__Variant::#name))
        })
    });

    quote! {
        #[doc(hidden)]
        const VARIANTS: &'static [&'static str] = &[ #(#variant_names),* ];

        #[allow(non_camel_case_types)]
        #[doc(hidden)]
        pub enum __Variant {
            #(#declare_variant,)*
        }

        #[doc(hidden)]
        struct __VariantVisitor;

        #[automatically_derived]
        impl<'de> _serde::de::Visitor<'de> for __VariantVisitor {
            type Value = _serde::de::Key<__Variant>;

            fn expecting(&self, __formatter: &mut _serde::__private::Formatter) -> _serde::__private::fmt::Result {
                _serde::__private::Formatter::write_str(__formatter, #expecting)
            }

            fn visit_str<__E>(self, value: &str) -> _serde::__private::Result<Self::Value, __E>
            where
                __E: _serde::de::Error,
            {
                self.visit_bytes(value.as_bytes())
            }

            fn visit_bytes<__E>(self, __value: &[u8]) -> _serde::__private::Result<Self::Value, __E>
            where
                __E: _serde::de::Error,
            {
                match __value {
                    #(#visit_bytes,)*
                    _ => _serde::__private::Ok(_serde::de::Key::Unknown)
                }
            }
        }

        #[automatically_derived]
        impl<'de> _serde::de::DeserializeSeed<'de> for __VariantVisitor {
            type Value = Variant;

            fn deserialize<__D>(self, __deserializer: __D) -> _serde::__private::Result<Self::Value, __D::Error>
            where
                __D: _serde::Deserializer<'de>,
            {
                match _serde::Deserializer::deserialize_identifier(self, __deserializer) {
                    _serde::__private::Ok(_serde::de::Key::Own(variant)) => _serde::__private::Ok(variant),
                    _serde::__private::Ok(_serde::de::Key::Unknown) => _serde::__private::Err(Error::unknown_variant("TODO", VARIANTS)),
                    _serde::__private::Err(__err) => _serde::__private::Err(__err),
                }
            }
        }
    }
}

/// Generates implementation of `ValueVisitor` for `FromFlatten`
pub fn generate_value_visitor(
    params: &Parameters,
    variants: &[Variant],
    cattrs: &attr::Container,
) -> TokenStream {
    let this_type = &params.this_type;
    let expecting = format!("enum {}", params.type_name());
    let expecting = cattrs.expecting().unwrap_or(&expecting);
    let (de_impl_generics, de_ty_generics, ty_generics, where_clause) = params.generics();
    let delife = params.borrowed.de_lifetime();

    // Skip any variants that has #[serde(skip_deserializing)]
    let variants = variants.iter().filter(|v| !v.attrs.skip_deserializing());
    let construct = variants.map(|variant| {
        let ident = &variant.ident;

        let block = Match(deserialize_externally_tagged_variant(
            params, variant, cattrs,
        ));

        quote!(__Variant::#ident => #block)
    });

    quote! {
        #[doc(hidden)]
        struct __ValueVisitor #de_impl_generics #where_clause {
            value: _serde::__private::Option<#this_type #ty_generics>,
            __ignore: _serde::__private::PhantomData<(&#delife (), #this_type #ty_generics)>,
        }

        impl #de_impl_generics __ValueVisitor #de_ty_generics #where_clause {
            fn construct_enum<__A>(self, __variant: __Variant, access: __A) -> _serde::__private::Result<#this_type #ty_generics, __A::Error>
            where
                __A: _serde::de::VariantAccess<'de>,
            {
                match __variant {
                    #(#construct,)*
                }
            }
        }

        #[automatically_derived]
        impl #de_impl_generics _serde::de::Visitor<#delife> for __ValueVisitor #de_ty_generics #where_clause {
            type Value = #this_type #ty_generics;

            fn expecting(&self, __formatter: &mut _serde::__private::Formatter) -> _serde::__private::fmt::Result {
                _serde::__private::Formatter::write_str(__formatter, #expecting)
            }

            fn visit_enum<__A>(self, __data: __A) -> Result<Self::Value, __A::Error>
            where
                __A: _serde::de::EnumAccess<#delife>,
            {
                match _serde::de::EnumAccess::variant_seed(__data, __VariantVisitor) {
                    _serde::__private::Ok((__variant, __access)) => self.construct_enum(__variant, __access),
                    _serde::__private::Err(__err) => _serde::__private::Err(__err),
                }
            }
        }

        #[automatically_derived]
        impl #de_impl_generics _serde::de::Storage<#delife> for __ValueVisitor #de_ty_generics #where_clause {
            type Field = __Variant;

            fn consume_value<__A>(&mut self, __map: &mut __A, __key: Self::Field) -> _serde::__private::Result<(), __A::Error>
            where
                __A: _serde::de::MapAccess<#delife>,
            {
                match self.construct_enum(__key, _serde::de::adapters::MapAsVariantAccess::new(__map)) {
                    _serde::__private::Ok(__value) => {
                        if _serde::__private::Option::is_some(&self.value) {
                            return _serde::__private::Err(<__A::Error as _serde::de::Error>::duplicate_field("TODO: enum consume_value"));
                        }
                        self.value = self.value = _serde::__private::Some(__value);
                        _serde::__private::Ok(())
                    }
                    _serde::__private::Err(__err) => _serde::__private::Err(__err),
                }
            }
        }
    }
}

/// Generates implementation of `FromFlatten`
pub fn generate_from_flatten(params: &Parameters) -> TokenStream {
    let ident = &params.local;
    let (de_impl_generics, de_ty_generics, ty_generics, where_clause) = params.generics();
    let delife = params.borrowed.de_lifetime();

    quote! {
        #[automatically_derived]
        impl #de_impl_generics _serde::de::FromFlatten<#delife> for #ident #ty_generics #where_clause {
            type ValueVisitor = __ValueVisitor #de_ty_generics;
            type FieldVisitor = __VariantVisitor;

            #[inline]
            fn value_visitor() -> Self::ValueVisitor {
                __ValueVisitor {
                    value: _serde::__private::None,
                    __ignore: _serde::__private::PhantomData,
                }
            }

            #[inline]
            fn field_visitor() -> Self::FieldVisitor {
                __VariantVisitor
            }

            fn construct<__E>(__storage: Self::ValueVisitor) -> _serde::__private::Result<Self, __E>
            where
                __E: _serde::de::Error,
            {
                match __storage.value {
                    _serde::__private::Some(value) => _serde::__private::Ok(value),
                    _serde::__private::None => _serde::__private::Err(_serde::de::Error::missing_field("TODO: enum construct")),
                }
            }
        }
    }
}
