//! `#[role]`: a workflow's implementation of a role, which makes the workflow provide it.

use proc_macro2::TokenStream;
use quote::quote;
use syn::ItemImpl;

/// Keeps the implementation, and implements `Provides` for the role it implements.
pub(crate) fn expand(arguments: TokenStream, block: ItemImpl) -> TokenStream {
    match provided(arguments, &block) {
        Ok(provided) => quote! {
            #block
            #provided
        },
        Err(error) => {
            let error = error.into_compile_error();
            quote! {
                #block
                #error
            }
        }
    }
}

fn provided(arguments: TokenStream, block: &ItemImpl) -> syn::Result<TokenStream> {
    if !arguments.is_empty() {
        return Err(syn::Error::new_spanned(
            arguments,
            "`#[role]` takes no arguments",
        ));
    }
    let Some((role, _)) = &block.trait_ else {
        return Err(syn::Error::new_spanned(
            &block.self_ty,
            "`#[role]` goes on a workflow's implementation of a role, such as \
             `impl Notifier for Orders`",
        ));
    };
    if let Some(parameter) = block.generics.params.first() {
        return Err(syn::Error::new_spanned(
            parameter,
            "`#[role]` goes on an implementation for one workflow: a generic one implements \
             `Provides` by hand",
        ));
    }
    let workflow = &block.self_ty;
    Ok(quote! {
        impl ::itinera::workflow::Provides<dyn #role> for #workflow {
            fn role(&self) -> &(dyn #role + 'static) {
                self
            }
        }
    })
}
