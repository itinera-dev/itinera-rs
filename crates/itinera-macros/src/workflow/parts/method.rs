//! What the methods of the workflow's `impl` block that are parts of its declaration share: how
//! they take the workflow, their mode, where their return type is, and their marks' arguments.

use proc_macro2::{Span, TokenStream};
use syn::spanned::Spanned as _;
use syn::{ImplItemFn, ReturnType};

/// Refuses a method that does not take the workflow as `&self`, or that is asynchronous.
pub(super) fn refuse_other_receivers_and_modes(function: &ImplItemFn) -> syn::Result<()> {
    if !function.sig.receiver().is_some_and(by_reference) {
        return Err(syn::Error::new_spanned(
            &function.sig,
            "an input adapter or ID generator takes the workflow as `&self`",
        ));
    }
    match function.sig.asyncness {
        Some(asynchronous) => Err(syn::Error::new_spanned(
            asynchronous,
            "an input adapter or ID generator is synchronous",
        )),
        None => Ok(()),
    }
}

pub(super) fn by_reference(receiver: &syn::Receiver) -> bool {
    matches!(receiver.kind, syn::ReceiverKind::Reference(_, _, None))
}

/// The tokens inside a mark's parentheses, or none when it has none; `misread` says what the
/// mark takes.
pub(super) fn arguments(mark: &syn::Attribute, misread: &str) -> syn::Result<TokenStream> {
    match &mark.meta {
        syn::Meta::Path(_) => Ok(TokenStream::new()),
        syn::Meta::List(list) => Ok(list.tokens.clone()),
        syn::Meta::NameValue(_) => Err(syn::Error::new_spanned(mark, misread)),
    }
}

/// Where the method's return type is written, or its name when it has none, so that a type it
/// does not return is reported there.
pub(super) fn returned(function: &ImplItemFn) -> Span {
    match &function.sig.output {
        ReturnType::Type(_, ty) => ty.span(),
        ReturnType::Default => function.sig.ident.span(),
    }
}
