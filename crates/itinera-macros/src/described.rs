//! What every item the macros describe shares: it is not generic, and it runs in one mode.

use proc_macro2::TokenStream;
use quote::quote;
use syn::Generics;

/// Refuses a generic type, which the macros do not describe.
pub(crate) fn refuse_generics(generics: &Generics, what: &str) -> syn::Result<()> {
    match generics.params.first() {
        None => Ok(()),
        Some(parameter) => Err(syn::Error::new_spanned(
            parameter,
            format!("a generic {what} is described with the builder, not with the macros"),
        )),
    }
}

/// The path of a mode: `Asynchronous`, or `Synchronous`.
pub(crate) fn mode(asynchronous: bool) -> TokenStream {
    if asynchronous {
        quote!(::itinera::mode::Asynchronous)
    } else {
        quote!(::itinera::mode::Synchronous)
    }
}
