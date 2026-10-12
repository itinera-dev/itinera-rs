//! A service's key as a type, which the expansions of a step and of its workflow both name the
//! same way, so that the step's bounds can ask what the workflow's providers of that key make.

use proc_macro2::{Literal, TokenStream};
use quote::{quote, quote_spanned};
use syn::LitStr;

/// The key as the nested tuple of its characters, ending with `()`: `"pa"` is
/// `(Char<'p'>, (Char<'a'>, ()))`.
pub(crate) fn key_type(key: &LitStr) -> TokenStream {
    let span = key.span();
    key.value()
        .chars()
        .rev()
        .fold(quote_spanned!(span=> ()), |rest, character| {
            with_char(character, &rest)
        })
}

fn with_char(character: char, rest: &TokenStream) -> TokenStream {
    let character = Literal::character(character);
    quote!((::itinera::service::Char<#character>, #rest))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_is_the_nested_tuple_of_its_characters() {
        let key: LitStr = syn::parse_quote!("pa");
        assert_eq!(
            key_type(&key).to_string(),
            quote!((
                ::itinera::service::Char<'p'>,
                (::itinera::service::Char<'a'>, ())
            ))
            .to_string()
        );
    }
}
