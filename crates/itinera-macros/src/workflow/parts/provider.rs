//! A service provider of the workflow's `impl` block: a method marked `#[provider]`, the service
//! it supplies, its scope, the steps it is attached to, and what it requests.

mod request;
mod settings;

use proc_macro2::TokenStream;
use quote::{ToTokens as _, format_ident, quote, quote_spanned};
use syn::spanned::Spanned as _;
use syn::{Attribute, Ident, ImplItemFn, LitStr, ReturnType, Type};

use super::{arguments, by_reference};
use crate::datum::held_by;
use crate::local::local;
use crate::{block, described};
pub(super) use request::MARKS;
use request::Parameter;
use settings::Settings;

/// A method marked as a service provider.
pub(super) struct Provider {
    name: LitStr,
    key: LitStr,
    per_attempt: bool,
    /// The steps it is attached to, none when it serves every step without a provider of its
    /// own for the service.
    steps: Vec<LitStr>,
    method: Ident,
    asynchronous: bool,
    parameters: Vec<Parameter>,
    /// The method's return type, as written.
    output: Type,
}

impl Provider {
    /// Reads the provider its mark describes, and removes its parameters' attributes.
    pub(super) fn read(function: &mut ImplItemFn, mark: &Attribute) -> syn::Result<Self> {
        let settings = Settings::read(arguments(mark, settings::MISREAD)?)?;
        if let Some(steps) = &settings.steps {
            if steps.is_empty() {
                return Err(syn::Error::new_spanned(
                    mark,
                    "a service provider attached to steps lists at least one, as in \
                     `steps = [\"charge\"]`: leave `steps` out for one attached to no step",
                ));
            }
        }
        if !function.sig.receiver().is_some_and(by_reference) {
            return Err(syn::Error::new_spanned(
                &function.sig,
                "a service provider takes the workflow as `&self`",
            ));
        }
        if let Some(asynchronous) = function.sig.asyncness {
            if !cfg!(feature = "async") {
                return Err(syn::Error::new_spanned(
                    asynchronous,
                    "an asynchronous service provider needs the `async` feature of `itinera`",
                ));
            }
        }
        let ReturnType::Type(_, output) = &function.sig.output else {
            return Err(syn::Error::new_spanned(
                &function.sig.ident,
                "a service provider returns the instance it makes, as `Result<Provided<T>, Error>`",
            ));
        };
        let output = (**output).clone();
        let per_attempt = settings.per_attempt.is_some();
        let parameters = function
            .sig
            .inputs
            .iter_mut()
            .filter_map(block::typed)
            .map(|parameter| Parameter::read(parameter, per_attempt))
            .collect::<syn::Result<_>>()?;
        let method = function.sig.ident.clone();
        Ok(Self {
            name: settings
                .name
                .unwrap_or_else(|| described::named_after(&method)),
            key: settings
                .key
                .unwrap_or_else(|| described::named_after(&method)),
            per_attempt,
            steps: settings.steps.unwrap_or_default(),
            method,
            asynchronous: function.sig.asyncness.is_some(),
            parameters,
            output,
        })
    }

    pub(super) fn name(&self) -> &LitStr {
        &self.name
    }

    pub(super) fn key(&self) -> &LitStr {
        &self.key
    }

    pub(super) fn steps(&self) -> &[LitStr] {
        &self.steps
    }

    pub(super) fn is_asynchronous(&self) -> bool {
        self.asynchronous
    }

    pub(super) fn requests_a_reporter(&self) -> bool {
        self.parameters.iter().any(Parameter::is_reporter)
    }

    pub(super) fn shares_name_with(&self, other: &Self) -> bool {
        self.name.value() == other.name.value()
    }

    pub(super) fn shares_key_with(&self, other: &Self) -> bool {
        self.key.value() == other.key.value()
    }

    /// The type of the instances the provider makes, read from its return type when it is written
    /// `Result<Provided<T>>`, with the error of `itinera::error::Result`, or `Result<Provided<T>, ...>`.
    pub(super) fn made(&self) -> Option<&Type> {
        let returned =
            held_by(&self.output, "Result", 1).or_else(|| held_by(&self.output, "Result", 2))?;
        held_by(returned, "Provided", 1)
    }

    /// Whether both providers' return types are written as `Result<Provided<T>, ...>` or
    /// `Result<Provided<T>>` with the same `T`, written alike.
    pub(super) fn makes_as(&self, other: &Self) -> bool {
        match (self.made(), other.made()) {
            (Some(made), Some(other)) => spelled(made) == spelled(other),
            _ => false,
        }
    }

    /// The service the provider supplies and the steps it is attached to, as the checks of the
    /// workflow's struct read them.
    pub(super) fn listed(&self) -> TokenStream {
        let Self { key, steps, .. } = self;
        quote!((#key, &[#(#steps),*]))
    }

    /// The function the descriptor of a workflow in this mode calls, which resolves the requests
    /// and calls the method: an associated function of the workflow, so that the method's return
    /// type means the same there.
    pub(super) fn function(&self, workflow: &Type, asynchronous: bool) -> TokenStream {
        let function = self.function_name(asynchronous);
        let got = local("got");
        let workflow_value = local("workflow");
        let pattern = if self.parameters.is_empty() {
            quote!(_)
        } else if self.parameters.iter().any(Parameter::takes) {
            quote!(mut #got)
        } else {
            quote!(#got)
        };
        let arguments: Vec<Ident> = self
            .parameters
            .iter()
            .enumerate()
            .map(|(index, parameter)| parameter.argument(index))
            .collect();
        let taken = self
            .parameters
            .iter()
            .map(|parameter| parameter.taken(&got));
        let output = &self.output;
        let span = output.span();
        let mut method = self.method.clone();
        method.set_span(span);
        let awaited = self.asynchronous.then_some(quote!(.await));
        let called =
            quote_spanned!(span=> <#workflow>::#method(#workflow_value, #(#arguments),*) #awaited);
        let scope = self.scope();
        let (asyncness, mode) = if asynchronous {
            (quote!(async), quote!(, ::itinera::mode::Asynchronous))
        } else {
            (quote!(), quote!())
        };
        quote! {
            #[doc(hidden)]
            #asyncness fn #function(
                #workflow_value: &#workflow,
                #pattern: ::itinera::policy::Requested<
                    '_,
                    #workflow,
                    ::itinera::policy::Provider<#scope>
                    #mode
                >,
            ) -> #output {
                #(let #arguments = #taken;)*
                #called
            }
        }
    }

    /// The provider's descriptor, for a workflow in this mode, attached to its steps, with what
    /// it needs.
    pub(super) fn descriptor(&self, workflow: &Type, asynchronous: bool) -> TokenStream {
        let Self { name, key, .. } = self;
        let function = self.function_name(asynchronous);
        let constructor = match (self.per_attempt, asynchronous) {
            (false, false) => quote!(new),
            (true, false) => quote!(per_attempt),
            (false, true) => quote!(new_async),
            (true, true) => quote!(per_attempt_async),
        };
        let needs = self.parameters.iter().filter_map(Parameter::need);
        let steps = &self.steps;
        let described = quote_spanned! {self.output.span()=>
            ::itinera::service::ServiceProviderDescriptor::#constructor(
                #name,
                &::itinera::service::Service::new(#key),
                <#workflow>::#function,
            )
        };
        quote! {
            #described
                .needing(::itinera::policy::HookNeeds::new() #(#needs)*)
                #(.step(::itinera::step::step_name!(#steps)))*
        }
    }

    fn scope(&self) -> TokenStream {
        if self.per_attempt {
            quote!(::itinera::service::PerAttempt)
        } else {
            quote!(::itinera::service::PerJourney)
        }
    }

    fn function_name(&self, asynchronous: bool) -> Ident {
        if asynchronous {
            format_ident!("__itinera_async_provider_{}", self.method)
        } else {
            format_ident!("__itinera_provider_{}", self.method)
        }
    }
}

/// The type as written, in tokens.
fn spelled(ty: &Type) -> String {
    ty.to_token_stream().to_string()
}

#[cfg(test)]
mod tests;
