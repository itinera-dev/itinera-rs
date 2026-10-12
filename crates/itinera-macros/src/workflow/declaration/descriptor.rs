//! The workflow's implementation of `Workflow`, whose descriptor is built the first time it is
//! asked for, from the arguments, the steps, and the parts the workflow's `impl` block adds.

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned as _;
use syn::{Ident, LitStr, Type};

use crate::local::local;
use crate::workflow::arguments::Arguments;
use crate::workflow::mode;
use crate::workflow::with_step::WithStep;

pub(super) fn descriptor(
    workflow: &Ident,
    name: &LitStr,
    arguments: &Arguments,
    steps: &[WithStep],
) -> TokenStream {
    let chosen = mode::of(steps);
    let flagged = mode::flagged();
    let mode = &mode::named(workflow);
    let steps = steps.iter().map(|step| step.declared(workflow, mode));
    let policies = arguments
        .policies
        .iter()
        .map(|policy| attached(policy, workflow));
    let reporters = arguments.reporters.iter().map(listed);
    let declared = local("declared");
    let async_reporters = arguments
        .async_reporters
        .iter()
        .map(|reporter| listed_async(reporter, mode, &declared));
    let modes = mode::declared(workflow);
    quote! {
        impl ::itinera::workflow::Workflow for #workflow {
            type Mode = #chosen;

            fn descriptor() -> &'static ::itinera::workflow::WorkflowDescriptor<Self, Self::Mode> {
                static DESCRIPTOR: ::std::sync::LazyLock<
                    ::itinera::workflow::WorkflowDescriptor<#workflow, #mode>,
                > = ::std::sync::LazyLock::new(__itinera_declared);
                &DESCRIPTOR
            }
        }

        fn __itinera_declared() -> ::itinera::workflow::WorkflowDescriptor<#workflow, #mode> {
            let #declared = <#mode as __ItineraStart>::__itinera_builder(#name)
                #(#steps)*
                #(#policies)*
                #(#reporters)*;
            #(let #declared = #async_reporters;)*
            match <#workflow>::__itinera_parts(#declared).build() {
                ::core::result::Result::Ok(descriptor) => descriptor,
                ::core::result::Result::Err(violations) => ::core::panic!(
                    "the workflow `{}` cannot be declared: {violations}",
                    #name
                ),
            }
        }

        trait __ItineraParts {
            fn __itinera_parts<__ItineraMode: ::itinera::mode::Mode>(
                declared: ::itinera::workflow::WorkflowBuilder<#workflow, __ItineraMode>,
            ) -> ::itinera::workflow::WorkflowBuilder<#workflow, __ItineraMode> {
                declared
            }
        }

        impl __ItineraParts for #workflow {}

        #flagged
        #modes
    }
}

fn attached(policy: &Type, workflow: &Ident) -> TokenStream {
    quote_spanned! {policy.span()=>
        .policy(<#policy as ::itinera::policy::WorkflowPolicy<#workflow>>::descriptor())
    }
}

fn listed(reporter: &Type) -> TokenStream {
    quote_spanned!(reporter.span()=> .reporter::<#reporter>())
}

/// The asynchronous reporter listed on what is declared, through a trait that only a workflow
/// in the asynchronous mode takes, so that a synchronous one is refused at the reporter.
fn listed_async(reporter: &Type, mode: &TokenStream, declared: &Ident) -> TokenStream {
    quote_spanned! {reporter.span()=>
        <#reporter as __ItineraAsyncReporter<#mode>>::__itinera_listed(#declared)
    }
}
