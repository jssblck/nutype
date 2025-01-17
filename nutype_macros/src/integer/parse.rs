use std::fmt::Debug;
use std::str::FromStr;

use crate::common::parse::{
    is_comma, parse_nutype_attributes, parse_value_as_number, parse_with_token_stream,
    split_and_parse,
};
use proc_macro2::{Span, TokenStream, TokenTree};

use super::{
    models::{
        IntegerGuard, IntegerRawGuard, IntegerSanitizer, IntegerValidator, SpannedIntegerSanitizer,
        SpannedIntegerValidator,
    },
    validate::validate_number_meta,
};

pub fn parse_attributes<T>(input: TokenStream) -> Result<IntegerGuard<T>, syn::Error>
where
    T: FromStr + PartialOrd + Clone,
    <T as FromStr>::Err: Debug,
{
    parse_raw_attributes(input).and_then(validate_number_meta)
}

fn parse_raw_attributes<T>(input: TokenStream) -> Result<IntegerRawGuard<T>, syn::Error>
where
    T: FromStr,
    <T as FromStr>::Err: Debug,
{
    parse_nutype_attributes(parse_sanitize_attrs, parse_validate_attrs)(input)
}

fn parse_sanitize_attrs<T>(
    stream: TokenStream,
) -> Result<Vec<SpannedIntegerSanitizer<T>>, syn::Error>
where
    T: FromStr,
    <T as FromStr>::Err: Debug,
{
    let tokens: Vec<TokenTree> = stream.into_iter().collect();
    split_and_parse(tokens, is_comma, parse_sanitize_attr)
}

fn parse_sanitize_attr<T>(tokens: Vec<TokenTree>) -> Result<SpannedIntegerSanitizer<T>, syn::Error>
where
    T: FromStr,
    <T as FromStr>::Err: Debug,
{
    let mut token_iter = tokens.iter();
    let token = token_iter.next();
    if let Some(TokenTree::Ident(ident)) = token {
        match ident.to_string().as_ref() {
            "with" => {
                // Preserve the rest as `custom_sanitizer_fn`
                let stream = parse_with_token_stream(token_iter, ident.span())?;
                let span = ident.span();
                let sanitizer = IntegerSanitizer::With(stream);
                Ok(SpannedIntegerSanitizer {
                    span,
                    item: sanitizer,
                })
            }
            unknown_sanitizer => {
                let msg = format!("Unknown sanitizer `{unknown_sanitizer}`");
                let error = syn::Error::new(ident.span(), msg);
                Err(error)
            }
        }
    } else {
        Err(syn::Error::new(Span::call_site(), "Invalid syntax."))
    }
}

fn parse_validate_attrs<T>(
    stream: TokenStream,
) -> Result<Vec<SpannedIntegerValidator<T>>, syn::Error>
where
    T: FromStr,
    <T as FromStr>::Err: Debug,
{
    let tokens: Vec<TokenTree> = stream.into_iter().collect();
    split_and_parse(tokens, is_comma, parse_validate_attr)
}

fn parse_validate_attr<T>(tokens: Vec<TokenTree>) -> Result<SpannedIntegerValidator<T>, syn::Error>
where
    T: FromStr,
    <T as FromStr>::Err: Debug,
{
    let mut token_iter = tokens.into_iter();
    let token = token_iter.next();
    if let Some(TokenTree::Ident(ident)) = token {
        match ident.to_string().as_ref() {
            "min" => {
                let (value, _iter) = parse_value_as_number(token_iter)?;
                let validator = IntegerValidator::Min(value);
                let parsed_validator = SpannedIntegerValidator {
                    span: ident.span(),
                    item: validator,
                };
                Ok(parsed_validator)
            }
            "max" => {
                let (value, _iter) = parse_value_as_number(token_iter)?;
                let validator = IntegerValidator::Max(value);
                let parsed_validator = SpannedIntegerValidator {
                    span: ident.span(),
                    item: validator,
                };
                Ok(parsed_validator)
            }
            "with" => {
                let rest_tokens: Vec<_> = token_iter.collect();
                let stream = parse_with_token_stream(rest_tokens.iter(), ident.span())?;
                let span = ident.span();
                let validator = IntegerValidator::With(stream);
                Ok(SpannedIntegerValidator {
                    span,
                    item: validator,
                })
            }
            unknown_validator => {
                let msg = format!("Unknown validation rule `{unknown_validator}`");
                let error = syn::Error::new(ident.span(), msg);
                Err(error)
            }
        }
    } else {
        Err(syn::Error::new(Span::call_site(), "Invalid syntax."))
    }
}
