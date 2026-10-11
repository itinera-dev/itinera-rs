//! A step's `run`: the handles it takes, and the attempt and factory that call it.

mod parameter;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{ImplItemFn, Receiver, ReceiverKind, Type};

use super::StepKind;
use crate::{block, described};
use parameter::{Parameter, parameter, repeated};

/// What `run` takes besides the step, in the order it takes them, whether it is asynchronous, and
/// the kind of step it runs.
pub(super) struct Run {
    parameters: Vec<Parameter>,
    asynchronous: bool,
    kind: StepKind,
}

impl Run {
    /// Reads the handles of the `run` of a step of this kind, and removes their attributes.
    pub(super) fn read(run: &mut ImplItemFn, kind: StepKind) -> syn::Result<Self> {
        let noun = kind.noun();
        if let Some(asynchronous) = run.sig.asyncness {
            if !cfg!(feature = "async") {
                return Err(syn::Error::new_spanned(
                    asynchronous,
                    format!("an asynchronous {noun} needs the `async` feature of `itinera`"),
                ));
            }
        }
        if !run.sig.receiver().is_some_and(by_value) {
            return Err(syn::Error::new_spanned(
                &run.sig,
                format!("a {noun}'s `run` takes the {noun} by value, as `self`"),
            ));
        }
        let parameters = run
            .sig
            .inputs
            .iter_mut()
            .filter_map(block::typed)
            .map(|typed| parameter(typed, noun))
            .collect::<syn::Result<Vec<_>>>()?;
        if let Some(repeated) = repeated(&parameters) {
            return Err(syn::Error::new(
                repeated.mark,
                format!("a {noun} takes each handle only once"),
            ));
        }
        Ok(Self {
            parameters,
            asynchronous: run.sig.asyncness.is_some(),
            kind,
        })
    }

    /// The attempt that runs the step with its handles, the factory that builds it, and the
    /// implementation that describes it.
    pub(super) fn expand(&self, step: &Type) -> TokenStream {
        let names: Vec<_> = self.parameters.iter().map(Parameter::name).collect();
        let fields = self
            .parameters
            .iter()
            .map(|parameter| parameter.field(self.asynchronous));
        let arguments: Vec<_> = self.parameters.iter().map(Parameter::argument).collect();
        let mode = described::mode(self.asynchronous);
        let outcome = self.kind.outcome();
        let (step_trait, factory_trait) = self.kind.traits(self.asynchronous);
        let (needs, resolved, from, build) = self.kind.building();
        let run = if self.asynchronous {
            quote! {
                fn run(self) -> impl ::core::future::Future<
                    Output = ::core::result::Result<#outcome, ::itinera::error::Error>,
                > + ::core::marker::Send {
                    <#step>::run(self.step, #(#arguments),*)
                }
            }
        } else {
            quote! {
                fn run(self) -> ::core::result::Result<#outcome, ::itinera::error::Error> {
                    <#step>::run(self.step, #(#arguments),*)
                }
            }
        };
        let described = self
            .kind
            .described(step, self.asynchronous, &quote!(__ItineraFactory));
        quote! {
            const _: () = {
                struct __ItineraAttempt<'a> {
                    step: #step,
                    #(#fields,)*
                    attempt: ::core::marker::PhantomData<&'a ()>,
                }

                impl<'a> #step_trait for __ItineraAttempt<'a> {
                    #run
                }

                struct __ItineraFactory;

                impl #factory_trait for __ItineraFactory {
                    type Step<'a> = __ItineraAttempt<'a>;

                    fn needs(&self) -> #needs {
                        <#step as #from>::needs() #(.#names())*
                    }

                    fn build<'a>(
                        &'a self,
                        got: &mut #resolved<'a, #mode>,
                    ) -> ::core::result::Result<__ItineraAttempt<'a>, ::itinera::error::Error> {
                        ::core::result::Result::Ok(__ItineraAttempt {
                            step: <#step as #from>::#build(got)?,
                            #(#names: got.#names()?,)*
                            attempt: ::core::marker::PhantomData,
                        })
                    }
                }

                #described
            };
        }
    }
}

fn by_value(receiver: &Receiver) -> bool {
    matches!(receiver.kind, ReceiverKind::Value)
}
