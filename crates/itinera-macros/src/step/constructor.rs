//! A step's own `new`, whose parameters are its inputs, and which may refuse to build it.

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::spanned::Spanned as _;
use syn::{FnArg, ImplItemFn, Pat, ReturnType, Type};

use crate::block;
use crate::datum::{Arguments, Datum};
use crate::marker;

/// The inputs `new` takes, in order, whether it may fail, and where its return type is written.
pub(super) struct Constructor {
    inputs: Vec<Datum>,
    fallible: bool,
    output: Span,
}

impl Constructor {
    /// Reads `new`'s inputs, and removes their attributes.
    pub(super) fn read(new: &mut ImplItemFn) -> syn::Result<Self> {
        let inputs = new
            .sig
            .inputs
            .iter_mut()
            .map(input)
            .collect::<syn::Result<_>>()?;
        Ok(Self {
            inputs,
            fallible: block::returns_result(&new.sig.output),
            output: returned(&new.sig.output),
        })
    }

    /// The step's `FromInputs`, which calls `new`.
    pub(super) fn expand(&self, step: &Type) -> TokenStream {
        let got = format_ident!("got");
        let declared = self.inputs.iter().map(|input| input.declared("input"));
        let taken = self.inputs.iter().map(|input| input.taken(&got, "input"));
        let new = quote_spanned!(self.output=> <#step>::new(#(#taken),*));
        let built = if self.fallible {
            quote_spanned!(self.output=> {
                let step = #new?;
                ::core::result::Result::Ok(step)
            })
        } else {
            quote_spanned!(self.output=> ::core::result::Result::Ok(#new))
        };
        quote! {
            impl ::itinera::step::FromInputs for #step {
                fn needs() -> ::itinera::step::StepNeeds {
                    ::itinera::step::StepNeeds::new() #(#declared)*
                }

                fn from_inputs<__ItineraMode>(
                    #got: &mut ::itinera::step::Resolved<'_, __ItineraMode>,
                ) -> ::core::result::Result<Self, ::itinera::error::Error> {
                    #built
                }
            }
        }
    }
}

/// Where `new`'s return type is written, so that a step `new` does not build is reported there.
fn returned(output: &ReturnType) -> Span {
    match output {
        ReturnType::Type(_, ty) => ty.span(),
        ReturnType::Default => Span::call_site(),
    }
}

/// Reads the input a parameter of `new` is, and removes its attribute.
fn input(argument: &mut FnArg) -> syn::Result<Datum> {
    let FnArg::Typed(parameter) = argument else {
        return Err(syn::Error::new_spanned(
            argument,
            "a step's `new` builds the step from its inputs alone, without `self`",
        ));
    };
    let mark = marker::single(
        marker::take(&mut parameter.attrs, &["input"]),
        &parameter.pat,
        "every parameter of a step's `new` is an `#[input]`",
    )?;
    let name = match &*parameter.pat {
        Pat::Ident(pattern) => Some(&pattern.ident),
        _ => None,
    };
    Arguments::of(&mark)?.datum(name, &parameter.ty)
}
