//! How a policy is built: with `Default`, or by the function of its block marked `#[factory]`.

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned as _;
use syn::{Ident, ImplItemFn, Type};

use crate::block;

/// The function marked `#[factory]`, and whether it may fail.
pub(super) struct Factory {
    function: Ident,
    fallible: bool,
}

impl Factory {
    /// Reads the factory.
    pub(super) fn read(function: &ImplItemFn) -> syn::Result<Self> {
        if let Some(input) = function.sig.inputs.first() {
            return Err(syn::Error::new_spanned(
                input,
                "a policy's factory builds it from nothing: it takes no parameters",
            ));
        }
        if let Some(asynchronous) = function.sig.asyncness {
            return Err(syn::Error::new_spanned(
                asynchronous,
                "a policy's factory is synchronous",
            ));
        }
        Ok(Self {
            function: function.sig.ident.clone(),
            fallible: block::returns_result(&function.sig.output),
        })
    }

    /// The body of `DescribedPolicy::build`.
    pub(super) fn build(factory: Option<&Self>, policy: &Type) -> TokenStream {
        match factory {
            None => quote_spanned! {policy.span()=>
                ::core::result::Result::Ok(<#policy as ::core::default::Default>::default())
            },
            Some(Self {
                function,
                fallible: false,
            }) => quote!(::core::result::Result::Ok(<#policy>::#function())),
            Some(Self {
                function,
                fallible: true,
            }) => quote! {
                let policy = <#policy>::#function()?;
                ::core::result::Result::Ok(policy)
            },
        }
    }
}
