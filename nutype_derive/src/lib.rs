mod base;
mod common;
mod models;
mod number;
mod parse;
mod string;

use std::{fmt::Debug, str::FromStr};

use models::{InnerType, NumberType, TypeNameAndInnerType};
use number::gen::gen_nutype_for_number;
use parse::parse_type_name_and_inner_type;
use proc_macro2::{Ident, TokenStream};
use quote::ToTokens;
use string::{gen::gen_nutype_for_string, validate::validate_derive_traits};
use syn::Visibility;

#[proc_macro_attribute]
pub fn nutype(
    attrs: proc_macro::TokenStream,
    type_definition: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    expand_nutype(attrs.into(), type_definition.into())
        .unwrap_or_else(|e| syn::Error::to_compile_error(&e))
        .into()
}

fn expand_nutype(
    attrs: TokenStream,
    type_definition: TokenStream,
) -> Result<TokenStream, syn::Error> {
    let TypeNameAndInnerType {
        doc_attrs,
        type_name,
        inner_type,
        vis,
        derive_traits,
    } = parse_type_name_and_inner_type(type_definition)?;

    match inner_type {
        InnerType::String => {
            let meta = string::parse::parse_attributes(attrs)?;
            let traits = validate_derive_traits(&meta, derive_traits)?;
            Ok(gen_nutype_for_string(
                doc_attrs, traits, vis, &type_name, meta,
            ))
        }
        InnerType::Number(tp) => {
            // TODO: inject derive_traits
            let params = NumberParams {
                doc_attrs,
                vis,
                tp,
                type_name,
                attrs,
            };
            match tp {
                NumberType::U8 => parse_number_attrs_and_gen::<u8>(params),
                NumberType::U16 => parse_number_attrs_and_gen::<u16>(params),
                NumberType::U32 => parse_number_attrs_and_gen::<u32>(params),
                NumberType::U64 => parse_number_attrs_and_gen::<u64>(params),
                NumberType::U128 => parse_number_attrs_and_gen::<u128>(params),
                NumberType::Usize => parse_number_attrs_and_gen::<usize>(params),
                NumberType::I8 => parse_number_attrs_and_gen::<i8>(params),
                NumberType::I16 => parse_number_attrs_and_gen::<i16>(params),
                NumberType::I32 => parse_number_attrs_and_gen::<i32>(params),
                NumberType::I64 => parse_number_attrs_and_gen::<i64>(params),
                NumberType::I128 => parse_number_attrs_and_gen::<i128>(params),
                NumberType::Isize => parse_number_attrs_and_gen::<isize>(params),
                NumberType::F32 => parse_number_attrs_and_gen::<f32>(params),
                NumberType::F64 => parse_number_attrs_and_gen::<f64>(params),
            }
        }
    }
}

struct NumberParams {
    doc_attrs: Vec<syn::Attribute>,
    vis: Visibility,
    tp: NumberType,
    type_name: Ident,
    attrs: TokenStream,
}

fn parse_number_attrs_and_gen<T>(params: NumberParams) -> Result<TokenStream, syn::Error>
where
    T: FromStr + ToTokens + PartialOrd + Clone,
    <T as FromStr>::Err: Debug,
{
    let NumberParams {
        doc_attrs,
        vis,
        tp,
        type_name,
        attrs,
    } = params;
    let meta = number::parse::parse_attributes::<T>(attrs)?;
    Ok(gen_nutype_for_number(doc_attrs, vis, tp, &type_name, meta))
}
