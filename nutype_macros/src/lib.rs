mod common;
mod float;
mod integer;
mod string;
mod utils;

use std::collections::HashSet;
use std::{fmt::Debug, str::FromStr};

use common::models::{
    Attributes, DeriveTrait, FloatInnerType, Guard, InnerType, IntegerInnerType, NewUnchecked,
    NewtypeMeta, SpannedDeriveTrait, SpannedItem, TypeName,
};
use common::parse::meta::parse_meta;
use float::validate::validate_float_derive_traits;
use integer::validate::validate_integer_derive_traits;
use proc_macro2::TokenStream;
use quote::ToTokens;
use string::StringNewtype;
use syn::{Attribute, Visibility};

#[proc_macro_attribute]
pub fn nutype(
    attrs: proc_macro::TokenStream,
    type_definition: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    expand_nutype(attrs.into(), type_definition.into())
        .unwrap_or_else(|e| syn::Error::to_compile_error(&e))
        .into()
}

// Functions
// * parse_attributes(attrs: TokenStream) ->   Attributes<G> { guard, new_unchecked, maybe_default_value  }
// * validate(&gard, derive_traits) -> traits
// * generate(
//       doc_attrs,
//       traits,
//       vis,
//       &type_name,
//       guard,
//       new_unchecked,
//       maybe_default_value,
//   ) -> TokenStream

struct GenerateParams<T, G> {
    pub doc_attrs: Vec<Attribute>,
    pub traits: HashSet<T>,
    pub vis: syn::Visibility,
    pub type_name: TypeName,
    pub guard: G,
    pub new_unchecked: NewUnchecked,
    pub maybe_default_value: Option<TokenStream>,
}

trait Newtype {
    type Sanitizer;
    type Validator;
    type TypedTrait;

    fn parse_attributes(
        attrs: TokenStream,
    ) -> Result<Attributes<Guard<Self::Sanitizer, Self::Validator>>, syn::Error>;

    fn validate(
        guard: &Guard<Self::Sanitizer, Self::Validator>,
        derive_traits: Vec<SpannedItem<DeriveTrait>>,
    ) -> Result<HashSet<Self::TypedTrait>, syn::Error>;

    fn generate(
        params: GenerateParams<Self::TypedTrait, Guard<Self::Sanitizer, Self::Validator>>,
    ) -> TokenStream;

    fn expand(
        attrs: TokenStream,
        doc_attrs: Vec<Attribute>,
        type_name: TypeName,
        vis: syn::Visibility,
        derive_traits: Vec<SpannedDeriveTrait>,
    ) -> Result<TokenStream, syn::Error> {
        let Attributes {
            guard,
            new_unchecked,
            maybe_default_value,
        } = Self::parse_attributes(attrs)?;
        let traits = Self::validate(&guard, derive_traits)?;
        let generated_output = Self::generate(GenerateParams {
            doc_attrs,
            traits,
            vis,
            type_name,
            guard,
            new_unchecked,
            maybe_default_value,
        });
        Ok(generated_output)
    }
}

fn expand_nutype(
    attrs: TokenStream,
    type_definition: TokenStream,
) -> Result<TokenStream, syn::Error> {
    let NewtypeMeta {
        doc_attrs,
        type_name,
        inner_type,
        vis,
        derive_traits,
    } = parse_meta(type_definition)?;
    match inner_type {
        InnerType::String => StringNewtype::expand(attrs, doc_attrs, type_name, vis, derive_traits),
        InnerType::Integer(tp) => {
            let params = NumberParams {
                doc_attrs,
                vis,
                tp,
                type_name,
                attrs,
                derive_traits,
            };
            match tp {
                IntegerInnerType::U8 => parse_integer_attrs_and_gen::<u8>(params),
                IntegerInnerType::U16 => parse_integer_attrs_and_gen::<u16>(params),
                IntegerInnerType::U32 => parse_integer_attrs_and_gen::<u32>(params),
                IntegerInnerType::U64 => parse_integer_attrs_and_gen::<u64>(params),
                IntegerInnerType::U128 => parse_integer_attrs_and_gen::<u128>(params),
                IntegerInnerType::Usize => parse_integer_attrs_and_gen::<usize>(params),
                IntegerInnerType::I8 => parse_integer_attrs_and_gen::<i8>(params),
                IntegerInnerType::I16 => parse_integer_attrs_and_gen::<i16>(params),
                IntegerInnerType::I32 => parse_integer_attrs_and_gen::<i32>(params),
                IntegerInnerType::I64 => parse_integer_attrs_and_gen::<i64>(params),
                IntegerInnerType::I128 => parse_integer_attrs_and_gen::<i128>(params),
                IntegerInnerType::Isize => parse_integer_attrs_and_gen::<isize>(params),
            }
        }
        InnerType::Float(tp) => {
            let params = NumberParams {
                doc_attrs,
                vis,
                tp,
                type_name,
                attrs,
                derive_traits,
            };
            match tp {
                FloatInnerType::F32 => parse_float_attrs_and_gen::<f32>(params),
                FloatInnerType::F64 => parse_float_attrs_and_gen::<f64>(params),
            }
        }
    }
}

struct NumberParams<NumberType> {
    doc_attrs: Vec<syn::Attribute>,
    vis: Visibility,
    tp: NumberType,
    type_name: TypeName,
    attrs: TokenStream,
    derive_traits: Vec<SpannedDeriveTrait>,
}

fn parse_integer_attrs_and_gen<T>(
    params: NumberParams<IntegerInnerType>,
) -> Result<TokenStream, syn::Error>
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
        derive_traits,
    } = params;
    let Attributes {
        guard,
        new_unchecked,
        maybe_default_value,
    } = integer::parse::parse_attributes::<T>(attrs)?;
    let traits = validate_integer_derive_traits(derive_traits, guard.has_validation())?;
    Ok(integer::gen::gen_nutype_for_integer(
        doc_attrs,
        vis,
        tp,
        &type_name,
        guard,
        traits,
        new_unchecked,
        maybe_default_value,
    ))
}

fn parse_float_attrs_and_gen<T>(
    params: NumberParams<FloatInnerType>,
) -> Result<TokenStream, syn::Error>
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
        derive_traits,
    } = params;
    let Attributes {
        guard,
        new_unchecked,
        maybe_default_value,
    } = float::parse::parse_attributes::<T>(attrs)?;
    let traits = validate_float_derive_traits(derive_traits, &guard)?;
    Ok(float::gen::gen_nutype_for_float(
        doc_attrs,
        vis,
        tp,
        &type_name,
        guard,
        traits,
        new_unchecked,
        maybe_default_value,
    ))
}
