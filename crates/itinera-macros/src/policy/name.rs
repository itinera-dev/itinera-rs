//! A policy's name: the type's as written, unless the macro's arguments give one.

use proc_macro2::TokenStream;
use syn::parse::{Parse, ParseStream};
use syn::{LitStr, Token, Type};

use crate::described;

syn::custom_keyword!(name);

/// The macro's arguments: nothing, or `name = "..."`.
struct Arguments {
    name: Option<LitStr>,
}

impl Parse for Arguments {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        if input.is_empty() {
            return Ok(Self { name: None });
        }
        input.parse::<name>()?;
        input.parse::<Token![=]>()?;
        let name = input.parse()?;
        Ok(Self { name: Some(name) })
    }
}

/// The policy's name, from the macro's arguments or from its type.
pub(super) fn read(arguments: TokenStream, attribute: &str, policy: &Type) -> syn::Result<LitStr> {
    let arguments: Arguments =
        syn::parse2(arguments).map_err(|error| misread(&error, attribute))?;
    match (arguments.name, written(policy)) {
        (Some(name), _) => Ok(name),
        (None, Some(written)) => Ok(written),
        (None, None) => Err(syn::Error::new_spanned(
            policy,
            "name this policy, such as `name = \"audit\"`, since its type has no name to take",
        )),
    }
}

fn misread(error: &syn::Error, attribute: &str) -> syn::Error {
    syn::Error::new(
        error.span(),
        format!("`{attribute}` takes nothing, or a name such as `name = \"audit\"`"),
    )
}

/// The type's name as written, such as `Audit` for `audit::Audit`.
fn written(policy: &Type) -> Option<LitStr> {
    let Type::Path(path) = policy else {
        return None;
    };
    let last = path.path.segments.last()?;
    Some(described::named_after(&last.ident))
}
