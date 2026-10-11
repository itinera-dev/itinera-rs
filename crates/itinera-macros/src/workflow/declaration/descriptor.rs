//! The workflow's implementation of `Workflow`, whose descriptor is built the first time it is
//! asked for, from the arguments, the steps, and the parts the workflow's `impl` block adds.

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned as _;
use syn::{Ident, LitStr, Type};

use crate::local::local;
use crate::workflow::arguments::Arguments;
use crate::workflow::with_step::WithStep;

pub(super) fn descriptor(
    workflow: &Ident,
    name: &LitStr,
    arguments: &Arguments,
    steps: &[WithStep],
    mode: &TokenStream,
) -> TokenStream {
    let start = if arguments.is_asynchronous() {
        quote!(::itinera::workflow::WorkflowDescriptor::<#workflow, #mode>::async_builder)
    } else {
        quote!(::itinera::workflow::WorkflowDescriptor::<#workflow>::builder)
    };
    let steps = steps.iter().map(|step| step.declared(workflow));
    let policies = arguments
        .policies
        .iter()
        .map(|policy| attached(policy, workflow));
    let reporters = arguments.reporters.iter().map(listed);
    let async_reporters = arguments.async_reporters.iter().map(listed_async);
    let declared = local("declared");
    quote! {
        impl ::itinera::workflow::Workflow for #workflow {
            type Mode = #mode;

            fn descriptor() -> &'static ::itinera::workflow::WorkflowDescriptor<Self, #mode> {
                static DESCRIPTOR: ::std::sync::LazyLock<
                    ::itinera::workflow::WorkflowDescriptor<#workflow, #mode>,
                > = ::std::sync::LazyLock::new(__itinera_declared);
                &DESCRIPTOR
            }
        }

        fn __itinera_declared() -> ::itinera::workflow::WorkflowDescriptor<#workflow, #mode> {
            let #declared = #start(#name)
                #(#steps)*
                #(#policies)*
                #(#reporters)*
                #(#async_reporters)*;
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

fn listed_async(reporter: &Type) -> TokenStream {
    quote_spanned!(reporter.span()=> .async_reporter::<#reporter>())
}
