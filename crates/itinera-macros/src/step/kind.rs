//! The kind of step a macro describes, a step or a rescue step, and the names each kind uses.

use proc_macro2::TokenStream;
use quote::quote;
use syn::Type;

use crate::described;

/// The kind of step a macro describes.
#[derive(Clone, Copy)]
pub(super) enum StepKind {
    Step,
    Rescue,
}

impl StepKind {
    /// What messages call it.
    pub(super) fn noun(self) -> &'static str {
        match self {
            Self::Step => "step",
            Self::Rescue => "rescue step",
        }
    }

    /// The outcome its `run` reports.
    pub(super) fn outcome(self) -> TokenStream {
        match self {
            Self::Step => quote!(::itinera::step::Outcome),
            Self::Rescue => quote!(::itinera::step::RescueOutcome),
        }
    }

    /// The trait its attempts implement, and the trait its factory implements, in a mode.
    pub(super) fn traits(self, asynchronous: bool) -> (TokenStream, TokenStream) {
        match (self, asynchronous) {
            (Self::Step, false) => (
                quote!(::itinera::step::Step),
                quote!(::itinera::step::StepFactory),
            ),
            (Self::Step, true) => (
                quote!(::itinera::step::AsyncStep),
                quote!(::itinera::step::AsyncStepFactory),
            ),
            (Self::Rescue, false) => (
                quote!(::itinera::step::RescueStep),
                quote!(::itinera::step::RescueFactory),
            ),
            (Self::Rescue, true) => (
                quote!(::itinera::step::AsyncRescueStep),
                quote!(::itinera::step::AsyncRescueFactory),
            ),
        }
    }

    /// What it needs, what it is built from, and the trait and function that build it from its
    /// requests.
    pub(super) fn building(self) -> (TokenStream, TokenStream, TokenStream, TokenStream) {
        match self {
            Self::Step => (
                quote!(::itinera::step::StepNeeds),
                quote!(::itinera::step::Resolved),
                quote!(::itinera::step::FromInputs),
                quote!(from_inputs),
            ),
            Self::Rescue => (
                quote!(::itinera::step::RescueNeeds),
                quote!(::itinera::step::RescueResolved),
                quote!(::itinera::step::FromRescueRequests),
                quote!(from_requests),
            ),
        }
    }

    /// The implementation through which a workflow lists the step, or gives a step the rescue
    /// step, with `factory` building it, and with `services` listing what it requests.
    pub(super) fn described(
        self,
        step: &Type,
        asynchronous: bool,
        factory: &TokenStream,
        services: Option<TokenStream>,
    ) -> TokenStream {
        let mode = described::mode(asynchronous);
        match self {
            Self::Step => {
                let new = if asynchronous {
                    quote!(new_async)
                } else {
                    quote!(new)
                };
                quote! {
                    impl ::itinera::step::DescribedStep for #step {
                        type Mode = #mode;

                        #services

                        fn descriptor<W>(
                            name: ::itinera::step::StepName,
                        ) -> ::itinera::step::StepDescriptor<W, #mode> {
                            ::itinera::step::StepDescriptor::#new(name, #factory)
                        }
                    }
                }
            }
            Self::Rescue => {
                let rescue = if asynchronous {
                    quote!(rescue_async)
                } else {
                    quote!(rescue)
                };
                quote! {
                    impl ::itinera::step::DescribedRescueStep for #step {
                        type Mode = #mode;

                        #services

                        fn rescue<W>(
                            step: ::itinera::step::StepDescriptor<W, #mode>,
                        ) -> ::itinera::step::StepDescriptor<W, #mode> {
                            step.#rescue(#factory)
                        }
                    }
                }
            }
        }
    }
}
