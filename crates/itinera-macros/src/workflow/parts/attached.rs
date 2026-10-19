//! The steps the input adapters are attached to: each adapted once, and each one the workflow
//! lists or the rescue step of one.

use proc_macro2::TokenStream;
use quote::quote_spanned;
use syn::{LitStr, Type};

use super::adapter::Adapter;
use crate::workflow::repeated;

/// Refuses an adapter named like another.
pub(super) fn refuse_names_given_twice(adapters: &[&Adapter]) -> syn::Result<()> {
    match repeated(adapters, share_a_name) {
        Some(adapter) => Err(syn::Error::new_spanned(
            adapter.name(),
            format!(
                "duplicate adapter name: {}, which another input adapter of the workflow has",
                adapter.name().value()
            ),
        )),
        None => Ok(()),
    }
}

fn share_a_name(one: &&Adapter, other: &&Adapter) -> bool {
    one.shares_name_with(other)
}

/// Refuses a step that two adapters, or one adapter twice, are attached to.
pub(super) fn refuse_steps_adapted_twice(adapters: &[&Adapter]) -> syn::Result<()> {
    let attached: Vec<(&Adapter, &LitStr)> = adapters
        .iter()
        .flat_map(|adapter| adapter.attachments())
        .collect();
    let twice = attached
        .iter()
        .enumerate()
        .find_map(|(index, &(adapter, step))| adapted_before(&attached, index, adapter, step));
    match twice {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

/// The error for the step at this index, when an earlier attachment adapts it already.
fn adapted_before(
    attached: &[(&Adapter, &LitStr)],
    index: usize,
    adapter: &Adapter,
    step: &LitStr,
) -> Option<syn::Error> {
    let (first, _) = attached
        .iter()
        .take(index)
        .find(|attachment| adapts(attachment, step))?;
    let message = if first.shares_name_with(adapter) {
        format!(
            "step adapted twice: {}, which this input adapter lists already",
            step.value()
        )
    } else {
        format!(
            "step adapted twice: {}, by `{}` and `{}`",
            step.value(),
            first.name().value(),
            adapter.name().value()
        )
    };
    Some(syn::Error::new_spanned(step, message))
}

fn adapts((_, adapted): &&(&Adapter, &LitStr), step: &LitStr) -> bool {
    adapted.value() == step.value()
}

/// The checks, run as the program compiles, that each step the adapter is attached to is one the
/// workflow lists or the rescue step of one.
pub(super) fn known<'a>(
    adapter: &'a Adapter,
    workflow: &'a Type,
) -> impl Iterator<Item = TokenStream> + 'a {
    adapter
        .attachments()
        .map(move |(adapter, step)| listed(adapter, step, workflow))
}

fn listed(adapter: &Adapter, step: &LitStr, workflow: &Type) -> TokenStream {
    let message = format!(
        "input adapter for unknown step: {} on {}",
        adapter.name().value(),
        step.value()
    );
    quote_spanned! {step.span()=>
        const _: () = ::core::assert!(
            __itinera_steps::listed(<#workflow>::__ITINERA_STEPS, #step),
            #message
        );
    }
}
