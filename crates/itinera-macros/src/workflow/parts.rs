//! `#[workflow]` on the workflow's `impl` block: the parts of its declaration that are methods,
//! its input adapters and its ID generator.

mod adapter;
mod attached;
mod id_generator;

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::spanned::Spanned as _;
use syn::{ImplItemFn, ItemImpl, ReturnType};

use super::checks;
use crate::{block, described, marker};
use adapter::Adapter;
use id_generator::IdGenerator;

/// The attributes that mark a method of the block.
const MARKS: [&str; 2] = ["input_adapter", "id_generator"];

/// A method of the block marked as a part of the declaration.
enum Part {
    Adapter(Adapter),
    IdGenerator(IdGenerator),
}

/// Adds the parts the block holds to the workflow's declaration, through a hidden function that
/// the descriptor of the workflow's struct calls.
///
/// Reading an input adapter takes its parameters' marks, so a mark left on a parameter of another
/// method is reported by the compiler as unknown. After an error, every mark is taken, so that the
/// compiler reports that error alone.
pub(super) fn expand(arguments: TokenStream, mut block: ItemImpl) -> TokenStream {
    let described = describe(arguments, &mut block);
    let marks: Vec<&str> = match &described {
        Ok(_) => MARKS.to_vec(),
        Err(_) => MARKS.into_iter().chain(adapter::MARKS).collect(),
    };
    for function in block::functions(&mut block) {
        block::unmark(function, &marks);
    }
    match described {
        Ok(parts) => quote! {
            #block
            #parts
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

fn describe(arguments: TokenStream, block: &mut ItemImpl) -> syn::Result<TokenStream> {
    if !arguments.is_empty() {
        return Err(syn::Error::new_spanned(
            arguments,
            "`#[workflow]` on an `impl` block takes no arguments: they go on the workflow's struct",
        ));
    }
    if let Some((path, _)) = &block.trait_ {
        return Err(syn::Error::new_spanned(
            path,
            "`#[workflow]` goes on the workflow's own `impl` block, not on a trait implementation",
        ));
    }
    described::refuse_generics(&block.generics, "workflow")?;
    let parts = block::functions(block)
        .map(Part::read)
        .collect::<syn::Result<Vec<_>>>()?;
    let adapters: Vec<&Adapter> = parts.iter().filter_map(Part::adapter).collect();
    let generator = IdGenerator::only(parts.iter().filter_map(Part::id_generator))?;
    attached::refuse_names_given_twice(&adapters)?;
    attached::refuse_steps_adapted_twice(&adapters)?;
    let workflow = &block.self_ty;
    let functions = adapters.iter().map(|adapter| adapter.function(workflow));
    let descriptors = adapters.iter().map(|adapter| adapter.descriptor());
    let generated = generator.map(|generator| generator.declared(workflow));
    let steps = (!adapters.is_empty()).then(checks::steps);
    let known = adapters
        .iter()
        .flat_map(|adapter| attached::known(adapter, workflow));
    Ok(quote! {
        const _: () = {
            impl #workflow {
                #[doc(hidden)]
                pub(crate) fn __itinera_parts<__ItineraMode: ::itinera::mode::Mode>(
                    declared: ::itinera::workflow::WorkflowBuilder<#workflow, __ItineraMode>,
                ) -> ::itinera::workflow::WorkflowBuilder<#workflow, __ItineraMode> {
                    declared
                        #(.input_adapter(#descriptors))*
                        #generated
                }
            }

            const _: fn() = || {
                fn described<W: ::itinera::workflow::Workflow>() {}
                described::<#workflow>();
            };

            #(#functions)*

            #steps

            #(#known)*
        };
    })
}

impl Part {
    /// Reads what the function is marked as, if anything.
    fn read(function: &mut ImplItemFn) -> syn::Result<Option<Self>> {
        let marks = marker::found(&function.attrs, &MARKS)
            .into_iter()
            .cloned()
            .collect();
        let Some(mark) =
            marker::at_most_one(marks, "a method is an input adapter or the ID generator")?
        else {
            return Ok(None);
        };
        if marker::name(&mark) == "id_generator" {
            mark.meta.require_path_only()?;
            return IdGenerator::read(function).map(Self::IdGenerator).map(Some);
        }
        Adapter::read(function, &mark).map(Self::Adapter).map(Some)
    }

    fn adapter(part: &Option<Self>) -> Option<&Adapter> {
        match part {
            Some(Self::Adapter(adapter)) => Some(adapter),
            _ => None,
        }
    }

    fn id_generator(part: &Option<Self>) -> Option<&IdGenerator> {
        match part {
            Some(Self::IdGenerator(generator)) => Some(generator),
            _ => None,
        }
    }
}

/// Refuses a method that does not take the workflow as `&self`, or that is asynchronous.
fn refuse_other_receivers_and_modes(function: &ImplItemFn) -> syn::Result<()> {
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

fn by_reference(receiver: &syn::Receiver) -> bool {
    matches!(receiver.kind, syn::ReceiverKind::Reference(_, _, None))
}

/// Where the method's return type is written, or its name when it has none, so that a type it
/// does not return is reported there.
fn returned(function: &ImplItemFn) -> Span {
    match &function.sig.output {
        ReturnType::Type(_, ty) => ty.span(),
        ReturnType::Default => function.sig.ident.span(),
    }
}
