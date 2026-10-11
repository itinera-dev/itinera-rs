//! Data requested by key: a step's inputs, the data a hook requests from its step or its
//! workflow, and the fields of journey data. Whether a datum is optional is read from its type.

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Attribute, GenericArgument, Ident, LitStr, Meta, PathArguments, Token, Type, TypePath};

use crate::{described, marker};

syn::custom_keyword!(required);

/// One datum requested by key, and the type of its value.
pub(crate) struct Datum {
    key: LitStr,
    request: Request,
}

/// How a datum is requested, which its type decides.
pub(crate) enum Request {
    /// The key must be present, and its value has this type.
    Required(Type),
    /// The key may be absent: the parameter or field is an `Option` of this type.
    Optional(Type),
}

/// What an attribute naming a datum says: its key, unless it is named after its parameter or
/// field, and whether a datum whose value is itself an `Option` is required.
pub(crate) struct Arguments {
    key: Option<LitStr>,
    required: Option<required>,
}

impl Parse for Arguments {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let key = if input.peek(LitStr) {
            Some(input.parse()?)
        } else {
            None
        };
        if key.is_some() && !input.is_empty() {
            input.parse::<Token![,]>()?;
        }
        let required = if input.is_empty() {
            None
        } else {
            Some(input.parse()?)
        };
        Ok(Self { key, required })
    }
}

impl Arguments {
    /// The arguments of a datum named by separate attributes, such as `#[key("note")]` and
    /// `#[required]`, where `required` is the span of the mark that makes it required.
    pub(crate) fn new(key: Option<LitStr>, required: Option<Span>) -> Self {
        Self {
            key,
            required: required.map(self::required),
        }
    }

    /// What an attribute naming a datum says, written as `#[input]` or with arguments, such as
    /// `#[input("amount", required)]`.
    pub(crate) fn of(mark: &Attribute) -> syn::Result<Self> {
        match &mark.meta {
            Meta::Path(_) => Ok(Self {
                key: None,
                required: None,
            }),
            _ => mark.parse_args().map_err(|error| misread(&error, mark)),
        }
    }

    /// The datum these arguments name, for a parameter or field of this name and type.
    pub(crate) fn datum(self, name: Option<&Ident>, ty: &Type) -> syn::Result<Datum> {
        let key = match (self.key, name) {
            (Some(key), _) => key,
            (None, Some(name)) => described::named_after(name),
            (None, None) => {
                return Err(syn::Error::new_spanned(
                    ty,
                    "name the key, as in `(\"amount\")`, since this one has no name to take it from",
                ));
            }
        };
        let request = match (self.required, option_of(ty)) {
            (None, Some(inner)) => Request::Optional(inner.clone()),
            (None, None) | (Some(_), Some(_)) => Request::Required(ty.clone()),
            (Some(required), None) => {
                return Err(syn::Error::new(
                    required.span,
                    "only a datum of an `Option` type is marked `required`: any other type is \
                     required already",
                ));
            }
        };
        Ok(Datum { key, request })
    }
}

fn misread(error: &syn::Error, mark: &Attribute) -> syn::Error {
    let name = marker::name(mark);
    syn::Error::new(
        error.span(),
        format!(
            "`#[{name}]` takes a key, `required`, or both, such as `#[{name}(\"note\", required)]`"
        ),
    )
}

impl Datum {
    /// The datum's key.
    pub(crate) fn key(&self) -> &LitStr {
        &self.key
    }

    /// How the datum is requested.
    pub(crate) fn request(&self) -> &Request {
        &self.request
    }

    /// The token naming the datum, such as `Input::<i64>::new("amount")`.
    fn token(&self) -> TokenStream {
        let key = &self.key;
        match &self.request {
            Request::Required(ty) => quote!(::itinera::step::Input::<#ty>::new(#key)),
            Request::Optional(ty) => quote!(::itinera::step::OptionalInput::<#ty>::new(#key)),
        }
    }

    /// The call that declares the datum on needs, for data named as `input` or `from_step`, or
    /// any other prefix: `.input(&token)` or `.optional_input(&token)`.
    pub(crate) fn declared(&self, prefix: &str) -> TokenStream {
        let token = self.token();
        let method = self.method(prefix);
        quote!(.#method(&#token))
    }

    /// The expression that takes the datum from what was resolved: `got.input(&token)?`.
    pub(crate) fn taken(&self, got: &Ident, prefix: &str) -> TokenStream {
        let token = self.token();
        let method = self.method(prefix);
        quote!(#got.#method(&#token)?)
    }

    fn method(&self, prefix: &str) -> Ident {
        let name = match self.request {
            Request::Required(_) => prefix.to_owned(),
            Request::Optional(_) => format!("optional_{prefix}"),
        };
        Ident::new(&name, self.key.span())
    }
}

/// The type an `Option` holds, if this type is written as one.
pub(crate) fn option_of(ty: &Type) -> Option<&Type> {
    let Type::Path(TypePath {
        qself: None, path, ..
    }) = ty
    else {
        return None;
    };
    let last = path.segments.last()?;
    let PathArguments::AngleBracketed(arguments) = &last.arguments else {
        return None;
    };
    match (last.ident == "Option", arguments.args.first()) {
        (true, Some(GenericArgument::Type(inner))) if arguments.args.len() == 1 => Some(inner),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use syn::Field;
    use syn::parse::Parser as _;

    use super::*;

    fn declared(field: &str) -> syn::Result<String> {
        let field = Field::parse_named.parse_str(field)?;
        let datum = Arguments::of(&field.attrs[0])?.datum(field.ident.as_ref(), &field.ty)?;
        Ok(compact(&datum.declared("input").to_string()))
    }

    /// The code without its spaces, which tokens place differently from the way code is written.
    fn compact(code: &str) -> String {
        code.split_whitespace().collect()
    }

    #[rstest]
    #[case::named_after_its_field(
        "#[input] amount: i64",
        r#".input(&::itinera::step::Input::<i64>::new("amount"))"#
    )]
    #[case::with_its_own_key(
        r#"#[input("total")] amount: i64"#,
        r#".input(&::itinera::step::Input::<i64>::new("total"))"#
    )]
    #[case::named_after_a_raw_identifier(
        "#[input] r#type: i64",
        r#".input(&::itinera::step::Input::<i64>::new("type"))"#
    )]
    #[case::optional(
        "#[input] note: Option<String>",
        r#".optional_input(&::itinera::step::OptionalInput::<String>::new("note"))"#
    )]
    #[case::optional_by_its_full_path(
        "#[input] note: ::core::option::Option<String>",
        r#".optional_input(&::itinera::step::OptionalInput::<String>::new("note"))"#
    )]
    #[case::optional_after_a_trailing_comma(
        r#"#[input("note",)] note: Option<String>"#,
        r#".optional_input(&::itinera::step::OptionalInput::<String>::new("note"))"#
    )]
    #[case::required_though_its_value_is_an_option(
        r#"#[input("note", required)] note: Option<String>"#,
        r#".input(&::itinera::step::Input::<Option<String>>::new("note"))"#
    )]
    #[case::required_and_named_after_its_field(
        "#[input(required)] note: Option<String>",
        r#".input(&::itinera::step::Input::<Option<String>>::new("note"))"#
    )]
    fn a_datum_is_declared_by_its_key_and_by_whether_its_type_is_an_option(
        #[case] field: &str,
        #[case] expected: &str,
    ) {
        assert_eq!(declared(field).unwrap(), compact(expected));
    }

    #[rstest]
    #[case::required_on_a_type_that_is_required_already(
        "#[input(required)] amount: i64",
        "only a datum of an `Option` type is marked `required`: any other type is required already"
    )]
    #[case::an_unknown_argument(
        r#"#[input("note", optional)] note: Option<String>"#,
        r#"`#[input]` takes a key, `required`, or both, such as `#[input("note", required)]`"#
    )]
    fn a_datum_is_refused_when_its_attribute_says_what_it_cannot(
        #[case] field: &str,
        #[case] refused: &str,
    ) {
        assert_eq!(declared(field).unwrap_err().to_string(), refused);
    }
}
