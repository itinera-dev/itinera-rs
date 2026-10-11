//! A step's `run`: the handles it takes, and the attempt and factory that call it.

mod parameter;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{ImplItemFn, Receiver, ReceiverKind, Type};

use crate::block;
use crate::described;
use parameter::{Parameter, parameter, repeated};

/// What `run` takes besides the step, in the order it takes them, and whether it is
/// asynchronous.
pub(super) struct Run {
    parameters: Vec<Parameter>,
    asynchronous: bool,
}

impl Run {
    /// Reads `run`'s handles, and removes their attributes.
    pub(super) fn read(run: &mut ImplItemFn) -> syn::Result<Self> {
        if let Some(asynchronous) = run.sig.asyncness {
            if !cfg!(feature = "async") {
                return Err(syn::Error::new_spanned(
                    asynchronous,
                    "an asynchronous step needs the `async` feature of `itinera`",
                ));
            }
        }
        if !run.sig.receiver().is_some_and(by_value) {
            return Err(syn::Error::new_spanned(
                &run.sig,
                "a step's `run` takes the step by value, as `self`",
            ));
        }
        let parameters = run
            .sig
            .inputs
            .iter_mut()
            .filter_map(block::typed)
            .map(parameter)
            .collect::<syn::Result<Vec<_>>>()?;
        if let Some(repeated) = repeated(&parameters) {
            return Err(syn::Error::new(
                repeated.mark,
                "a step takes each handle only once",
            ));
        }
        Ok(Self {
            parameters,
            asynchronous: run.sig.asyncness.is_some(),
        })
    }

    /// The attempt that runs the step with its handles, the factory that builds it, and the
    /// step's `DescribedStep`.
    pub(super) fn expand(&self, step: &Type) -> TokenStream {
        let names: Vec<_> = self.parameters.iter().map(Parameter::name).collect();
        let fields = self
            .parameters
            .iter()
            .map(|parameter| parameter.field(self.asynchronous));
        let arguments: Vec<_> = self.parameters.iter().map(Parameter::argument).collect();
        let mode = described::mode(self.asynchronous);
        let (step_trait, run, factory_trait, describe) = if self.asynchronous {
            (
                quote!(::itinera::step::AsyncStep),
                quote! {
                    fn run(self) -> impl ::core::future::Future<
                        Output = ::core::result::Result<
                            ::itinera::step::Outcome,
                            ::itinera::error::Error,
                        >,
                    > + ::core::marker::Send {
                        <#step>::run(self.step, #(#arguments),*)
                    }
                },
                quote!(::itinera::step::AsyncStepFactory),
                quote!(new_async),
            )
        } else {
            (
                quote!(::itinera::step::Step),
                quote! {
                    fn run(self) -> ::core::result::Result<
                        ::itinera::step::Outcome,
                        ::itinera::error::Error,
                    > {
                        <#step>::run(self.step, #(#arguments),*)
                    }
                },
                quote!(::itinera::step::StepFactory),
                quote!(new),
            )
        };
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

                    fn needs(&self) -> ::itinera::step::StepNeeds {
                        <#step as ::itinera::step::FromInputs>::needs() #(.#names())*
                    }

                    fn build<'a>(
                        &'a self,
                        got: &mut ::itinera::step::Resolved<'a, #mode>,
                    ) -> ::core::result::Result<__ItineraAttempt<'a>, ::itinera::error::Error> {
                        ::core::result::Result::Ok(__ItineraAttempt {
                            step: <#step as ::itinera::step::FromInputs>::from_inputs(got)?,
                            #(#names: got.#names()?,)*
                            attempt: ::core::marker::PhantomData,
                        })
                    }
                }

                impl ::itinera::step::DescribedStep for #step {
                    type Mode = #mode;

                    fn descriptor<W>(
                        name: ::itinera::step::StepName,
                    ) -> ::itinera::step::StepDescriptor<W, #mode> {
                        ::itinera::step::StepDescriptor::#describe(name, __ItineraFactory)
                    }
                }
            };
        }
    }
}

fn by_value(receiver: &Receiver) -> bool {
    matches!(receiver.kind, ReceiverKind::Value)
}
