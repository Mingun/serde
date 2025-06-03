//! Generator for struct deserializer

use crate::de::Parameters;
use crate::internals::ast::Field;
use crate::internals::attr;
use proc_macro2::{Literal, TokenStream};
use quote::quote;

fn generate_bounds(fields: &[Field]) -> Vec<TokenStream> {
    fields
        .iter()
        .filter_map(|field| {
            let ty = field.ty;
            // Skip any fields that has #[serde(skip_deserializing)]
            if !field.attrs.skip_deserializing() && field.attrs.flatten() {
                Some(quote!(#ty: FromFlatten<'de>))
            } else {
                None
            }
        })
        .collect()
}

/// Generates implementation of `Field` and `FieldVisitor` for `FromFlatten`
pub fn generate_field_visitor(params: &Parameters, fields: &[Field]) -> TokenStream {
    let expecting = format!("field identifier of struct {}", params.type_name());
    let this_type = &params.this_type;
    let (de_impl_generics, de_ty_generics, ty_generics, where_clause) = params.generics();
    let delife = params.borrowed.de_lifetime();

    // Skip any fields that has #[serde(skip_deserializing)]
    let fields = fields.iter().filter(|f| !f.attrs.skip_deserializing());

    let declare_field = fields.clone().map(|field| {
        let ident = &field.member;
        let ty = field.ty;
        if field.attrs.flatten() {
            quote!(#ident(<<#ty as _serde::de::FromFlatten<#delife>>::ValueVisitor as _serde::de::Storage<#delife>>::Field))
        } else {
            quote!(#ident)
        }
    });
    let declare_visitor = fields.clone().filter_map(|field| {
        let ident = &field.member;
        let ty = field.ty;
        if field.attrs.flatten() {
            Some(quote!(#ident: <#ty as _serde::de::FromFlatten<#delife>>::FieldVisitor))
        } else {
            None
        }
    });
    let visit_own = fields.clone().filter_map(|field| {
        let name = &field.member;
        // `aliases` also contains a main name
        let aliases = field
            .attrs
            .aliases()
            .iter()
            .map(|alias| Literal::byte_string(alias.value.as_bytes()));
        if !field.attrs.flatten() {
            Some(quote! {
                #(#aliases|)* => _serde::__private::Ok(_serde::de::Key::Own(__Field::#name))
            })
        } else {
            None
        }
    });
    let visit_flatten = fields.clone().filter_map(|field| {
        let ident = &field.member;
        if field.attrs.flatten() {
            Some(quote! {
                if let _serde::de::Key::Own(key) = self.#ident.visit_bytes(__value)? {
                    return _serde::__private::Ok(_serde::de::Key::Own(__Field::#ident(key)));
                }
            })
        } else {
            None
        }
    });

    quote! {
        #[allow(non_camel_case_types)]
        #[doc(hidden)]
        pub enum __Field #de_impl_generics #where_clause {
            #(#declare_field,)*

            __ignore(_serde::__private::PhantomData<(&#delife (), #this_type #ty_generics)>),
        }

        #[doc(hidden)]
        struct __FieldVisitor #de_impl_generics #where_clause {
            #(#declare_visitor,)*

            __ignore: _serde::__private::PhantomData<(&#delife (), #this_type #ty_generics)>,
        }

        #[automatically_derived]
        impl #de_impl_generics _serde::de::Visitor<#delife> for __FieldVisitor #de_ty_generics #where_clause {
            type Value = _serde::de::Key<__Field #de_ty_generics>;

            fn expecting(&self, __formatter: &mut _serde::__private::Formatter) -> _serde::__private::fmt::Result {
                _serde::__private::Formatter::write_str(__formatter, #expecting)
            }

            fn visit_str<__E>(self, __value: &str) -> _serde::__private::Result<Self::Value, __E>
            where
                __E: _serde::de::Error,
            {
                self.visit_bytes(__value.as_bytes())
            }

            fn visit_bytes<__E>(self, __value: &[u8]) -> _serde::__private::Result<Self::Value, __E>
            where
                __E: _serde::de::Error,
            {
                match __value {
                    #(#visit_own,)*
                    _ => {
                        #(#visit_flatten)*
                        _serde::__private::Ok(_serde::de::Key::Unknown)
                    }
                }
            }
        }

        #[automatically_derived]
        impl #de_impl_generics _serde::de::DeserializeSeed<#delife> for __FieldVisitor #de_ty_generics #where_clause {
            type Value = _serde::de::Key<__Field #de_ty_generics>;

            fn deserialize<__D>(self, __deserializer: __D) -> _serde::__private::Result<Self::Value, __D::Error>
            where
                __D: _serde::Deserializer<#delife>,
            {
                _serde::Deserializer::deserialize_identifier(__deserializer, self)
            }
        }
    }
}

/// Generates implementation of `ValueVisitor` for `FromFlatten`
pub fn generate_value_visitor(
    params: &Parameters,
    fields: &[Field],
    cattrs: &attr::Container,
) -> TokenStream {
    let this_type = &params.this_type;
    let expecting = format!("struct {}", params.type_name());
    let expecting = cattrs.expecting().unwrap_or(&expecting);
    let (de_impl_generics, de_ty_generics, ty_generics, where_clause) = params.generics();
    let delife = params.borrowed.de_lifetime();

    // Skip any fields that has #[serde(skip_deserializing)]
    let fields = fields.iter().filter(|f| !f.attrs.skip_deserializing());

    let declare = fields.clone().map(|field| {
        let ident = &field.member;
        let ty = field.ty;
        if field.attrs.flatten() {
            quote!(#ident: <#ty as _serde::de::FromFlatten<#delife>>::ValueVisitor)
        } else {
            quote!(#ident: _serde::__private::Option<#ty>)
        }
    });
    let construct = fields.clone().map(|field| {
        let ident = &field.member;
        if field.attrs.flatten() {
            quote! {
                __Field::#ident(key) => self.#ident.consume_value(__map, key)?,
            }
        } else {
            let name = field.attrs.name().deserialize_name();
            quote! {
                __Field::#ident => {
                    if self.#ident.is_some() {
                        return _serde::__private::Err(<__A::Error as _serde::de::Error>::duplicate_field(#name));
                    }
                    self.#ident = _serde::__private::Some(__map.next_value()?);
                }
            }
        }
    });

    quote! {
        #[doc(hidden)]
        struct __ValueVisitor #de_impl_generics #where_clause {
            #(#declare,)*

            __ignore: _serde::__private::PhantomData<(&#delife (), #this_type #ty_generics)>,
        }

        #[automatically_derived]
        impl #de_impl_generics _serde::de::Visitor<#delife> for __ValueVisitor #de_ty_generics #where_clause {
            type Value = #this_type #ty_generics;

            fn expecting(&self, __formatter: &mut _serde::__private::Formatter) -> _serde::__private::fmt::Result {
                _serde::__private::Formatter::write_str(__formatter, #expecting)
            }

            fn visit_map<__A>(mut self, mut __map: __A) -> _serde::__private::Result<Self::Value, __A::Error>
            where
                __A: _serde::de::MapAccess<#delife>,
            {
                while let _serde::__private::Some(key) = map.next_key_seed(Self::Value::field_visitor())? {
                    match key {
                        _serde::de::Key::Own(key) => self.consume_value(&mut __map, key)?,
                        _serde::de::Key::Unknown => {
                            return _serde::__private::Err(_serde::de::Error::unknown_field("TODO", FIELDS));
                        }
                    }
                }
                _serde::de::Storage::construct(self)
            }
        }

        #[automatically_derived]
        impl #de_impl_generics _serde::de::Storage<#delife> for __ValueVisitor #de_ty_generics #where_clause {
            type Field = __Field #ty_generics;

            fn consume_value<__A>(&mut self, __map: &mut __A, __key: Self::Field) -> _serde::__private::Result<(), __A::Error>
            where
                __A: _serde::de::MapAccess<#delife>,
            {
                match __key {
                    #(#construct)*
                }
                _serde::__private::Ok(())
            }
        }
    }
}

/// Generates implementation of `FromFlatten`
pub fn generate_from_flatten(params: &Parameters, fields: &[Field]) -> TokenStream {
    let ident = &params.local;
    let (de_impl_generics, de_ty_generics, ty_generics, where_clause) = params.generics();
    let delife = params.borrowed.de_lifetime();

    // Skip any fields that has #[serde(skip_deserializing)]
    let fields = fields.iter().filter(|f| !f.attrs.skip_deserializing());

    let value_visitor = fields.clone().map(|field| {
        let ident = &field.member;
        let ty = field.ty;
        if field.attrs.flatten() {
            quote!(#ident: #ty::value_visitor())
        } else {
            quote!(#ident: _serde::__private::None)
        }
    });

    let field_visitor = fields.clone().filter_map(|field| {
        let ident = &field.member;
        let ty = field.ty;
        if field.attrs.flatten() {
            Some(quote!(#ident: #ty::field_visitor()))
        } else {
            None
        }
    });

    let construct = fields.map(|field| {
        let ident = &field.member;
        if field.attrs.flatten() {
            quote!(#ident: _serde::de::FromFlatten::construct(__storage.#ident)?)
        } else {
            let name = field.attrs.name().deserialize_name();
            quote! {
                #ident: match __storage.#ident {
                    _serde::__private::Some(value) => value,
                    _serde::__private::None => return _serde::__private::Err(_serde::de::Error::missing_field(#name)),
                }
            }
        }
    });

    quote! {
        #[automatically_derived]
        impl #de_impl_generics _serde::de::FromFlatten<#delife> for #ident #ty_generics #where_clause {
            type ValueVisitor = __ValueVisitor #de_ty_generics;
            type FieldVisitor = __FieldVisitor #de_ty_generics;

            #[inline]
            fn value_visitor() -> Self::ValueVisitor {
                __ValueVisitor {
                    #(#value_visitor,)*

                    __ignore: _serde::__private::PhantomData,
                }
            }

            #[inline]
            fn field_visitor() -> Self::FieldVisitor {
                __FieldVisitor {
                    #(#field_visitor,)*

                    __ignore: _serde::__private::PhantomData,
                }
            }

            fn construct<__E>(__storage: Self::ValueVisitor) -> _serde::__private::Result<Self, __E>
            where
                __E: _serde::de::Error,
            {
                _serde::__private::Ok(#ident {
                    #(#construct,)*
                })
            }
        }
    }
}
