//! `#[step]` and `#[derive(Step)]`: a step's `run`, with the handles it uses, and how it is built
//! from its inputs.

mod attempt;
mod constructor;
mod derive;
mod kind;
mod rescue;
mod service;

pub(crate) use derive::expand as expand_derive;
use kind::StepKind;
pub(crate) use rescue::{expand as expand_rescue, expand_derive as expand_rescue_derive};

use proc_macro2::TokenStream;
use quote::quote;
use syn::{ImplItem, ImplItemFn, ItemImpl};

use attempt::Run;
use constructor::Constructor;
use service::Service;

use crate::block;
use crate::described;

/// The attributes that mark the parameters of `run` and `new`.
const MARKS: [&str; 4] = ["input", "contributor", "reporter", "service"];

/// The shorthands for a step's outcomes that `DescribedStep` gives.
const SHORTHANDS: [&str; 4] = ["succeed", "fail", "skip", "skip_because"];

/// Implements `DescribedStep` for the type of the block, and `FromInputs` from its `new` if it
/// has one. On an error, it implements a stand-in instead, so that the compiler reports that error
/// alone.
pub(crate) fn expand(arguments: TokenStream, mut block: ItemImpl) -> TokenStream {
    let described = describe(arguments, &mut block);
    let import = quote!(
        use ::itinera::step::DescribedStep as _;
    );
    for function in block::functions(&mut block) {
        if is_run(&function) || is_new(&function) {
            block::unmark(function, &MARKS);
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
            "`#[step]` takes no arguments: a step's name is given where a workflow lists it",
        ));
    }
    if let Some((path, _)) = &block.trait_ {
        return Err(syn::Error::new_spanned(
            path,
            "`#[step]` goes on the step's own `impl` block, not on a trait implementation",
        ));
    }
    described::refuse_generics(&block.generics, "step")?;
    let constructor = block::functions(block)
        .find(is_new)
        .map(Constructor::read)
        .transpose()?;
    let Some(run) = block::functions(block)
        .find(is_run)
        .map(|run| Run::read(run, StepKind::Step))
        .transpose()?
    else {
        return Err(syn::Error::new_spanned(
            &block.self_ty,
            "a `#[step]` block holds the step's `run`",
        ));
    };
    let step = &block.self_ty;
    let built_with: Vec<&Service> = constructor.iter().flat_map(Constructor::services).collect();
    let attempt = run.expand(step, &built_with);
    let constructor = constructor.map(|constructor| constructor.expand(step));
    Ok(quote! {
        #constructor
        #attempt
    })
}

fn is_run(function: &&mut ImplItemFn) -> bool {
    function.sig.ident == "run"
}

fn is_new(function: &&mut ImplItemFn) -> bool {
    function.sig.ident == "new"
}

/// A `DescribedStep` that is never called, in the mode of the block's `run`.
fn stand_in(block: &ItemImpl) -> TokenStream {
    let (generics, _, where_clause) = block.generics.split_for_impl();
    let step = &block.self_ty;
    let mode =
        described::mode(cfg!(feature = "async") && block.items.iter().any(is_asynchronous_run));
    quote! {
        impl #generics ::itinera::step::DescribedStep for #step #where_clause {
            type Mode = #mode;

            fn descriptor<W>(
                _: ::itinera::step::StepName,
            ) -> ::itinera::step::StepDescriptor<W, #mode> {
                ::core::unreachable!()
            }
        }
    }
}

fn is_asynchronous_run(item: &ImplItem) -> bool {
    match item {
        ImplItem::Fn(function) => function.sig.ident == "run" && function.sig.asyncness.is_some(),
        _ => false,
    }
}
