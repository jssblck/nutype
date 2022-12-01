pub mod error;

use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens};
use syn::Visibility;

use self::error::{gen_error_type_name, gen_validation_error_type};
use super::models::{IntegerSanitizer, IntegerValidator, NewtypeIntegerMeta};
use crate::{common::gen::type_custom_sanitizier_closure, models::IntegerType};

pub fn gen_nutype_for_number<T>(
    doc_attrs: Vec<syn::Attribute>,
    vis: Visibility,
    number_type: IntegerType,
    type_name: &Ident,
    meta: NewtypeIntegerMeta<T>,
) -> TokenStream
where
    T: ToTokens + PartialOrd,
{
    let module_name = gen_module_name_for_type(type_name);
    let implementation = gen_implementation(type_name, number_type, &meta);

    // TODO: refactor: inject InnerType, that implements ToString
    let tp: TokenStream =
        syn::parse_str(std::any::type_name::<T>()).expect("Expected to parse a type");

    let error_type_import = match meta {
        NewtypeIntegerMeta::From { .. } => quote!(),
        NewtypeIntegerMeta::TryFrom { .. } => {
            let error_type_name = gen_error_type_name(type_name);
            quote! (
                #vis use #module_name::#error_type_name;
            )
        }
    };
    let derive = gen_derive();

    quote!(
        mod #module_name {
            use super::*;

            #(#doc_attrs)*
            #derive
            pub struct #type_name(#tp);

            #implementation
        }
        #vis use #module_name::#type_name;
        #error_type_import
    )
}

pub fn gen_implementation<T>(
    type_name: &Ident,
    inner_type: IntegerType,
    meta: &NewtypeIntegerMeta<T>,
) -> TokenStream
where
    T: ToTokens + PartialOrd,
{
    let convert_implementation = match meta {
        NewtypeIntegerMeta::From { sanitizers } => gen_new_and_from(type_name, sanitizers),
        NewtypeIntegerMeta::TryFrom {
            sanitizers,
            validators,
        } => gen_new_and_try_from(type_name, sanitizers, validators),
    };
    let methods = gen_impl_methods(type_name, inner_type);

    quote! {
        #convert_implementation
        #methods
    }
}

fn gen_impl_methods(type_name: &Ident, inner_type: IntegerType) -> TokenStream {
    quote! {
        impl ::core::convert::From<#type_name> for  #inner_type {
            fn from(val: #type_name) -> #inner_type {
                val.0
            }
        }

        impl #type_name {
            pub fn into_inner(self) -> #inner_type {
                self.0
            }
        }
    }
}

fn gen_new_and_from<T>(type_name: &Ident, sanitizers: &[IntegerSanitizer<T>]) -> TokenStream
where
    T: ToTokens + PartialOrd,
{
    let inner_type: TokenStream =
        syn::parse_str(std::any::type_name::<T>()).expect("Expected to parse a type");
    let sanitize = gen_sanitize_fn(sanitizers);

    quote!(
        #sanitize

        impl #type_name {
            pub fn new(raw_value: #inner_type) -> Self {
                Self(sanitize(raw_value))
            }
        }

        impl ::core::convert::From<#inner_type> for #type_name {
            fn from(raw_value: #inner_type) -> Self {
                Self::new(raw_value)
            }
        }
    )
}

fn gen_new_and_try_from<T>(
    type_name: &Ident,
    sanitizers: &[IntegerSanitizer<T>],
    validators: &[IntegerValidator<T>],
) -> TokenStream
where
    T: ToTokens + PartialOrd,
{
    let inner_type: TokenStream =
        syn::parse_str(std::any::type_name::<T>()).expect("Expected to parse a type");
    let sanitize = gen_sanitize_fn(sanitizers);
    let validation_error = gen_validation_error_type(type_name, validators);
    let error_type_name = gen_error_type_name(type_name);
    let validate = gen_validate_fn(type_name, validators);

    quote!(
        #sanitize
        #validation_error
        #validate

        impl #type_name {
            pub fn new(raw_value: #inner_type) -> Result<Self, #error_type_name> {
                let sanitized_value = sanitize(raw_value);
                validate(sanitized_value)?;
                Ok(#type_name(sanitized_value))
            }
        }

        impl ::core::convert::TryFrom<#inner_type> for #type_name {
            type Error = #error_type_name;

            fn try_from(raw_value: #inner_type) -> Result<#type_name, Self::Error> {
                #type_name::new(raw_value)
            }
        }
    )
}

// TODO: DRY
fn gen_module_name_for_type(type_name: &Ident) -> Ident {
    let module_name = format!("__nutype_module_for_{type_name}");
    Ident::new(&module_name, Span::call_site())
}

fn gen_sanitize_fn<T>(sanitizers: &[IntegerSanitizer<T>]) -> TokenStream
where
    T: ToTokens + PartialOrd,
{
    let tp: TokenStream =
        syn::parse_str(std::any::type_name::<T>()).expect("Expected to parse a type");
    let transformations: TokenStream = sanitizers
        .iter()
        .map(|san| match san {
            IntegerSanitizer::Clamp { min, max } => {
                quote!(
                    value = value.clamp(#min, #max);
                )
            }
            IntegerSanitizer::With(token_stream) => {
                let tp = Ident::new(std::any::type_name::<T>(), Span::call_site());
                let tp = quote!(#tp);
                let custom_sanitizer = type_custom_sanitizier_closure(token_stream, tp);
                quote!(
                    value = (#custom_sanitizer)(value);
                )
            }
        })
        .collect();

    quote!(
        fn sanitize(mut value: #tp) -> #tp {
            #transformations
            value
        }
    )
}

fn gen_validate_fn<T>(type_name: &Ident, validators: &[IntegerValidator<T>]) -> TokenStream
where
    T: ToTokens + PartialOrd,
{
    let error_name = gen_error_type_name(type_name);
    let tp: TokenStream =
        syn::parse_str(std::any::type_name::<T>()).expect("Expected to parse a type");

    let validations: TokenStream = validators
        .iter()
        .map(|validator| match validator {
            IntegerValidator::Max(max) => {
                quote!(
                    if val > #max {
                        return Err(#error_name::TooBig);
                    }
                )
            }
            IntegerValidator::Min(min) => {
                quote!(
                    if val < #min {
                        return Err(#error_name::TooSmall);
                    }
                )
            }
            IntegerValidator::With(is_valid_fn) => {
                let tp = Ident::new(std::any::type_name::<T>(), Span::call_site());
                let tp = quote!(&#tp);
                // TODO: rename type_custom_sanitizier_closure, cause it's used only for
                // sanitizers
                let is_valid_fn = type_custom_sanitizier_closure(is_valid_fn, tp);
                quote!(
                    if !(#is_valid_fn)(&val) {
                        return Err(#error_name::Invalid);
                    }
                )
            }
        })
        .collect();

    quote!(
        fn validate(val: #tp) -> Result<(), #error_name> {
            #validations
            Ok(())
        }
    )
}

fn gen_derive() -> TokenStream {
    quote! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    }
}