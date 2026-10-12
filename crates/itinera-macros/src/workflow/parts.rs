//! `#[workflow]` on the workflow's `impl` block: the parts of its declaration that are methods,
//! its input adapters, its ID generator and, with the `unstable` feature, its service providers.

mod adapter;
mod attached;
#[cfg(test)]
pub(crate) mod fixtures;
mod id_generator;
mod method;
mod provided;
mod provider;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{ImplItemFn, ItemImpl};

use super::checks;
use crate::{block, described, marker};
use adapter::Adapter;
use id_generator::IdGenerator;
use method::{arguments, by_reference, refuse_other_receivers_and_modes, returned};
use provider::Provider;

/// The attributes that mark a method of the block.
const MARKS: [&str; 3] = ["input_adapter", "id_generator", "provider"];

/// A method of the block marked as a part of the declaration.
enum Part {
    Adapter(Adapter),
    IdGenerator(IdGenerator),
    Provider(Box<Provider>),
}

/// Adds the parts the block holds to the workflow's declaration, through a hidden function that
/// the descriptor of the workflow's struct calls.
///
/// Reading an input adapter takes its parameters' marks, so a mark left on a parameter of another
/// method is reported by the compiler as unknown. After an error, every mark is taken, and the
/// providers are declared unknown, so that the compiler reports that error alone.
pub(super) fn expand(arguments: TokenStream, mut block: ItemImpl) -> TokenStream {
    let described = describe(arguments, &mut block);
    let marks: Vec<&str> = match &described {
        Ok(_) => MARKS.to_vec(),
        Err(_) => MARKS
            .into_iter()
            .chain(adapter::MARKS)
            .chain(provider::MARKS)
            .collect(),
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
            let unknown = provided::unknown(&block);
            quote! {
                #block
                #error
                #unknown
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
    let providers: Vec<&Provider> = parts.iter().filter_map(Part::provider).collect();
    let generator = IdGenerator::only(parts.iter().filter_map(Part::id_generator))?;
    attached::refuse_names_given_twice(&adapters)?;
    attached::refuse_steps_adapted_twice(&adapters)?;
    provided::refuse_names_given_twice(&providers)?;
    provided::refuse_steps_listed_twice(&providers)?;
    provided::refuse_services_provided_twice(&providers)?;
    let workflow = &block.self_ty;
    let functions = adapters.iter().map(|adapter| adapter.function(workflow));
    let descriptors = adapters.iter().map(|adapter| adapter.descriptor());
    let generated = generator.map(|generator| generator.declared(workflow));
    let attached_to_steps = !adapters.is_empty() || providers.iter().any(is_attached);
    let steps = attached_to_steps.then(checks::steps);
    let known = adapters
        .iter()
        .flat_map(|adapter| attached::known(adapter, workflow))
        .chain(
            providers
                .iter()
                .flat_map(|provider| provided::known(provider, workflow)),
        );
    let declared = quote!(declared #(.input_adapter(#descriptors))* #generated);
    let (bound, declared, providers) = if providers.is_empty() {
        (None, declared, None)
    } else {
        (
            Some(quote!(where #workflow: __ItineraProviders<__ItineraMode>)),
            quote!(<#workflow as __ItineraProviders<__ItineraMode>>::__itinera_providers(#declared)),
            Some(provided::declared(workflow, &providers)),
        )
    };
    Ok(quote! {
        const _: () = {
            impl #workflow {
                #[doc(hidden)]
                pub(crate) fn __itinera_parts<__ItineraMode: ::itinera::mode::Mode>(
                    declared: ::itinera::workflow::WorkflowBuilder<#workflow, __ItineraMode>,
                ) -> ::itinera::workflow::WorkflowBuilder<#workflow, __ItineraMode>
                #bound
                {
                    #declared
                }
            }

            #providers

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
        let Some(mark) = marker::at_most_one(
            marks,
            "a method is an input adapter, the ID generator or a service provider",
        )?
        else {
            return Ok(None);
        };
        match marker::name(&mark).as_str() {
            "id_generator" => {
                mark.meta.require_path_only()?;
                IdGenerator::read(function).map(Self::IdGenerator).map(Some)
            }
            "provider" => Provider::read(function, &mark)
                .map(Box::new)
                .map(Self::Provider)
                .map(Some),
            _ => Adapter::read(function, &mark).map(Self::Adapter).map(Some),
        }
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

    fn provider(part: &Option<Self>) -> Option<&Provider> {
        match part {
            Some(Self::Provider(provider)) => Some(provider),
            _ => None,
        }
    }
}

fn is_attached(provider: &&Provider) -> bool {
    !provider.steps().is_empty()
}
