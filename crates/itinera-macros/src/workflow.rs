//! `#[workflow]`: a workflow declared on its struct, with a `#[with_step]` line per step, and its
//! input adapters and ID generator on its `impl` block.

mod arguments;
mod builder;
mod checks;
mod declaration;
mod parts;
mod services;
mod with_step;

use proc_macro2::TokenStream;
use syn::Item;

/// Declares the workflow its struct describes, or adds the parts its `impl` block holds.
pub(crate) fn expand(arguments: TokenStream, item: Item) -> TokenStream {
    match item {
        Item::Struct(declared) => declaration::expand(arguments, declared),
        Item::Impl(block) => parts::expand(arguments, block),
        other => {
            let error = syn::Error::new_spanned(
                &other,
                "`#[workflow]` goes on a workflow's struct, or on its `impl` block",
            )
            .into_compile_error();
            quote::quote! {
                #other
                #error
            }
        }
    }
}

/// Lists a list of settings such as `policies = [Audit, Alarm]`, from what follows its key.
fn list<T: syn::parse::Parse>(
    meta: &syn::meta::ParseNestedMeta<'_>,
) -> syn::Result<syn::punctuated::Punctuated<T, syn::Token![,]>> {
    let content;
    let value = meta.value()?;
    syn::bracketed!(content in value);
    content.parse_terminated(T::parse, syn::Token![,])
}

/// Refuses a setting given twice.
fn once<T>(
    setting: &mut Option<T>,
    value: T,
    meta: &syn::meta::ParseNestedMeta<'_>,
) -> syn::Result<()> {
    if setting.is_some() {
        return Err(meta.error("this setting is given once"));
    }
    *setting = Some(value);
    Ok(())
}

/// The key of a setting, such as `policies`, or nothing when it is not a single name.
fn key(meta: &syn::meta::ParseNestedMeta<'_>) -> String {
    meta.path
        .get_ident()
        .map(syn::Ident::to_string)
        .unwrap_or_default()
}

/// The first item that shares something, such as its name, with an item before it.
fn repeated<T>(items: &[T], shares: fn(&T, &T) -> bool) -> Option<&T> {
    items
        .iter()
        .enumerate()
        .find_map(|(index, item)| given_before(items, index, item, shares))
}

/// The item, when an item before this index shares something with it.
fn given_before<'a, T>(
    items: &[T],
    index: usize,
    item: &'a T,
    shares: fn(&T, &T) -> bool,
) -> Option<&'a T> {
    items
        .iter()
        .take(index)
        .any(|earlier| shares(earlier, item))
        .then_some(item)
}
