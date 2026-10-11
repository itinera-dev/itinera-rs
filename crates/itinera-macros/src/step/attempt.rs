//! A step's `run`: the handles and services it takes, and the attempt and factory that call it.

mod parameter;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{ImplItemFn, Receiver, ReceiverKind, Type};

use super::StepKind;
use super::service::{self, Service};
use crate::local::local;
use crate::{block, described};
use parameter::{Parameter, parameter, repeated};

/// The attributes that mark the parameters of `run`.
pub(super) const MARKS: [&str; 3] = ["contributor", "reporter", "service"];

/// What `run` takes besides the step, the handles and services, in the order it takes them,
/// whether it is asynchronous, and the kind of step it runs.
pub(super) struct Run {
    parameters: Vec<Parameter>,
    asynchronous: bool,
    kind: StepKind,
}

impl Run {
    /// Reads the handles and services of the `run` of a step of this kind, and removes their
    /// attributes.
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
        let services: Vec<&Service> = parameters.iter().filter_map(Parameter::service).collect();
        if let Some(error) = service::repeated(&services) {
            return Err(error);
        }
        Ok(Self {
            parameters,
            asynchronous: run.sig.asyncness.is_some(),
            kind,
        })
    }

    fn indexed(&self) -> impl Iterator<Item = (usize, &Parameter)> {
        self.parameters.iter().enumerate()
    }

    /// The services `run` takes, in order.
    fn services(&self) -> impl Iterator<Item = &Service> {
        self.parameters.iter().filter_map(Parameter::service)
    }

    /// The attempt that runs the step with its handles and services, the factory that builds
    /// it, and the implementation that describes it, for a step whose `new`, if any, takes these
    /// services. A service `new` takes is declared there, and `run` requests it as the same type.
    pub(super) fn expand(&self, step: &Type, built_with: &[&Service]) -> TokenStream {
        let got = local("got");
        let names: Vec<_> = self
            .indexed()
            .map(|(index, parameter)| parameter.name(index))
            .collect();
        let fields = self
            .indexed()
            .map(|(index, parameter)| parameter.field(index, self.asynchronous));
        let taken = self
            .parameters
            .iter()
            .map(|parameter| parameter.taken(&got));
        let arguments: Vec<_> = self
            .indexed()
            .map(|(index, parameter)| parameter.argument(index))
            .collect();
        let handles = self.parameters.iter().filter_map(Parameter::need);
        let (shared, own): (Vec<&Service>, Vec<&Service>) = self
            .services()
            .partition(|service| is_among(service, built_with));
        let declared = own.iter().map(|service| service.declared());
        let same_types = shared
            .iter()
            .filter_map(|service| same_type(service, built_with));
        let requested: Vec<&Service> = built_with.iter().chain(&own).copied().collect();
        let services = listed(requested.iter().map(|service| service.key()));
        let fitted = service::fitted(step, &requested);
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
        let described =
            self.kind
                .described(step, self.asynchronous, &quote!(__ItineraFactory), services);
        quote! {
            const _: () = {
                #(#same_types)*
                #fitted

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
                        <#step as #from>::needs() #(#handles)* #(#declared)*
                    }

                    fn build<'a>(
                        &'a self,
                        #got: &mut #resolved<'a, #mode>,
                    ) -> ::core::result::Result<__ItineraAttempt<'a>, ::itinera::error::Error> {
                        ::core::result::Result::Ok(__ItineraAttempt {
                            step: <#step as #from>::#build(#got)?,
                            #(#names: #taken,)*
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

/// The keys of the step's services, as the implementation that describes it lists them.
fn listed<'a>(keys: impl Iterator<Item = &'a syn::LitStr>) -> TokenStream {
    quote! {
        const SERVICES: &'static [&'static str] = &[#(#keys),*];
    }
}

/// Whether one of these services shares the service's key.
fn is_among(service: &Service, services: &[&Service]) -> bool {
    services.iter().any(|other| other.shares_key_with(service))
}

/// The check that the service is requested as the same type as the one of these that shares its
/// key.
fn same_type(service: &Service, services: &[&Service]) -> Option<TokenStream> {
    services
        .iter()
        .find(|other| other.shares_key_with(service))
        .map(|other| service.same_type_as(other))
}
