//! The workflow's mode, which it takes from its steps: the mode of its first step, which every
//! other step shares.

use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use super::with_step::WithStep;
use crate::described;

/// The mode of a workflow listing these steps, as its implementation of `Workflow` names it: its
/// first step's, or `Synchronous` when no step could be read. The step's type is named only in a
/// constant, so that a public workflow may list private steps.
pub(super) fn of(steps: &[WithStep]) -> TokenStream {
    match steps.first() {
        Some(first) => {
            let step = first.step();
            quote! {
                <__ItineraModeOf<{
                    <<#step as ::itinera::step::DescribedStep>::Mode as __ItineraModeFlag>::ASYNCHRONOUS
                }> as __ItineraFlaggedMode>::Mode
            }
        }
        None => quote!(::itinera::mode::Synchronous),
    }
}

/// The workflow's mode, as the rest of the generated code names it.
pub(super) fn named(workflow: &Ident) -> TokenStream {
    quote!(<#workflow as ::itinera::workflow::Workflow>::Mode)
}

/// What [`of`] names: whether each mode is asynchronous, and the mode that a flag stands for.
pub(super) fn flagged() -> TokenStream {
    let flags = flags();
    let modes = modes();
    quote! {
        pub trait __ItineraModeFlag {
            const ASYNCHRONOUS: bool;
        }

        pub struct __ItineraModeOf<const ASYNCHRONOUS: bool>;

        pub trait __ItineraFlaggedMode {
            type Mode: ::itinera::mode::Mode;
        }

        #(
            impl __ItineraModeFlag for #modes {
                const ASYNCHRONOUS: bool = #flags;
            }

            impl __ItineraFlaggedMode for __ItineraModeOf<#flags> {
                type Mode = #modes;
            }
        )*
    }
}

/// The traits through which the descriptor starts declaring the workflow in its mode, and takes
/// each step in that mode, which only a step of the same mode can be taken in.
pub(super) fn declared(workflow: &Ident) -> TokenStream {
    let modes = modes();
    let asynchronous = asynchronous_start(workflow);
    let builder = quote!(::itinera::workflow::WorkflowBuilder);
    let step = quote!(::itinera::step::StepDescriptor);
    quote! {
        pub trait __ItineraStart: ::itinera::mode::Mode + ::core::marker::Sized {
            fn __itinera_builder(name: &'static str) -> #builder<#workflow, Self>;
        }

        impl __ItineraStart for ::itinera::mode::Synchronous {
            fn __itinera_builder(name: &'static str) -> #builder<#workflow, Self> {
                ::itinera::workflow::WorkflowDescriptor::<#workflow>::builder(name)
            }
        }

        #[diagnostic::on_unimplemented(
            message = "the steps of a workflow are all synchronous or all asynchronous",
            label = "a step of another mode than the workflow's first step",
            note = "a workflow takes its mode from its steps, and an asynchronous step, whose \
                    `run` is an `async fn`, belongs only to an asynchronous workflow"
        )]
        pub trait __ItineraSameMode<__ItineraMode: ::itinera::mode::Mode>:
            ::itinera::mode::Mode + ::core::marker::Sized
        {
            fn __itinera_step(step: #step<#workflow, Self>) -> #step<#workflow, __ItineraMode>;
        }

        #(
            impl __ItineraSameMode<#modes> for #modes {
                fn __itinera_step(step: #step<#workflow, Self>) -> #step<#workflow, Self> {
                    step
                }
            }
        )*

        #asynchronous
    }
}

/// Starting an asynchronous workflow, with the `async` feature.
fn asynchronous_start(workflow: &Ident) -> Option<TokenStream> {
    cfg!(feature = "async").then(|| {
        quote! {
            impl __ItineraStart for ::itinera::mode::Asynchronous {
                fn __itinera_builder(
                    name: &'static str,
                ) -> ::itinera::workflow::WorkflowBuilder<#workflow, Self> {
                    ::itinera::workflow::WorkflowDescriptor::<#workflow, Self>::async_builder(name)
                }
            }
        }
    })
}

/// Whether each mode a workflow may be declared in is asynchronous: `Synchronous`, then
/// `Asynchronous` with the `async` feature.
fn flags() -> Vec<bool> {
    let asynchronous = cfg!(feature = "async").then_some(true);
    std::iter::once(false).chain(asynchronous).collect()
}

/// The modes a workflow may be declared in.
fn modes() -> Vec<TokenStream> {
    flags().into_iter().map(described::mode).collect()
}

/// The trait through which an asynchronous reporter is listed on the workflow, or added to one of
/// its instances, which only a workflow in the asynchronous mode takes, with the `async`
/// feature.
pub(super) fn asynchronous_reporters(workflow: &Ident) -> Option<TokenStream> {
    cfg!(feature = "async").then(|| {
        let asynchronous = quote!(::itinera::mode::Asynchronous);
        let declared = quote!(::itinera::workflow::WorkflowBuilder<#workflow, __ItineraMode>);
        let instance = quote!(::itinera::instance::InstanceBuilder<#workflow, __ItineraMode>);
        quote! {
            #[diagnostic::on_unimplemented(
                message = "asynchronous reporters belong to an asynchronous workflow",
                label = "an asynchronous reporter of a workflow whose steps are synchronous",
                note = "a workflow takes its mode from its steps: it is asynchronous when its \
                        steps are, their `run` an `async fn`"
            )]
            pub trait __ItineraAsyncReporter<__ItineraMode: ::itinera::mode::Mode> {
                fn __itinera_listed(declared: #declared) -> #declared
                where
                    Self: ::core::default::Default;

                fn __itinera_added(self, instance: #instance) -> #instance;
            }

            impl<__ItineraReporter: ::itinera::report::AsyncReporter>
                __ItineraAsyncReporter<#asynchronous> for __ItineraReporter
            {
                fn __itinera_listed(
                    declared: ::itinera::workflow::WorkflowBuilder<#workflow, #asynchronous>,
                ) -> ::itinera::workflow::WorkflowBuilder<#workflow, #asynchronous>
                where
                    Self: ::core::default::Default,
                {
                    declared.async_reporter::<Self>()
                }

                fn __itinera_added(
                    self,
                    instance: ::itinera::instance::InstanceBuilder<#workflow, #asynchronous>,
                ) -> ::itinera::instance::InstanceBuilder<#workflow, #asynchronous> {
                    instance.async_reporter(self)
                }
            }
        }
    })
}
