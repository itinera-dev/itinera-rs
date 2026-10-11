//! `#[rescue_step]` and `#[derive(RescueStep)]`: a rescue step's `run`, with the handles it uses,
//! and how it is built from its requests.

mod derive;

pub(crate) use derive::expand as expand_derive;

use proc_macro2::TokenStream;
use quote::quote;
use syn::ItemImpl;

use super::attempt::Run;
use super::{StepKind, is_asynchronous_run, is_run};
use crate::{block, described};

/// The shorthands for a rescue step's outcomes that `DescribedRescueStep` gives.
const SHORTHANDS: [&str; 3] = ["succeed", "fail", "fail_retriably"];

/// Implements `DescribedRescueStep` for the type of the block. On an error, it implements a
/// stand-in instead, so that the compiler reports that error alone.
pub(crate) fn expand(arguments: TokenStream, mut block: ItemImpl) -> TokenStream {
    let described = describe(arguments, &mut block);
    let import = quote!(
        use ::itinera::step::DescribedRescueStep as _;
    );
    for function in block::functions(&mut block) {
        if is_run(&function) {
            block::unmark(function, &["contributor", "reporter"]);
        }
        block::bring_into_scope(function, &import, &SHORTHANDS);
    }
    match described {
        Ok(described) => quote! {
            #block
            #described
        },
        Err(error) => {
            let error = error.into_compile_error();
            let stand_in = stand_in(&block);
            quote! {
                #block
                #error
                #stand_in
            }
        }
    }
}

fn describe(arguments: TokenStream, block: &mut ItemImpl) -> syn::Result<TokenStream> {
    if !arguments.is_empty() {
        return Err(syn::Error::new_spanned(
            arguments,
            "`#[rescue_step]` takes no arguments: a rescue step's name is made from the name of \
             the step it rescues",
        ));
    }
    if let Some((path, _)) = &block.trait_ {
        return Err(syn::Error::new_spanned(
            path,
            "`#[rescue_step]` goes on the rescue step's own `impl` block, not on a trait \
             implementation",
        ));
    }
    described::refuse_generics(&block.generics, "rescue step")?;
    let Some(run) = block::functions(block)
        .find(is_run)
        .map(|run| Run::read(run, StepKind::Rescue))
        .transpose()?
    else {
        return Err(syn::Error::new_spanned(
            &block.self_ty,
            "a `#[rescue_step]` block holds the rescue step's `run`",
        ));
    };
    Ok(run.expand(&block.self_ty))
}

/// A `DescribedRescueStep` that is never called, in the mode of the block's `run`.
fn stand_in(block: &ItemImpl) -> TokenStream {
    let step = &block.self_ty;
    let mode =
        described::mode(cfg!(feature = "async") && block.items.iter().any(is_asynchronous_run));
    quote! {
        impl ::itinera::step::DescribedRescueStep for #step {
            type Mode = #mode;

            fn rescue<W>(
                _: ::itinera::step::StepDescriptor<W, #mode>,
            ) -> ::itinera::step::StepDescriptor<W, #mode> {
                ::core::unreachable!()
            }
        }
    }
}
